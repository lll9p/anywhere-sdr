use constants::{OMEGA_EARTH, SPEED_OF_LIGHT};
use geometry::{Azel, Ecef, Location, LocationMath, Neu};

use crate::{
    datetime::{GpsTime, TimeRange},
    delay::ionospheric_delay,
    eph::Ephemeris,
    ionoutc::IonoUtc,
};

///  \brief Compute range between a satellite and the receiver
///  \param[out] rho The computed range
///  \param[in] eph Ephemeris data of the satellite
///  \param[in] g GPS time at time of receiving the signal
///  \param[in] xyz position of the receiver
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
