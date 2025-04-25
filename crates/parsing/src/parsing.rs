//! File format parsing for GPS simulation input data.
//!
//! This crate provides parsers for various file formats used in GPS simulation:
//! - NMEA GGA sentences for position data
//! - User motion files in ECEF and LLH formats
//!
//! The parsers convert the input data into appropriate coordinate structures
//! that can be used by the GPS signal generator.

/// Error types for parsing operations
mod error;
/// NMEA sentence parsing implementation
mod nmea;
/// User motion file parsing implementation
mod user_motion;

pub use error::Error;
pub use nmea::read_nmea_gga;
pub use user_motion::{read_user_motion, read_user_motion_llh};
