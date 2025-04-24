use constants::{OMEGA_EARTH, SPEED_OF_LIGHT};
use geometry::{Azel, Ecef, Location, LocationMath, Neu};

use crate::{
    datetime::{GpsTime, TimeRange},
    delay::ionospheric_delay,
    ephemeris::Ephemeris,
    ionoutc::IonoUtc,
};

/// Computes the range between a satellite and the receiver.
///
/// This function calculates the pseudorange, geometric distance, range rate,
/// azimuth/elevation angles, and ionospheric delay between a satellite and
/// the receiver at a specific time. It accounts for:
///
/// - Satellite motion during signal propagation (light time)
/// - Earth rotation during signal propagation
/// - Satellite clock offset
/// - Ionospheric delay
///
/// The calculation follows these steps:
/// 1. Compute satellite position at reception time
/// 2. Calculate initial light time
/// 3. Extrapolate satellite position backward to transmission time
/// 4. Apply Earth rotation correction
/// 5. Recalculate geometric range
/// 6. Apply satellite clock correction to get pseudorange
/// 7. Calculate range rate (Doppler)
/// 8. Calculate azimuth and elevation angles
/// 9. Add ionospheric delay
///
/// # Arguments
/// * `eph` - Ephemeris data of the satellite
/// * `ionoutc` - Ionospheric and UTC parameters
/// * `time` - GPS time at the moment of signal reception
/// * `xyz` - Position of the receiver in ECEF coordinates
///
/// # Returns
/// A `TimeRange` structure containing the computed range information
pub fn compute_range(
    eph: &Ephemeris, ionoutc: &IonoUtc, time: &GpsTime, xyz: &Ecef,
) -> TimeRange {
    let mut rho = TimeRange::default();
    // SV position at time of the pseudorange observation.
    let (mut pos, vel, clk) = eph.compute_satellite_state(time);
    // Receiver to satellite vector and light-time.
    let los = Ecef::from(&pos) - xyz;

    let tau = los.norm() / SPEED_OF_LIGHT;
    // Extrapolate the satellite position backwards to the transmission time.
    pos[0] -= vel[0] * tau;
    pos[1] -= vel[1] * tau;
    pos[2] -= vel[2] * tau;
    let xrot = pos[0] + pos[1] * OMEGA_EARTH * tau;
    let yrot = pos[1] - pos[0] * OMEGA_EARTH * tau;
    pos[0] = xrot;
    pos[1] = yrot;
    // New observer to satellite vector and satellite range.
    let los = Ecef::from(&pos) - xyz;
    // sub_vect(&mut los, &pos, xyz);
    let range = los.norm();
    rho.distance = range;
    // Pseudorange.
    rho.range = range - SPEED_OF_LIGHT * clk[0];
    // Relative velocity of SV and receiver.
    let vel = Ecef::from(&vel);
    let rate = vel.dot_prod(&los) / range;
    // Pseudorange rate.
    rho.rate = rate; // - SPEED_OF_LIGHT*clk[1];
    // Time of application.
    rho.time = time.clone();

    // Azimuth and elevation angles.
    let llh = Location::from(xyz);
    let neu = Neu::from_ecef(&los, llh.ltcmat());
    rho.azel = Azel::from(&neu);
    // Add ionospheric delay
    rho.iono_delay = ionospheric_delay(ionoutc, time, &llh, &rho.azel);
    rho.range += rho.iono_delay;
    rho
}
