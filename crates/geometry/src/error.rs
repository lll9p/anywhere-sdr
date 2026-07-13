use thiserror::Error;

/// Custom error type for the geometry crate
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Error when geodetic coordinates are non-finite or outside valid ranges
    #[error(
        "Invalid geodetic coordinates: latitude={latitude_radians} rad, \
         longitude={longitude_radians} rad, height={height_meters} m"
    )]
    InvalidCoordinates {
        /// Latitude value that caused the error, in radians
        latitude_radians: f64,
        /// Longitude value that caused the error, in radians
        longitude_radians: f64,
        /// Height value that caused the error, in meters
        height_meters: f64,
    },

    /// Error when ECEF coordinates are invalid
    #[error("Invalid ECEF coordinates: x={x}, y={y}, z={z}")]
    InvalidEcef {
        /// X coordinate value that caused the error
        x: f64,
        /// Y coordinate value that caused the error
        y: f64,
        /// Z coordinate value that caused the error
        z: f64,
    },

    /// Error when the ECEF vector is at or numerically near Earth center
    #[error(
        "ECEF position is too close to Earth center for unique geodetic \
         coordinates"
    )]
    EcefOrigin,

    /// Error when iterative ECEF conversion does not converge
    #[error(
        "ECEF conversion did not converge after {iterations} iterations: \
         x={x}, y={y}, z={z}"
    )]
    EcefConversionDidNotConverge {
        /// X coordinate in meters
        x: f64,
        /// Y coordinate in meters
        y: f64,
        /// Z coordinate in meters
        z: f64,
        /// Number of attempted iterations
        iterations: usize,
    },

    /// Error when NEU coordinates are invalid
    #[error("Invalid NEU coordinates: north={north}, east={east}, up={up}")]
    InvalidNeu {
        /// North coordinate value that caused the error
        north: f64,
        /// East coordinate value that caused the error
        east: f64,
        /// Up coordinate value that caused the error
        up: f64,
    },

    /// Error when a NEU vector does not define a unique azimuth
    #[error(
        "NEU vector has no unique azimuth: north={north}, east={east}, up={up}"
    )]
    UndefinedNeuDirection {
        /// North component in meters
        north: f64,
        /// East component in meters
        east: f64,
        /// Up component in meters
        up: f64,
    },

    /// Error when azimuth-elevation coordinates are invalid
    #[error("Invalid azimuth-elevation: azimuth={az}, elevation={el}")]
    InvalidAzel {
        /// Azimuth value that caused the error
        az: f64,
        /// Elevation value that caused the error
        el: f64,
    },

    /// Error during coordinate system conversion
    #[error("Coordinate conversion error: {0}")]
    ConversionError(String),

    /// Error when parsing floating-point values
    #[error("Float parsing error: {0}")]
    ParseFloatError(#[from] std::num::ParseFloatError),

    /// Error when parsing integer values
    #[error("Integer parsing error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    /// General geometry error with a message
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
    pub fn invalid_coordinates(
        latitude_radians: f64, longitude_radians: f64, height_meters: f64,
    ) -> Self {
        Error::InvalidCoordinates {
            latitude_radians,
            longitude_radians,
            height_meters,
        }
    }

    /// Create a new error for invalid ECEF coordinates
    #[inline]
    pub fn invalid_ecef(x: f64, y: f64, z: f64) -> Self {
        Error::InvalidEcef { x, y, z }
    }

    /// Create a new error for an ECEF conversion that did not converge
    #[inline]
    pub fn ecef_conversion_did_not_converge(
        x: f64, y: f64, z: f64, iterations: usize,
    ) -> Self {
        Error::EcefConversionDidNotConverge {
            x,
            y,
            z,
            iterations,
        }
    }

    /// Create a new error for invalid NEU coordinates
    #[inline]
    pub fn invalid_neu(north: f64, east: f64, up: f64) -> Self {
        Error::InvalidNeu { north, east, up }
    }

    /// Create a new error for a NEU vector without a unique azimuth
    #[inline]
    pub fn undefined_neu_direction(north: f64, east: f64, up: f64) -> Self {
        Error::UndefinedNeuDirection { north, east, up }
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
