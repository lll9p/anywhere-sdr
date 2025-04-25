//! Utility functions for the gpssim application.
//!
//! This module provides helper functions for logging, diagnostics, and other
//! utility operations needed by the application.

/// Initializes the tracing system for application logging.
///
/// Sets up a daily rolling file logger that writes to app.log in the current
/// directory. Configures the logging level, format, and other options.
///
/// # Returns
/// A guard that must be kept alive for the duration of the application to
/// ensure log messages are properly flushed.
pub fn tracing_init() -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender = tracing_appender::rolling::daily("./", "app.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_file(true)
        .with_line_number(true)
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();
    guard
}
