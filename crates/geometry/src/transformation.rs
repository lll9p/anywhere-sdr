use constants::*;

use crate::{coordinates::*, traits::LocationMath};
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
impl From<&[f64; 3]> for Location {
    fn from(value: &[f64; 3]) -> Self {
        Self {
            latitude: value[0],
            longitude: value[1],
            height: value[2],
        }
    }
}
impl From<&Location> for Ecef {
    /// Converts LLH to ECEF using WGS84 ellipsoid parameters:
    /// N = a / √(1 - e²·sin²φ)
    /// x = (N + h)cosφ·cosλ
    /// y = (N + h)cosφ·sinλ
    /// z = ((1 - e²)N + h)sinφ
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
impl From<&[f64; 3]> for Ecef {
    fn from(value: &[f64; 3]) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: value[2],
        }
    }
}
impl From<&Ecef> for Neu {
    /// Transforms ECEF to NEU using rotation matrix:
    /// `neu = R·(ecef - reference_ecef)`
    /// where R is the local tangent plane rotation matrix
    fn from(value: &Ecef) -> Self {
        let ltcmat = Location::from(value).ltcmat();
        Self::from_ecef(value, ltcmat)
    }
}
impl From<&[f64; 3]> for Neu {
    fn from(value: &[f64; 3]) -> Self {
        Self {
            north: value[0],
            east: value[1],
            up: value[2],
        }
    }
}
impl From<&[f64; 2]> for Azel {
    fn from(value: &[f64; 2]) -> Self {
        Self {
            az: value[0],
            el: value[1],
        }
    }
}

impl From<&Neu> for Azel {
    /// Converts NEU to Azimuth/Elevation:
    /// azimuth = atan2(east, north) [adjusted to 0-2π]
    /// elevation = atan2(up, √(north² + east²))
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
