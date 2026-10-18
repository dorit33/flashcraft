//! The crate's error type. Errors are `Result<T, E>` end to end; nothing panics.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid parameters: {0}")]
    InvalidParams(String),
    #[error("image not usable: {0}")]
    BadImage(String),
    #[error("drive not usable: {0}")]
    BadDrive(String),
    #[error("refusing to write: {0}")]
    Refused(String),
    #[error("verify failed: {0}")]
    Verify(String),
    #[error("i/o error: {0}")]
    Io(String),
    #[error("backend error: {0}")]
    Backend(String),
}

pub type Result<T> = std::result::Result<T, Error>;
