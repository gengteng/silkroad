use crate::error::{
    SkrdError,
    SkrdResult
};
use crate::registry::{Crate, Registry, UrlConfig};
use crate::stream::WalkStream;
use actix_http::http::header::HttpDate;
use actix_web::Responder;
use git2::build::CheckoutBuilder;
use git2::Oid;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use futures::{
    future::Future,
    stream::Stream
};
use actix_web::client::Client;

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
//    let wd = walkdir::WalkDir::new(registry.index_path());
//    let client = Client::default();
//    let mut stream: WalkStream = wd.into();
//    let filter = stream.filter(|e| {
//        if let Ok(metadata) = e.metadata() {
//            if metadata.is_dir() || !metadata.is_file() {
//                return false;
//            }
//
//            if e.path().starts_with(registry.index_git_path())
//                || e.file_name() == "config.json"
//            {
//                return false;
//            }
//
//            return true;
//        }
//
//        return false;
//    });
//
//    let download = filter.map_err(SkrdError::Walk).for_each(|e| {
//
//        tokio_fs::File::open("").map_err(SkrdError::Io).and_then(|file| {
//            let codec = tokio_codec::LinesCodec::new();
//            let read = tokio_codec::FramedRead::new(file, codec);
//
//            Ok(())
//        })
//
//    });

    let mut checked = 0;
    let mut downloaded = 0;
//    for w in wd {
//        match w {
//            Ok(entry) => {
//                let metadata = entry.metadata()?;
//                if metadata.is_dir() || !metadata.is_file() {
//                    continue;
//                }
//
//                if entry.path().starts_with(registry.index_git_path())
//                    || entry.file_name() == "config.json"
//                {
//                    continue;
//                }
//
//                let file = File::open(entry.path())?;
//                let reader = BufReader::new(file);
//
//                for line in reader.lines() {
//                    let json = line?;
//
//                    let krate = serde_json::from_str::<Crate>(&json)?;
//
//                    let crate_path = registry.crates_path().join(get_crate_path(&krate.name, &krate.version));
//
//                    checked += 1;
//                    if checked % 1000 == 0 {
//                        info!("{} crates has been checked.", checked);
//                    }
//
//                    if !crate_path.exists() {
//
//                        downloaded += 1;
//                    }
//                }
//            }
//            Err(e) => error!("walk error: {}", e),
//        }
//    }
    info!(
        "Total: {} crates is checked, {} crates is downloaded.",
        checked, downloaded
    );
    Ok(())
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
