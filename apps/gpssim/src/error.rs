use thiserror::Error;

/// Custom error type for the gpssim application
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("GPS error: {0}")]
    Gps(#[from] gps::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Time parsing error: {0}")]
    TimeParseError(#[from] jiff::Error),

    #[error("Command line argument error: {0}")]
    CliError(String),

    #[error("Application error: {0}")]
    Other(String),
}

impl Error {
    /// Create a new error with a message
    #[inline]
    pub fn msg(message: impl Into<String>) -> Self {
        Error::Other(message.into())
    }

    /// Create a new CLI error
    #[inline]
    pub fn cli_error(message: impl Into<String>) -> Self {
        Error::CliError(message.into())
    }
}
