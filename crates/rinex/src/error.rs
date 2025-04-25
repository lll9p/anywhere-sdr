use thiserror::Error;

use crate::rule::Rule;

/// Custom error type for the RINEX crate
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Error when reading RINEX file from disk
    #[error("RINEX file cannot be read")]
    ReadRinex(#[from] std::io::Error),

    /// Error when parsing floating point values from RINEX file
    #[error("Cannot parse to float: {0}")]
    ParseFloat(#[from] std::num::ParseFloatError),

    /// Error when parsing integer values from RINEX file
    #[error("Cannot parse to integer: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    /// Error from the pest parser when parsing RINEX file
    #[error("Cannot parse RINEX file: {0}")]
    ParseFile(#[from] Box<pest::error::Error<Rule>>),

    /// Error when processing a specific parsing rule
    #[error("Cannot parse rule: {0}")]
    Rule(String),

    /// Error when building a RINEX object
    #[error("RINEX builder error: {0}")]
    RinexBuilder(String),

    /// Error when building an Ephemeris object
    #[error("Ephemeris builder error: {0}")]
    EphemerisBuilder(String),

    /// Error related to time calculations
    #[error("Time error: {0}")]
    Jiff(#[from] jiff::Error),

    /// Unknown or unspecified error
    #[error("Unknown RINEX error")]
    Unknown,
}

impl Error {
    /// Create a new rule error
    #[inline]
    pub fn rule(message: impl Into<String>) -> Self {
        Error::Rule(message.into())
    }

    /// Create a new RINEX builder error
    #[inline]
    pub fn rinex_builder(message: impl Into<String>) -> Self {
        Error::RinexBuilder(message.into())
    }

    /// Create a new ephemeris builder error
    #[inline]
    pub fn ephemeris_builder(message: impl Into<String>) -> Self {
        Error::EphemerisBuilder(message.into())
    }
}
