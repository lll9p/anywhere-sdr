use std::fmt::{self, Display};

use thiserror::Error;

struct FinalizationFailureDisplay<'a>(&'a [Error]);

impl Display for FinalizationFailureDisplay<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, error) in self.0.iter().enumerate() {
            if index != 0 {
                formatter.write_str("; ")?;
            }
            write!(formatter, "{error}")?;
        }
        Ok(())
    }
}

fn display_finalization_failures(
    failures: &[Error],
) -> FinalizationFailureDisplay<'_> {
    FinalizationFailureDisplay(failures)
}

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

    /// Multiple TX sinks failed during ordered finalization
    #[error(
        "multiple TX finalization failures: first: {first}; additional: {}",
        display_finalization_failures(.additional)
    )]
    MultipleFinalizationFailures {
        /// First failure in configured sink order
        #[source]
        first: Box<Error>,
        /// Remaining failures in configured sink order
        additional: Vec<Error>,
    },

    /// A run and its output finalization both failed
    #[error(
        "run failed: {primary}; output finalization also failed: \
         {finalization}"
    )]
    RunAndFinalizationFailed {
        /// Primary generation or sink-write failure
        #[source]
        primary: Box<Error>,
        /// Secondary ordered output-finalization failure
        finalization: Box<Error>,
    },

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

pub(crate) fn resolve_finalization_failures(
    failures: Vec<Error>,
) -> Result<(), Error> {
    let mut failures = failures.into_iter();
    let Some(first) = failures.next() else {
        return Ok(());
    };
    let Some(second) = failures.next() else {
        return Err(first);
    };
    let mut additional = vec![second];
    additional.extend(failures);
    Err(Error::MultipleFinalizationFailures {
        first: Box::new(first),
        additional,
    })
}

pub(crate) fn resolve_run_and_finish<T>(
    run: Result<T, Error>, finish: Result<(), Error>,
) -> Result<T, Error> {
    match (run, finish) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(run), Ok(())) => Err(run),
        (Ok(_), Err(finish)) => Err(finish),
        (Err(primary), Err(finalization)) => {
            Err(Error::RunAndFinalizationFailed {
                primary: Box::new(primary),
                finalization: Box::new(finalization),
            })
        }
    }
}
