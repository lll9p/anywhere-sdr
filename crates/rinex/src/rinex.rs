//! RINEX (Receiver Independent Exchange Format) parser for GPS navigation
//! files.
//!
//! This crate provides functionality to parse RINEX navigation files containing
//! GPS ephemeris data. It supports the standard RINEX 2.x format commonly used
//! for distributing GPS satellite orbit information.

/// GPS satellite ephemeris data structures and builders
pub mod ephemeris;
/// Error types for RINEX parsing operations
pub mod error;
/// RINEX file parsing rules and implementation
pub mod rule;
/// UTC time conversion utilities
pub mod utc;
/// Utility functions for RINEX parsing
pub mod utils;
pub use error::Error;
pub use rule::Rinex;
