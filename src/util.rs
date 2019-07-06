use crate::error::{SkrdError, SkrdResult};
use crate::registry::{CrateMeta, Mirror, Registry, UrlConfig};
use actix_http::http::header::HttpDate;
use actix_web::Responder;
use digest::Digest;
use git2::build::CheckoutBuilder;
use git2::Oid;
use rayon::prelude::*;
use reqwest::Client;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use walkdir::DirEntry;

/// Get the service name from url query string
///
pub fn get_service_from_query_string(query: &str) -> Option<&str> {
    let head = "service=git-";
    query.find(head).and_then(|i| {
        let start = i + head.len();
        match &query[start..].find('&') {
            Some(u) => Some(&query[start..u + start]),
            None => Some(&query[start..]),
        }
    })
}

/// Set a Responder to no-cache
///
pub fn no_cache(res: impl Responder) -> impl Responder {
    res.with_header("Expires", "Fri, 01 Jan 1980 00:00:00 GMT")
        .with_header("Pragma", "no-cache")
        .with_header("Cache-Control", "no-cache, max-age=0, must-revalidate")
}

/// Set a Responder to cache forever
///
pub fn cache_forever(res: impl Responder) -> impl Responder {
    let now = SystemTime::now();
    let date: HttpDate = now.into();

    let next_year = now + Duration::from_secs(31_536_000u64);
    let expire: HttpDate = next_year.into();

    res.with_header("Date", date)
        .with_header("Expires", expire)
        .with_header("Cache-Control", "public, max-age=31536000")
}

/// Write custom url(dl and api) to config.json
///
pub fn write_config_json(registry: &Registry) -> SkrdResult<Option<Oid>> {
    const CONFIG_JSON: &str = "config.json";
    let path = registry.index_path().join(CONFIG_JSON);

    let url_config = UrlConfig::from(registry);

    let repo = git2::Repository::open(registry.index_path())?;
    repo.checkout_head(Some(CheckoutBuilder::new().path("config.json").force()))?;
    let mut index = repo.index()?;
    index.write()?;

    let mut file = OpenOptions::new().write(true).read(true).open(&path)?;

    let mut content = String::with_capacity(file.metadata()?.len() as usize);
    file.read_to_string(&mut content)?;

    let deserialize_result = serde_json::from_str::<UrlConfig>(&content);

    if let Ok(url_config_file) = deserialize_result {
        if url_config_file == url_config {
            return Ok(None);
        }
    }

    // If the deserialization fails or urls are incorrect, write the correct ones.
    let content = serde_json::to_string_pretty(&url_config)?;

    let bytes = content.as_bytes();

    file.seek(SeekFrom::Start(0))?;
    file.write_all(bytes)?;
    file.set_len(bytes.len() as u64)?;
    drop(file);

    let sig = repo.signature()?;
    let find = repo
        .head()
        .and_then(|reference| {
            reference
                .target()
                .ok_or_else(|| git2::Error::from_str("no reference found"))
        })
        .and_then(|target| repo.find_commit(target));

    let path = Path::new(CONFIG_JSON);
    index.add_path(&path)?;
    index.write()?;

    let tree = index.write_tree().and_then(|id| repo.find_tree(id))?;

    match find {
        Ok(parent) => Ok(Some(repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "base_url",
            &tree,
            &[&parent],
        )?)),
        Err(_) => Ok(Some(repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "base_url",
            &tree,
            &[],
        )?)),
    }
}

