use actix_http::http::header::HttpDate;
use actix_web::Responder;
use std::time::{Duration, SystemTime};

pub fn is_default_port(port: u16, ssl: bool) -> bool {
    if ssl {
        port == 443u16
    } else {
        port == 80u16
    }
}

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
