use thiserror::Error;

/// Custom error type for the parsing crate
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    #[error("Float parsing error: {0}")]
    ParseFloatError(#[from] std::num::ParseFloatError),

    #[error("Integer parsing error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("Invalid NMEA GGA format: {0}")]
    InvalidNmeaFormat(String),

    #[error("Invalid user motion format: {0}")]
    InvalidUserMotionFormat(String),

    #[error("Invalid coordinates: latitude={latitude}, longitude={longitude}")]
    InvalidCoordinates { latitude: f64, longitude: f64 },

    #[error("Missing field: {0}")]
    MissingField(String),

    #[error("Parsing error: {0}")]
    Other(String),
}

impl Error {
    /// Create a new error with a message
    #[inline]
    pub fn msg(message: impl Into<String>) -> Self {
        Error::Other(message.into())
    }

    /// Create a new error for invalid NMEA format
    #[inline]
    pub fn invalid_nmea(message: impl Into<String>) -> Self {
        Error::InvalidNmeaFormat(message.into())
    }

    /// Create a new error for invalid user motion format
    #[inline]
    pub fn invalid_user_motion(message: impl Into<String>) -> Self {
        Error::InvalidUserMotionFormat(message.into())
    }

    /// Create a new error for invalid coordinates
    #[inline]
    pub fn invalid_coordinates(latitude: f64, longitude: f64) -> Self {
        Error::InvalidCoordinates {
            latitude,
            longitude,
        }
    }

    /// Create a new error for missing field
    #[inline]
    pub fn missing_field(field: impl Into<String>) -> Self {
        Error::MissingField(field.into())
    }
}

/// Result type for the parsing crate
pub type Result<T> = std::result::Result<T, Error>;
