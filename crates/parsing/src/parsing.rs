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
use geometry::{Ecef, Location};
pub use nmea::read_nmea_gga;
pub use user_motion::{read_user_motion, read_user_motion_llh};

/// Validates degree-based LLH input and converts it to ECEF.
fn ecef_from_degrees(
    latitude_degrees: f64, longitude_degrees: f64, height_meters: f64,
) -> Result<Ecef, Error> {
    let location = Location::try_from_degrees(
        latitude_degrees,
        longitude_degrees,
        height_meters,
    )?;
    Ok(Ecef::from(&location))
}

/// Validates ECEF input through the fallible inverse geodetic domain.
fn validate_ecef(ecef: Ecef) -> Result<Ecef, Error> {
    Location::try_from(&ecef)?;
    Ok(ecef)
}

#[cfg(test)]
mod tests {
    use geometry::{Ecef, Error as GeometryError};

    use super::{Error, ecef_from_degrees, validate_ecef};

    #[test]
    fn geodetic_domain_errors_preserve_geometry_source() {
        let result = ecef_from_degrees(91.0, 0.0, 0.0);
        assert!(matches!(
            result,
            Err(Error::Geometry(GeometryError::InvalidCoordinates { .. }))
        ));
    }

    #[test]
    fn ecef_domain_errors_preserve_geometry_source() {
        let result = validate_ecef(Ecef::new(f64::NAN, 0.0, 0.0));
        assert!(matches!(
            result,
            Err(Error::Geometry(GeometryError::InvalidEcef { .. }))
        ));
    }
}
