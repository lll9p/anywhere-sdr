use thiserror::Error;

/// Custom error type for the gpssim application
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Error from the GPS signal generation module
    #[error("GPS error: {0}")]
    Gps(#[from] gps::Error),

    /// Error from geodetic coordinate validation or conversion
    #[error("Geometry error: {0}")]
    Geometry(#[from] geometry::Error),

    /// Error when performing I/O operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Error when parsing time values
    #[error("Time parsing error: {0}")]
    TimeParseError(#[from] jiff::Error),

    /// Error from `HackRF` control/USB layer
    #[error("HackRF error: {0}")]
    Hackrf(#[from] libhackrf::error::Error),

    /// Error related to command-line argument parsing or validation
    #[error("Command line argument error: {0}")]
    CliError(String),

    /// General application error with a message
    #[error("Application error: {0}")]
    Other(String),

    /// Error originating from a TX backend (with optional context)
    #[error("TX backend `{backend}` ({context}): {source}")]
    TxBackendWithSource {
        backend: &'static str,
        context: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Error originating from a TX backend (message only)
    #[error("TX backend `{backend}`: {message}")]
    TxBackendMsg {
        backend: &'static str,
        message: String,
    },
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

    pub fn tx_backend_with_source<E>(
        backend: &'static str, context: impl Into<String>, source: E,
    ) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Error::TxBackendWithSource {
            backend,
            context: context.into(),
            source: Box::new(source),
        }
    }

    pub fn tx_backend_msg(
        backend: &'static str, message: impl Into<String>,
    ) -> Self {
        Error::TxBackendMsg {
            backend,
            message: message.into(),
        }
    }
}
