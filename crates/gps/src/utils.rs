use crate::{
    channel::Channel,
    constants::*,
    datetime::{GpsTime, TimeRange},
    delay::ionospheric_delay,
    eph::Ephemeris,
    geometry::{Azel, Ecef, Location, LocationMath, Neu},
    ionoutc::IonoUtc,
};
#[allow(dead_code)]
pub fn sub_vect(y: &mut [f64; 3], x1: &[f64; 3], x2: &[f64; 3]) {
    y[0] = x1[0] - x2[0];
    y[1] = x1[1] - x2[1];
    y[2] = x1[2] - x2[2];
}

#[allow(dead_code)]
pub fn norm_vect(x: &[f64; 3]) -> f64 {
    (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt()
}

#[allow(dead_code)]
pub fn dot_prod(x1: &[f64; 3], x2: &[f64; 3]) -> f64 {
    x1[0] * x2[0] + x1[1] * x2[1] + x1[2] * x2[2]
}
///  Convert Earth-centered Earth-fixed (ECEF) into Lat/Long/Height
///  \param[in] xyz Input Array of X, Y and Z ECEF coordinates
///  \param[out] llh Output Array of Latitude, Longitude and Height
#[allow(dead_code)]
pub fn xyz2llh(xyz_0: &[f64; 3], llh: &mut [f64; 3]) {
    let mut zdz: f64;
    let mut nh: f64;
    let mut slat: f64;
    let mut n: f64;
    let mut dz_new: f64;
    let a = WGS84_RADIUS;
    let e = WGS84_ECCENTRICITY;
    let eps = 1.0e-3;
    let e2 = e * e;
    if norm_vect(xyz_0) < eps {
        // Invalid ECEF vector
        llh[0] = 0.0;
        llh[1] = 0.0;
        llh[2] = -a;
        return;
    }
    let x = xyz_0[0];
    let y = xyz_0[1];
    let z = xyz_0[2];
    let rho2 = x * x + y * y;
    let mut dz = e2 * z;
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
    llh[0] = zdz.atan2(rho2.sqrt());
    llh[1] = y.atan2(x);
    llh[2] = nh - n;
}

/// Convert Lat/Long/Height into Earth-centered Earth-fixed (ECEF)
/// \param[in] llh Input Array of Latitude, Longitude and Height
/// \param[out] xyz Output Array of X, Y and Z ECEF coordinates
#[allow(dead_code)]
pub fn llh2xyz(llh: &[f64; 3], xyz_0: &mut [f64; 3]) {
    let a = WGS84_RADIUS;
    let e = WGS84_ECCENTRICITY;
    let e2 = e * e;
    let (slat, clat) = llh[0].sin_cos();
    let (slon, clon) = llh[1].sin_cos();
    let d = e * slat;
    let n = a / (1.0 - d * d).sqrt();
    let nph = n + llh[2];
    let tmp = nph * clat;
    xyz_0[0] = tmp * clon;
    xyz_0[1] = tmp * slon;
    xyz_0[2] = ((1.0 - e2) * n + llh[2]) * slat;
}

///  \brief Compute the intermediate matrix for LLH to ECEF
///  \param[in] llh Input position in Latitude-Longitude-Height format
///  \param[out] t Three-by-Three output matrix
#[allow(dead_code)]
pub fn ltcmat(llh: &[f64; 3], t: &mut [[f64; 3]; 3]) {
    let slat = llh[0].sin();
    let clat = llh[0].cos();
    let slon = llh[1].sin();
    let clon = llh[1].cos();
    t[0][0] = -slat * clon;
    t[0][1] = -slat * slon;
    t[0][2] = clat;
    t[1][0] = -slon;
    t[1][1] = clon;
    t[1][2] = 0.0;
    t[2][0] = clat * clon;
    t[2][1] = clat * slon;
    t[2][2] = slat;
}

///  \brief Convert Earth-centered Earth-Fixed to ?
/// \param[in] xyz Input position as vector in ECEF format
/// \param[in] t Intermediate matrix computed by \ref ltcmat
/// \param[out] neu Output position as North-East-Up format
#[allow(dead_code)]
pub fn ecef2neu(xyz_0: &[f64; 3], t: &[[f64; 3]; 3], neu: &mut [f64; 3]) {
    neu[0] = t[0][0] * xyz_0[0] + t[0][1] * xyz_0[1] + t[0][2] * xyz_0[2];
    neu[1] = t[1][0] * xyz_0[0] + t[1][1] * xyz_0[1] + t[1][2] * xyz_0[2];
    neu[2] = t[2][0] * xyz_0[0] + t[2][1] * xyz_0[1] + t[2][2] * xyz_0[2];
}

///  \brief Convert North-East-Up to Azimuth + Elevation
/// \param[in] neu Input position in North-East-Up format
/// \param[out] azel Output array of azimuth + elevation as double
#[allow(dead_code)]
pub fn neu2azel(azel: &mut [f64; 2], neu: &[f64; 3]) {
    azel[0] = neu[1].atan2(neu[0]);
    if azel[0] < 0.0 {
        azel[0] += 2.0 * PI;
    }
    let ne = (neu[0] * neu[0] + neu[1] * neu[1]).sqrt();
    azel[1] = neu[2].atan2(ne);
}

/// !generate the C/A code sequence for a given Satellite Vehicle PRN
///  \param[in] prn PRN number of the Satellite Vehicle
///  \param[out] ca Caller-allocated integer array of 1023 bytes
pub fn codegen(ca: &mut [i32; CA_SEQ_LEN], prn: usize) {
    let delay: [usize; 32] = [
        5, 6, 7, 8, 17, 18, 139, 140, 141, 251, 252, 254, 255, 256, 257, 258,
        469, 470, 471, 472, 473, 474, 509, 512, 513, 514, 515, 516, 859, 860,
        861, 862,
    ];
    let mut g1: [i32; CA_SEQ_LEN] = [0; CA_SEQ_LEN];
    let mut g2: [i32; CA_SEQ_LEN] = [0; CA_SEQ_LEN];
    let mut r1: [i32; N_DWRD_SBF] = [0; N_DWRD_SBF];
    let mut r2: [i32; N_DWRD_SBF] = [0; N_DWRD_SBF];
    let mut c1: i32;
    let mut c2: i32;
    if !(1..=32).contains(&prn) {
        return;
    }
    for i in 0..N_DWRD_SBF {
        r2[i] = -1;
        r1[i] = r2[i];
    }
    for i in 0..CA_SEQ_LEN {
        g1[i] = r1[9];
        g2[i] = r2[9];
        c1 = r1[2] * r1[9];
        c2 = r2[1] * r2[2] * r2[5] * r2[7] * r2[8] * r2[9];
        for j in (1..10).rev() {
            r1[j] = r1[j - 1];
            r2[j] = r2[j - 1];
        }
        r1[0] = c1;
        r2[0] = c2;
    }
    let mut j = CA_SEQ_LEN - delay[prn - 1];
    for i in 0..CA_SEQ_LEN {
        ca[i] = (1 - g1[i] * g2[j % CA_SEQ_LEN]) / 2;
        j += 1;
    }
}

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
    let los = Ecef::from(pos) - xyz;

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
    let los = Ecef::from(pos) - xyz;
    // sub_vect(&mut los, &pos, xyz);
    let range = los.norm();
    rho.distance = range;
    // Pseudorange.
    rho.range = range - SPEED_OF_LIGHT * clk[0];
    // Relative velocity of SV and receiver.
    let vel = Ecef::from(vel);
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

pub fn allocate_channel(
    chan: &mut [Channel; MAX_CHAN], eph: &mut [Ephemeris; MAX_SAT],
    ionoutc: &mut IonoUtc, grx: &GpsTime, xyz: &Ecef, _elv_mask: f64,
    allocated_sat: &mut [i32; MAX_SAT],
) -> i32 {
    let mut nsat: i32 = 0;
    // let ref_0: [f64; 3] = [0., 0., 0.];
    // #[allow(unused_variables)]
    // let mut r_ref: f64 = 0.;
    // #[allow(unused_variables)]
    // let mut r_xyz: f64;
    let mut phase_ini: f64;
    for sv in 0..MAX_SAT {
        if let Some((azel, true)) = &eph[sv].check_visibility(grx, xyz, 0.0) {
            nsat += 1; // Number of visible satellites
            if allocated_sat[sv] == -1 {
                // Visible but not allocated
                //
                // Allocated new satellite
                let mut channel_index = 0;
                for (i, ichan) in chan.iter_mut().take(MAX_CHAN).enumerate() {
                    if ichan.prn == 0 {
                        // Initialize channel
                        ichan.prn = sv + 1;
                        ichan.azel = *azel;
                        // C/A code generation
                        codegen(&mut ichan.ca, ichan.prn);
                        // Generate subframe
                        eph[sv].generate_navigation_subframes(
                            ionoutc,
                            &mut ichan.sbf,
                        );
                        // Generate navigation message
                        ichan.generate_nav_msg(grx, true);
                        // Initialize pseudorange
                        let rho = compute_range(&eph[sv], ionoutc, grx, xyz);
                        ichan.rho0 = rho;
                        // Initialize carrier phase
                        // r_xyz = rho.range;
                        // below line does nothing
                        // let _rho =
                        //     compute_range(&eph[sv], ionoutc, grx, &ref_0);
                        // r_ref = rho.range;
                        phase_ini = 0.0; // TODO: Must initialize properly
                        //phase_ini = (2.0*r_ref - r_xyz)/LAMBDA_L1;
                        // #ifdef FLOAT_CARR_PHASE
                        //                         ichan.carr_phase =
                        // phase_ini - floor(phase_ini);
                        // #else
                        phase_ini -= phase_ini.floor();
                        ichan.carr_phase = (512.0 * 65536.0 * phase_ini) as u32;
                        break;
                    }
                    channel_index = i + 1;
                }
                // Set satellite allocation channel
                if channel_index < MAX_CHAN {
                    allocated_sat[sv] = channel_index as i32;
                }
            }
        } else if allocated_sat[sv] >= 0 {
            // Not visible but allocated
            // Clear channel
            chan[allocated_sat[sv] as usize].prn = 0;
            // Clear satellite allocation flag
            allocated_sat[sv] = -1;
        }
    }
    nsat
}
