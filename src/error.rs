use failure::Fail;
use rustls;

/// Error for SilkRoad
#[derive(Fail, Debug)]
pub enum SkrdError {
    /// IO error
    #[fail(display = "IO error: {}", _0)]
    Io(#[cause] std::io::Error),

    /// TLS error
    #[fail(display = "TLS error: {}", _0)]
    Tls(#[cause] rustls::TLSError),

    /// Custom error
    #[fail(display = "Custom error: {}", _0)]
    Custom(String),
}

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

pub type SkrdResult<T> = std::result::Result<T, SkrdError>;
