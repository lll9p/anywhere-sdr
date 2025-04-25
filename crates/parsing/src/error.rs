use thiserror::Error;

/// Custom error type for the parsing crate
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Error when performing I/O operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Error when parsing CSV files
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    /// Error when parsing floating-point values
    #[error("Float parsing error: {0}")]
    ParseFloatError(#[from] std::num::ParseFloatError),

    /// Error when parsing integer values
    #[error("Integer parsing error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    /// Error when parsing NMEA GGA sentences with invalid format
    #[error("Invalid NMEA GGA format: {0}")]
    InvalidNmeaFormat(String),

    /// Error when parsing user motion files with invalid format
    #[error("Invalid user motion format: {0}")]
    InvalidUserMotionFormat(String),

    /// Error when coordinates are outside valid ranges
    #[error("Invalid coordinates: latitude={latitude}, longitude={longitude}")]
    InvalidCoordinates {
        /// Latitude value that caused the error
        latitude: f64,
        /// Longitude value that caused the error
        longitude: f64,
    },

    /// Error when a required field is missing in the input data
    #[error("Missing field: {0}")]
    MissingField(String),

    /// General parsing error with a message
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
