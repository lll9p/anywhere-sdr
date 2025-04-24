use constants::{PI, SECONDS_IN_DAY, SPEED_OF_LIGHT};
use geometry::{Azel, Location};

use crate::{datetime::GpsTime, ionoutc::IonoUtc};

/// Calculates the ionospheric delay for a GPS signal.
///
/// This function implements the Klobuchar ionospheric model to estimate
/// the signal delay caused by the ionosphere. The model uses broadcast
/// parameters (alpha and beta) and takes into account:
/// - Receiver location (latitude and longitude)
/// - Satellite elevation and azimuth
/// - Time of day
///
/// The ionospheric delay varies with:
/// - Receiver's geomagnetic latitude
/// - Local time (maximum delay occurs around 14:00 local time)
/// - Satellite elevation angle (lower elevation = longer path through
///   ionosphere)
///
/// # Arguments
/// * `ionoutc` - Ionospheric model parameters from the navigation message
/// * `time` - Current GPS time
/// * `llh` - Receiver location in geodetic coordinates
/// * `azel` - Satellite azimuth and elevation angles
///
/// # Returns
/// The estimated ionospheric delay in meters (0.0 if ionospheric correction is
/// disabled)
#[allow(non_snake_case)]
pub fn ionospheric_delay(
    ionoutc: &IonoUtc, time: &GpsTime, llh: &Location, azel: &Azel,
) -> f64 {
    let iono_delay: f64;
    if !ionoutc.enable {
        // No ionospheric delay
        return 0.0;
    }
    let E = azel.el / PI;
    let phi_u = llh.latitude / PI;
    let lam_u = llh.longitude / PI;
    let F = 1.0 + 16.0 * (0.53 - E).powf(3.0);
    if ionoutc.vflg {
        let mut PER: f64;

        // Earth's central angle between the user position and the earth
        // projection of ionospheric intersection point (semi-circles)
        let psi = 0.0137 / (E + 0.11) - 0.022;

        // Geodetic latitude of the earth projection of the ionospheric
        // intersection point (semi-circles)
        let phi_i = phi_u + psi * azel.az.cos();
        let phi_i = phi_i.clamp(-0.416, 0.416);

        // Geodetic longitude of the earth projection of the ionospheric
        // intersection point (semi-circles)
        let lam_i = lam_u + psi * azel.az.sin() / (phi_i * PI).cos();
        // Geomagnetic latitude of the earth projection of the ionospheric
        // intersection point (mean ionospheric height assumed 350 km)
        // (semi-circles)
        let phi_m = phi_i + 0.064 * ((lam_i - 1.617) * PI).cos();
        let phi_m2 = phi_m * phi_m;
        let phi_m3 = phi_m2 * phi_m;
        let mut AMP = ionoutc.alpha0
            + ionoutc.alpha1 * phi_m
            + ionoutc.alpha2 * phi_m2
            + ionoutc.alpha3 * phi_m3;
        if AMP < 0.0 {
            AMP = 0.0;
        }
        PER = ionoutc.beta0
            + ionoutc.beta1 * phi_m
            + ionoutc.beta2 * phi_m2
            + ionoutc.beta3 * phi_m3;
        if PER < 72000.0 {
            PER = 72000.0;
        }
        // Local time (sec)
        let t = (time.sec + 0.5 * SECONDS_IN_DAY * lam_i)
            .rem_euclid(SECONDS_IN_DAY);

        // Phase (radians)
        let X = 2.0 * PI * (t - 50400.0) / PER;
        if X.abs() < 1.57 {
            let X2 = X * X;
            let X4 = X2 * X2;
            iono_delay = F
                * (5.0e-9 + AMP * (1.0 - X2 / 2.0 + X4 / 24.0))
                * SPEED_OF_LIGHT;
        } else {
            iono_delay = F * 5.0e-9 * SPEED_OF_LIGHT;
        }
    } else {
        iono_delay = F * 5.0e-9 * SPEED_OF_LIGHT;
    }
    iono_delay
}
