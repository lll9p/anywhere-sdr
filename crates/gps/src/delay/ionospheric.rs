use crate::{constants::*, datetime::GpsTime, ionoutc::IonoUtc};

#[allow(non_snake_case)]
pub fn ionospheric_delay(
    ionoutc: &IonoUtc, g: &GpsTime, llh: &[f64; 3], azel: &[f64; 2],
) -> f64 {
    let iono_delay: f64;
    if !ionoutc.enable {
        // No ionospheric delay
        return 0.0;
    }
    let E = azel[1] / PI;
    let phi_u = llh[0] / PI;
    let lam_u = llh[1] / PI;
    let F = 1.0 + 16.0 * (0.53f64 - E).powf(3.0);
    if ionoutc.vflg {
        let mut PER: f64;

        // Earth's central angle between the user position and the earth
        // projection of ionospheric intersection point (semi-circles)
        let psi = 0.0137 / (E + 0.11) - 0.022;

        // Geodetic latitude of the earth projection of the ionospheric
        // intersection point (semi-circles)
        let phi_i = phi_u + psi * (azel[0]).cos();
        let phi_i = phi_i.clamp(-0.416, 0.416);

        // Geodetic longitude of the earth projection of the ionospheric
        // intersection point (semi-circles)
        let lam_i = lam_u + psi * (azel[0]).sin() / (phi_i * PI).cos();
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
        let mut t = SECONDS_IN_DAY / 2.0 * lam_i + g.sec;
        while t >= SECONDS_IN_DAY {
            t -= SECONDS_IN_DAY;
        }
        while t < 0.0 {
            t += SECONDS_IN_DAY;
        }
        // Phase (radians)
        let X = 2.0 * PI * (t - 50400.0) / PER;
        if (X).abs() < 1.57 {
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
