use thiserror::Error;

/// Custom error type for the geometry crate
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("Invalid coordinates: latitude={latitude}, longitude={longitude}")]
    InvalidCoordinates { latitude: f64, longitude: f64 },

    #[error("Invalid ECEF coordinates: x={x}, y={y}, z={z}")]
    InvalidEcef { x: f64, y: f64, z: f64 },

    #[error("Invalid NEU coordinates: north={north}, east={east}, up={up}")]
    InvalidNeu { north: f64, east: f64, up: f64 },

    #[error("Invalid azimuth-elevation: azimuth={az}, elevation={el}")]
    InvalidAzel { az: f64, el: f64 },

    #[error("Coordinate conversion error: {0}")]
    ConversionError(String),

    #[error("Float parsing error: {0}")]
    ParseFloatError(#[from] std::num::ParseFloatError),

    #[error("Integer parsing error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("Geometry error: {0}")]
    Other(String),
}

impl Error {
    /// Create a new error with a message
    #[inline]
    pub fn msg(message: impl Into<String>) -> Self {
        Error::Other(message.into())
    }

    /// Create a new error for invalid coordinates
    #[inline]
    pub fn invalid_coordinates(latitude: f64, longitude: f64) -> Self {
        Error::InvalidCoordinates {
            latitude,
            longitude,
        }
    }

    /// Create a new error for invalid ECEF coordinates
    #[inline]
    pub fn invalid_ecef(x: f64, y: f64, z: f64) -> Self {
        Error::InvalidEcef { x, y, z }
    }

    /// Create a new error for invalid NEU coordinates
    #[inline]
    pub fn invalid_neu(north: f64, east: f64, up: f64) -> Self {
        Error::InvalidNeu { north, east, up }
    }

    /// Create a new error for invalid azimuth-elevation
    #[inline]
    pub fn invalid_azel(az: f64, el: f64) -> Self {
        Error::InvalidAzel { az, el }
    }

    /// Create a new error for coordinate conversion
    #[inline]
    pub fn conversion_error(message: impl Into<String>) -> Self {
        Error::ConversionError(message.into())
    }
}
