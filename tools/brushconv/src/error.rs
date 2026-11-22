//! What can go wrong, in words a user can act on.

use std::fmt;

/// A conversion that couldn't be done, or an input that couldn't be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(String);

impl Error {
    pub fn new(message: impl Into<String>) -> Error {
        Error(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error(format!("I/O error: {e}"))
    }
}

impl From<png::EncodingError> for Error {
    fn from(e: png::EncodingError) -> Error {
        Error(format!("could not encode PNG: {e}"))
    }
}

impl From<png::DecodingError> for Error {
    fn from(e: png::DecodingError) -> Error {
        Error(format!("could not decode PNG: {e}"))
    }
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Error {
        Error(format!("could not write JSON: {e}"))
    }
}
