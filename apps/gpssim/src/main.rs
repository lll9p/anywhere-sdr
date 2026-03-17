#![allow(clippy::missing_docs_in_private_items)]
//! Command-line GPS L1 C/A signal simulator for software-defined radio
//! applications.
//!
//! This application provides a command-line interface to the GPS signal
//! generation functionality in the `gps` crate. It allows users to generate GPS
//! L1 C/A signals based on RINEX navigation files and user-defined receiver
//! positions.

/// Command-line interface definition and implementation
mod cli;
/// Error types for the application
mod error;
/// Interactive terminal UI
mod tui;
/// TUI configuration model
mod tui_config;
/// Transmission backends (file / SDR hardware)
mod tx;
/// Utility functions for logging and diagnostics
mod utils;

use clap::Parser;
pub use error::Error;

/// Main entry point for the GPS signal simulator application.
///
/// Initializes logging, parses command-line arguments, and runs the simulation
/// based on the provided configuration.
///
/// # Returns
/// * `Ok(())` - If the simulation completes successfully
/// * `Err(Error)` - If an error occurs during simulation
pub fn main() -> Result<(), Error> {
    let cli = cli::Args::parse();

    if cli.tui {
        let log_buffer = utils::LogBuffer::new(2000);
        let _guard = utils::tracing_init_tui(log_buffer.clone());
        return tui::run(&cli, log_buffer);
    }

    let _guard = utils::tracing_init();

    cli.run()
}
