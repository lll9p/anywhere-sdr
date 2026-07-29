use std::fmt::{self, Display};

use thiserror::Error;

struct FailureListDisplay<'a>(&'a [Error]);

impl Display for FailureListDisplay<'_> {
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

fn display_failures(failures: &[Error]) -> FailureListDisplay<'_> {
    FailureListDisplay(failures)
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

    /// Exact sentinel for a user-requested streaming cancellation
    #[error("run cancelled")]
    RunCancelled,

    /// A worker panicked in an unwind-capable build
    #[error("worker thread panicked: {message}")]
    WorkerPanicked {
        /// String panic payload or a stable fallback
        message: String,
    },

    /// A worker exited without sending a terminal event
    #[error("worker exited without a terminal event")]
    WorkerExitedWithoutTerminalEvent,

    /// Multiple TX sinks failed during ordered finalization
    #[error(
        "multiple TX finalization failures: first: {first}; additional: {}",
        display_failures(.additional)
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
        /// Primary generation, sink-write, callback, or cancellation failure
        #[source]
        primary: Box<Error>,
        /// Secondary ordered output-finalization failure
        finalization: Box<Error>,
    },

    /// A terminal operation failed
    #[error("terminal operation `{operation}` failed: {source}")]
    TerminalOperation {
        /// Stable operation label
        operation: &'static str,
        /// Underlying terminal I/O failure
        #[source]
        source: std::io::Error,
    },

    /// Multiple terminal restoration operations failed
    #[error(
        "multiple terminal cleanup failures: first: {first}; additional: {}",
        display_failures(.additional)
    )]
    MultipleTerminalCleanupFailures {
        /// First failure in restoration order
        #[source]
        first: Box<Error>,
        /// Remaining failures in restoration order
        additional: Vec<Error>,
    },

    /// TUI execution and terminal restoration both failed
    #[error(
        "TUI operation failed: {primary}; terminal cleanup also failed: \
         {cleanup}"
    )]
    TuiAndTerminalCleanupFailed {
        /// Primary initialization, event-loop, or completed-run failure
        #[source]
        primary: Box<Error>,
        /// Secondary ordered terminal restoration failure
        cleanup: Box<Error>,
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

    pub(crate) fn terminal_operation(
        operation: &'static str, source: std::io::Error,
    ) -> Self {
        Error::TerminalOperation { operation, source }
    }
}

fn resolve_ordered_failures<F>(
    failures: Vec<Error>, aggregate: F,
) -> Result<(), Error>
where
    F: FnOnce(Box<Error>, Vec<Error>) -> Error,
{
    let mut failures = failures.into_iter();
    let Some(first) = failures.next() else {
        return Ok(());
    };
    let Some(second) = failures.next() else {
        return Err(first);
    };
    let mut additional = vec![second];
    additional.extend(failures);
    Err(aggregate(Box::new(first), additional))
}

pub(crate) fn resolve_finalization_failures(
    failures: Vec<Error>,
) -> Result<(), Error> {
    resolve_ordered_failures(failures, |first, additional| {
        Error::MultipleFinalizationFailures { first, additional }
    })
}

pub(crate) fn resolve_terminal_cleanup_failures(
    failures: Vec<Error>,
) -> Result<(), Error> {
    resolve_ordered_failures(failures, |first, additional| {
        Error::MultipleTerminalCleanupFailures { first, additional }
    })
}

pub(crate) fn attach_terminal_cleanup(
    primary: Error, cleanup: Result<(), Error>,
) -> Error {
    match cleanup {
        Ok(()) => primary,
        Err(cleanup) => Error::TuiAndTerminalCleanupFailed {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        },
    }
}

pub(crate) fn resolve_tui_and_terminal_cleanup<T>(
    primary: Result<T, Error>, cleanup: Result<(), Error>,
) -> Result<T, Error> {
    match primary {
        Ok(value) => cleanup.map(|()| value),
        Err(primary) => Err(attach_terminal_cleanup(primary, cleanup)),
    }
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
