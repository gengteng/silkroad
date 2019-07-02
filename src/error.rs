use actix_http::ResponseError;
use failure::Fail;
use std::convert::From;

/// Error for SilkRoad
#[derive(Fail, Debug)]
pub enum SkrdError {
    /// IO error
    #[fail(display = "IO error: {}", _0)]
    Io(#[cause] std::io::Error),

    /// TLS error
    #[fail(display = "TLS error: {}", _0)]
    Tls(#[cause] rustls::TLSError),

    /// Log error
    #[fail(display = "Log error: {}", _0)]
    Log(#[cause] log::SetLoggerError),

    /// Toml error
    #[fail(display = "Toml error: {}", _0)]
    Toml(toml::de::Error),

    /// Json error
    #[fail(display = "Toml error: {}", _0)]
    Json(serde_json::Error),

    /// Mime error
    #[fail(display = "Mime error: {}", _0)]
    Mime(mime::FromStrError),

    /// Payload error
    #[fail(display = "Payload error: {}", _0)]
    Payload(actix_http::error::PayloadError),

    /// Git error
    #[fail(display = "Git error: {}", _0)]
    Git(git2::Error),

    /// FromUtf8 error
    #[fail(display = "FromUtf8 error: {}", _0)]
    FromUtf8(std::string::FromUtf8Error),

    /// Poison error
    #[fail(display = "Poison error.")]
    Poison(String),

    /// Custom error
    #[fail(display = "Custom error: {}", _0)]
    Custom(String),

    /// Static custom error
    #[fail(display = "Custom error: {}", _0)]
    StaticCustom(&'static str),
}

impl ResponseError for SkrdError {}

impl From<std::io::Error> for SkrdError {
    fn from(err: std::io::Error) -> SkrdError {
        SkrdError::Io(err)
    }
}

impl From<rustls::TLSError> for SkrdError {
    fn from(err: rustls::TLSError) -> SkrdError {
        SkrdError::Tls(err)
    }
}

impl From<log::SetLoggerError> for SkrdError {
    fn from(err: log::SetLoggerError) -> SkrdError {
        SkrdError::Log(err)
    }
}

impl From<toml::de::Error> for SkrdError {
    fn from(err: toml::de::Error) -> SkrdError {
        SkrdError::Toml(err)
    }
}

impl From<git2::Error> for SkrdError {
    fn from(err: git2::Error) -> Self {
        SkrdError::Git(err)
    }
}

impl From<serde_json::Error> for SkrdError {
    fn from(err: serde_json::Error) -> Self {
        SkrdError::Json(err)
    }
}

impl From<mime::FromStrError> for SkrdError {
    fn from(err: mime::FromStrError) -> Self {
        SkrdError::Mime(err)
    }
}

impl From<actix_http::error::PayloadError> for SkrdError {
    fn from(err: actix_http::error::PayloadError) -> Self {
        SkrdError::Payload(err)
    }
}

impl From<std::string::FromUtf8Error> for SkrdError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        SkrdError::FromUtf8(err)
    }
}

impl<T> From<std::sync::PoisonError<T>> for SkrdError {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        SkrdError::Poison(err.to_string())
    }
}

pub type SkrdResult<T> = std::result::Result<T, SkrdError>;
