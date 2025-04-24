use thiserror::Error;

use crate::rule::Rule;

/// Custom error type for the RINEX crate
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("RINEX file cannot be read")]
    ReadRinex(#[from] std::io::Error),

    #[error("Cannot parse to float: {0}")]
    ParseFloat(#[from] std::num::ParseFloatError),

    #[error("Cannot parse to integer: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("Cannot parse RINEX file: {0}")]
    ParseFile(#[from] Box<pest::error::Error<Rule>>),

    #[error("Cannot parse rule: {0}")]
    Rule(String),

    #[error("RINEX builder error: {0}")]
    RinexBuilder(String),

    #[error("Ephemeris builder error: {0}")]
    EphemerisBuilder(String),

    #[error("Time error: {0}")]
    Jiff(#[from] jiff::Error),

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
