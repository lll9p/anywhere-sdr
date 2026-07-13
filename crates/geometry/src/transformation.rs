use std::f64::consts::PI;

use constants::{PI as LEGACY_PI, WGS84_ECCENTRICITY, WGS84_RADIUS};

use crate::{Error, coordinates::*};
/// Converts Earth-Centered, Earth-Fixed (ECEF) coordinates to geodetic
/// coordinates.
///
/// This implementation uses an iterative method to convert ECEF Cartesian
/// coordinates to geodetic latitude, longitude, and height above the WGS-84
/// ellipsoid. The algorithm iteratively refines the latitude calculation until
/// convergence.
///
/// # Algorithm
/// 1. Calculate the distance from the Z-axis (rho)
/// 2. Initialize the height correction term (dz)
/// 3. Iteratively refine the latitude and height until convergence
/// 4. Calculate final latitude, longitude, and height
///
/// # Errors
/// Returns an error for non-finite components, the Earth-center origin, numeric
/// overflow, or failure to converge within the fixed iteration budget.
impl TryFrom<&Ecef> for Location {
    type Error = Error;

    fn try_from(ecef: &Ecef) -> Result<Self, Self::Error> {
        let a: f64 = WGS84_RADIUS;
        let convergence_meters: f64 = 1.0e-3;
        const MAX_ITERATIONS: usize = 16;
        let e: f64 = WGS84_ECCENTRICITY;
        let e2: f64 = e.powi(2);

        if !ecef.x.is_finite() || !ecef.y.is_finite() || !ecef.z.is_finite() {
            return Err(Error::invalid_ecef(ecef.x, ecef.y, ecef.z));
        }
        let x = ecef.x;
        let y = ecef.y;
        let z = ecef.z;
        if x == 0.0 && y == 0.0 && z == 0.0 {
            return Err(Error::EcefOrigin);
        }

        let horizontal_squared = x * x + y * y;
        let horizontal = if horizontal_squared.is_finite()
            && (horizontal_squared != 0.0 || (x == 0.0 && y == 0.0))
        {
            horizontal_squared.sqrt()
        } else {
            x.hypot(y)
        };

        let geocentric_radius = horizontal.hypot(z);
        if !geocentric_radius.is_finite() {
            return Err(Error::invalid_ecef(x, y, z));
        }
        if geocentric_radius < convergence_meters {
            return Err(Error::EcefOrigin);
        }
        if horizontal == 0.0 {
            let semi_minor_axis = a * (1.0 - e2).sqrt();
            return Self::try_from_radians(
                if z.is_sign_positive() {
                    PI / 2.0
                } else {
                    -PI / 2.0
                },
                0.0,
                z.abs() - semi_minor_axis,
            );
        }

        let mut correction = e2 * z;
        for _ in 0..MAX_ITERATIONS {
            let corrected_z = z + correction;
            if !corrected_z.is_finite() {
                return Err(Error::invalid_ecef(x, y, z));
            }
            let ellipsoid_radius_squared =
                horizontal_squared + corrected_z * corrected_z;
            let ellipsoid_radius = if ellipsoid_radius_squared.is_finite() {
                ellipsoid_radius_squared.sqrt()
            } else {
                horizontal.hypot(corrected_z)
            };
            let sin_latitude = corrected_z / ellipsoid_radius;
            let prime_vertical_radius =
                a / (1.0 - e2 * sin_latitude * sin_latitude).sqrt();
            let next_correction = prime_vertical_radius * e2 * sin_latitude;
            if (correction - next_correction).abs() <= convergence_meters {
                return Self::try_from_radians(
                    corrected_z.atan2(horizontal),
                    y.atan2(x),
                    ellipsoid_radius - prime_vertical_radius,
                );
            }
            correction = next_correction;
        }

        Err(Error::ecef_conversion_did_not_converge(
            x,
            y,
            z,
            MAX_ITERATIONS,
        ))
    }
}
/// Converts geodetic coordinates to Earth-Centered, Earth-Fixed (ECEF)
/// coordinates.
///
/// This implementation transforms latitude, longitude, and height (LLH)
/// coordinates to ECEF Cartesian coordinates using the WGS-84 ellipsoid model.
///
/// # Algorithm
/// The conversion uses the following formulas:
/// - N = a / √(1 - e²·sin²φ)  (radius of curvature in the prime vertical)
/// - x = (N + h)·cosφ·cosλ
/// - y = (N + h)·cosφ·sinλ
/// - z = ((1 - e²)·N + h)·sinφ
///
/// Where:
/// - φ is latitude
/// - λ is longitude
/// - h is height above ellipsoid
/// - a is semi-major axis
/// - e is eccentricity
impl From<&Location> for Ecef {
    fn from(loc: &Location) -> Self {
        let a: f64 = WGS84_RADIUS;
        let e: f64 = WGS84_ECCENTRICITY;
        let e2: f64 = e * e;

        let clat: f64 = loc.latitude_radians().cos();
        let slat: f64 = loc.latitude_radians().sin();
        let clon: f64 = loc.longitude_radians().cos();
        let slon: f64 = loc.longitude_radians().sin();
        let d: f64 = e * slat;

        let n: f64 = a / (1. - d.powi(2)).sqrt();
        let nph: f64 = n + loc.height_meters();

        let tmp: f64 = nph * clat;
        let x = tmp * clon;
        let y = tmp * slon;
        let z = ((1. - e2) * n + loc.height_meters()) * slat;
        Self { x, y, z }
    }
}
/// Creates an ECEF coordinate from a 3-element array of [x, y, z].
///
/// This is a convenience method for creating an ECEF coordinate from an array,
/// which is useful when working with data from external sources.
///
/// # Arguments
/// * `value` - Array containing [x, y, z] values in meters
impl From<&[f64; 3]> for Ecef {
    fn from(value: &[f64; 3]) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: value[2],
        }
    }
}
/// Converts ECEF coordinates to North-East-Up (NEU) local tangent plane
/// coordinates.
///
/// This implementation transforms ECEF coordinates to NEU coordinates using
/// the local tangent plane at the point itself. This is useful for visualizing
/// the local orientation at a specific point.
///
/// Note: This differs from the typical NEU conversion where the reference point
/// is separate from the point being converted. Here, the point itself is used
/// as the reference point.
///
/// # Algorithm
/// 1. Convert the ECEF point to geodetic coordinates
/// 2. Compute the local tangent plane rotation matrix at that point
/// 3. Apply the rotation matrix to transform to NEU coordinates
impl TryFrom<&Ecef> for Neu {
    type Error = Error;