pub fn download_crates(registry: &Registry) -> SkrdResult<()> {
    let mirror = registry.mirror_config().ok_or_else(|| {
        SkrdError::Custom(format!(
            "Registry '{}' does not seem to be a mirror.",
            registry.config().name()
        ))
    })?;

    let wd = walkdir::WalkDir::new(registry.index_path())
        .sort_by(|a, b| a.file_name().cmp(b.file_name()));
    let client = reqwest::ClientBuilder::new().gzip(false).build()?;

    let (checked, downloaded, failed) = wd
        .into_iter()
        .filter(|result| match result {
            Ok(entry) => match entry.metadata() {
                Ok(metadata) => {
                    if metadata.is_dir() || !metadata.is_file() {
                        return false;
                    }

                    if entry.path().starts_with(registry.index_git_path())
                        || entry.file_name() == "config.json"
                    {
                        return false;
                    }

                    true
                }
                Err(_) => false,
            },
            Err(_) => false,
        })
        .par_bridge()
        .map(|w| match w {
            Ok(entry) => match download(mirror, registry, &entry, &client) {
                Ok(r) => r,
                Err(e) => {
                    error!("Download error: {}", e);
                    (0, 0, 0)
                }
            },
            Err(e) => {
                error!("Walk error: {}", e);
                (0, 0, 0)
            }
        })
        .reduce(|| (0, 0, 0), |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2));
    info!(
        "Total: {} crates is checked, {} crates is downloaded ({} error).",
        checked, downloaded, failed
    );
    Ok(())
}

fn download(
    mirror: &Mirror,
    registry: &Registry,
    entry: &DirEntry,
    client: &Client,
) -> SkrdResult<(u32, u32, u32)> {
    let mut checked = 0u32;
    let mut dl_ok = 0u32;
    let mut dl_error = 0u32;

    let file = File::open(entry.path())?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let json = line?;

        let crate_meta = serde_json::from_str::<CrateMeta>(&json)?;

        let crate_path = get_crate_path(&crate_meta.name, &crate_meta.version);

        let crate_file_path = registry.crates_path().join(&crate_path);

        checked += 1;
        if !crate_file_path.exists() {
            let crate_dl_url = format!(
                "{}/{}/{}/download",
                mirror.origin_urls.dl, crate_meta.name, crate_meta.version
            );

            let download = client
                .get(&crate_dl_url)
                .send()
                .map_err(SkrdError::Reqwest)
                .and_then(|mut r| {
                    if !r.status().is_success() {
                        return Err(SkrdError::Custom(format!("Http Response status: {}", r.status().as_u16())));
                    }

                    let (bytes, len) = {
                        let mut vec = Vec::with_capacity(200 * 1024);
                        let len = r.copy_to(&mut vec)?;
                        (vec, len)
                    };

                    let mut sha256 = sha2::Sha256::new();
                    sha256.input(&bytes);
                    let checksum = sha256.result().to_vec();

                    if checksum != crate_meta.checksum {
                        return Err(SkrdError::Custom(format!(
                            "Crate {}-{} checksum error: expected={}, actual={}",
                            crate_meta.name,
                            crate_meta.version,
                            hex::encode(&crate_meta.checksum),
                            hex::encode(&checksum)
                        )));
                    }

                    create_dir_all(crate_file_path.parent().ok_or_else(|| {
                        SkrdError::Custom(format!(
                            "{} does not have a parent directory.",
                            crate_file_path.display()
                        ))
                    })?)?;
                    let mut file = OpenOptions::new()
                        .write(true)
                        .create(true)
                        .open(&crate_file_path)?;
                    file.write_all(&bytes)?;

                    Ok::<_, SkrdError>(len)
                });

            match download {
                Ok(len) => {
                    dl_ok += 1;
                    info!(
                        "Crate {} ({} bytes) downloaded to {}.",
                        crate_meta,
                        len,
                        crate_file_path.display()
                    );
                }
                Err(e) => {
                    dl_error += 1;
                    warn!("Crate {} download error: {}", crate_meta, e);
                }
            }
        }
    }

    Ok((checked, dl_ok, dl_error))
}

pub fn get_crate_path(name: &str, version: &str) -> PathBuf {
    match name.len() {
        1 => format!("{}/{}/{}-{}.crate", 1, name, name, version),
        2 => format!("{}/{}/{}-{}.crate", 2, name, name, version),
        3 => format!("{}/{}/{}/{}-{}.crate", 3, &name[..1], name, name, version),
        _ => format!(
            "{}/{}/{}/{}-{}.crate",
            &name[..2],
            &name[2..4],
            name,
            name,
            version
        ),
    }
    .into()
}
