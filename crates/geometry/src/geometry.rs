//! Geodetic coordinate transformations and navigation calculations.
//!
//! This crate provides types and functions for working with various coordinate
//! systems used in GPS/GNSS applications, including:
//! - Geodetic coordinates (latitude, longitude, height)
//! - Earth-Centered, Earth-Fixed (ECEF) coordinates
//! - North-East-Up (NEU) local tangent plane coordinates
//! - Azimuth/Elevation (`AzEl`) coordinates
//!
//! It implements formulas from <http://www.movable-type.co.uk/scripts/latlong.html>
//! and standard WGS-84 coordinate transformations.

/// Coordinate system types and implementations
mod coordinates;
/// Error types for geometry operations
mod error;
#[cfg(test)]
mod tests;
/// Traits for coordinate system operations
mod traits;
/// Coordinate system transformation functions
mod transformation;
pub use coordinates::{Azel, Ecef, Location, NavigationTarget, Neu};
pub use error::Error;
pub use traits::LocationMath;
