use crate::error::SkrdResult;
use crate::registry::{Registry, UrlConfig};
use actix_http::http::header::HttpDate;
use actix_web::Responder;
use git2::build::CheckoutBuilder;
use git2::Oid;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::{Duration, SystemTime};

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

    let base_url = registry.base_url();
    let url_config = UrlConfig {
        dl: format!("{}{}", base_url, "/api/v1/crates"),
        api: format!("{}", base_url),
    };

    let repo = git2::Repository::open(registry.index_path())?;
    repo.checkout_head(Some(CheckoutBuilder::new().path("config.json").force()))?;
    let mut index = repo.index()?;
    index.write()?;

    let mut file = OpenOptions::new().write(true).read(true).open(&path)?;

    let mut content = String::with_capacity(file.metadata()?.len() as usize);
    file.read_to_string(&mut content)?;

    let url_config_file = serde_json::from_str::<UrlConfig>(&content)?;

    if url_config_file == url_config {
        return Ok(None);
    }

    let content = serde_json::to_string_pretty(&url_config)?;

    let bytes = content.as_bytes();

    file.seek(SeekFrom::Start(0))?;
    file.write(bytes)?;
    file.set_len(bytes.len() as u64)?;
    drop(file);

    let sig = repo.signature()?;
    let find = repo
        .head()
        .and_then(|reference| {
            reference
                .target()
                .ok_or(git2::Error::from_str("no reference found"))
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
