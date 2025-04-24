use constants::*;

use crate::{coordinates::*, traits::LocationMath};
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
/// # Special Cases
/// - If the ECEF vector is near zero (invalid), returns (0°, 0°, -a) where a is
///   Earth's radius
impl From<&Ecef> for Location {
    fn from(ecef: &Ecef) -> Self {
        let a: f64 = WGS84_RADIUS;
        let eps: f64 = 1.0e-3;
        let e: f64 = WGS84_ECCENTRICITY;
        let e2: f64 = e.powi(2);

        let mut dz: f64;
        let mut zdz: f64;
        let mut nh: f64;
        let mut slat: f64;
        let mut n: f64;
        let mut dz_new: f64;
        if ecef.norm() < eps {
            // Invalid ECEF vector
            return Self {
                latitude: 0.,
                longitude: 0.,
                height: -a,
            };
        }
        let x = ecef.x;
        let y = ecef.y;
        let z = ecef.z;
        let rho2: f64 = x * x + y * y;
        dz = e2 * z;
        loop {
            zdz = z + dz;
            nh = (rho2 + zdz * zdz).sqrt();
            slat = zdz / nh;
            n = a / (1.0 - e2 * slat * slat).sqrt();
            dz_new = n * e2 * slat;
            if (dz - dz_new).abs() < eps {
                break;
            }

            dz = dz_new;
        }
        let llh0 = zdz.atan2(rho2.sqrt());
        let llh1 = y.atan2(x);
        let llh2 = nh - n;

        Self {
            latitude: llh0,
            longitude: llh1,
            height: llh2,
        }
    }
}
/// Creates a Location from a 3-element array of [latitude, longitude, height].
///
/// This is a convenience method for creating a Location from an array,
/// which is useful when working with data from external sources.
///
/// # Arguments
/// * `value` - Array containing [latitude, longitude, height] values
impl From<&[f64; 3]> for Location {
    fn from(value: &[f64; 3]) -> Self {
        Self {
            latitude: value[0],
            longitude: value[1],
            height: value[2],
        }
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

        let clat: f64 = loc.latitude.cos();
        let slat: f64 = loc.latitude.sin();
        let clon: f64 = loc.longitude.cos();
        let slon: f64 = loc.longitude.sin();
        let d: f64 = e * slat;

        let n: f64 = a / (1. - d.powi(2)).sqrt();
        let nph: f64 = n + loc.height;

        let tmp: f64 = nph * clat;
        let x = tmp * clon;
        let y = tmp * slon;
        let z = ((1. - e2) * n + loc.height) * slat;
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
impl From<&Ecef> for Neu {
    fn from(value: &Ecef) -> Self {
        let ltcmat = Location::from(value).ltcmat();
        Self::from_ecef(value, ltcmat)
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
/// Creates an Azimuth-Elevation coordinate from a 2-element array of [azimuth,
/// elevation].
///
/// This is a convenience method for creating an Azel coordinate from an array,
/// which is useful when working with data from external sources.
///
/// # Arguments
/// * `value` - Array containing [azimuth, elevation] values in radians
impl From<&[f64; 2]> for Azel {
    fn from(value: &[f64; 2]) -> Self {
        Self {
            az: value[0],
            el: value[1],
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
impl From<&Neu> for Azel {
    fn from(neu: &Neu) -> Self {
        let mut az = neu.east.atan2(neu.north);
        if az < 0.0 {
            az += 2.0 * PI;
        }

        let ne = (neu.north * neu.north + neu.east * neu.east).sqrt();
        let el = neu.up.atan2(ne);
        Self { az, el }
    }
}