    fn try_from(value: &Ecef) -> Result<Self, Self::Error> {
        let ltcmat = Location::try_from(value)?.ltcmat();
        Ok(Self::from_ecef(value, ltcmat))
    }
}
/// Creates a NEU coordinate from a 3-element array of [north, east, up].
///
/// This is a convenience method for creating a NEU coordinate from an array,
/// which is useful when working with data from external sources.
///
/// # Arguments
/// * `value` - Array containing [north, east, up] values in meters
impl From<&[f64; 3]> for Neu {
    fn from(value: &[f64; 3]) -> Self {
        Self {
            north: value[0],
            east: value[1],
            up: value[2],
        }
    }
}
/// Converts North-East-Up (NEU) coordinates to Azimuth-Elevation angles.
///
/// This implementation transforms NEU coordinates to azimuth and elevation
/// angles, which are commonly used for satellite tracking and antenna pointing.
///
/// # Algorithm
/// The conversion uses the following formulas:
/// - azimuth = atan2(east, north) [adjusted to 0-2π]
///   - 0° is north, 90° is east, 180° is south, 270° is west
/// - elevation = atan2(up, √(north² + east²))
///   - 0° is horizontal, 90° is vertical up
///
/// # Notes
/// - Azimuth is adjusted to be in the range [0, 2π]
/// - Elevation is in the range [-π/2, π/2]
impl TryFrom<&Neu> for Azel {
    type Error = Error;

    fn try_from(neu: &Neu) -> Result<Self, Self::Error> {
        if !neu.north.is_finite()
            || !neu.east.is_finite()
            || !neu.up.is_finite()
        {
            return Err(Error::invalid_neu(neu.north, neu.east, neu.up));
        }
        let horizontal_squared = neu.north * neu.north + neu.east * neu.east;
        let horizontal = if horizontal_squared.is_finite()
            && (horizontal_squared != 0.0
                || (neu.north == 0.0 && neu.east == 0.0))
        {
            horizontal_squared.sqrt()
        } else {
            neu.north.hypot(neu.east)
        };
        if horizontal == 0.0 {
            return Err(Error::undefined_neu_direction(
                neu.north, neu.east, neu.up,
            ));
        }

        let mut azimuth = neu.east.atan2(neu.north);
        if azimuth < 0.0 {
            azimuth += 2.0 * LEGACY_PI;
        }
        if azimuth >= 2.0 * PI {
            azimuth -= 2.0 * PI;
        }
        Self::try_from_radians(azimuth, neu.up.atan2(horizontal))
    }
}
