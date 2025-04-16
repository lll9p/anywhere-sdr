use crate::{
    channel::Channel,
    constants::*,
    datetime::{GpsTime, TimeRange},
    delay::ionospheric_delay,
    eph::Ephemeris,
    ionoutc::IonoUtc,
};
pub fn sub_vect(y: &mut [f64; 3], x1: &[f64; 3], x2: &[f64; 3]) {
    y[0] = x1[0] - x2[0];
    y[1] = x1[1] - x2[1];
    y[2] = x1[2] - x2[2];
}

pub fn norm_vect(x: &[f64; 3]) -> f64 {
    (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt()
}

pub fn dot_prod(x1: &[f64; 3], x2: &[f64; 3]) -> f64 {
    x1[0] * x2[0] + x1[1] * x2[1] + x1[2] * x2[2]
}
///  Convert Earth-centered Earth-fixed (ECEF) into Lat/Long/Height
///  \param[in] xyz Input Array of X, Y and Z ECEF coordinates
///  \param[out] llh Output Array of Latitude, Longitude and Height
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
pub fn llh2xyz(llh: &[f64; 3], xyz_0: &mut [f64; 3]) {
    let a = WGS84_RADIUS;
    let e = WGS84_ECCENTRICITY;
    let e2 = e * e;
    let clat = llh[0].cos();
    let slat = llh[0].sin();
    let clon = llh[1].cos();
    let slon = llh[1].sin();
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
pub fn ecef2neu(xyz_0: &[f64; 3], t: &[[f64; 3]; 3], neu: &mut [f64; 3]) {
    neu[0] = t[0][0] * xyz_0[0] + t[0][1] * xyz_0[1] + t[0][2] * xyz_0[2];
    neu[1] = t[1][0] * xyz_0[0] + t[1][1] * xyz_0[1] + t[1][2] * xyz_0[2];
    neu[2] = t[2][0] * xyz_0[0] + t[2][1] * xyz_0[1] + t[2][2] * xyz_0[2];
}

///  \brief Convert North-East-Up to Azimuth + Elevation
/// \param[in] neu Input position in North-East-Up format
/// \param[out] azel Output array of azimuth + elevation as double
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
pub fn codegen(ca: &mut [i32; CA_SEQ_LEN], prn: i32) {
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
    let mut j = CA_SEQ_LEN - delay[(prn - 1) as usize];
    for i in 0..CA_SEQ_LEN {
        ca[i] = (1 - g1[i] * g2[j % CA_SEQ_LEN]) / 2;
        j += 1;
    }
}

/// \brief Compute Subframe from Ephemeris
/// \param[in] eph Ephemeris of given SV
/// \param[out] sbf Array of five sub-frames, 10 long words each
#[allow(clippy::too_many_lines)]
pub fn eph2sbf(
    eph: &Ephemeris, ionoutc: &IonoUtc, sbf: &mut [[u32; N_DWRD_SBF]; 5],
) {
    let ura = 0;
    let data_id = 1;
    let sbf4_page25_sv_id = 63;
    let sbf5_page25_sv_id = 51;
    let wnlsf;
    let dtlsf;
    let dn;
    let sbf4_page18_sv_id = 56;

    // FIXED: This has to be the "transmission" week number, not for the
    // ephemeris reference time wn = (unsigned long)(eph.toe.week%1024);
    let wn = 0;
    let toe = (eph.toe.sec / 16.0) as u32;
    let toc = (eph.toc.sec / 16.0) as u32;
    let iode = eph.iode as u32;
    let iodc = eph.iodc as u32;
    let deltan = (eph.deltan / POW2_M43 / PI) as i32;
    let cuc = (eph.cuc / POW2_M29) as i32;
    let cus = (eph.cus / POW2_M29) as i32;
    let cic = (eph.cic / POW2_M29) as i32;
    let cis = (eph.cis / POW2_M29) as i32;
    let crc = (eph.crc / POW2_M5) as i32;
    let crs = (eph.crs / POW2_M5) as i32;
    let ecc = (eph.ecc / POW2_M33) as u32;
    let sqrta = (eph.sqrta / POW2_M19) as u32;
    let m0 = (eph.m0 / POW2_M31 / PI) as i32;
    let omg0 = (eph.omg0 / POW2_M31 / PI) as i32;
    let inc0 = (eph.inc0 / POW2_M31 / PI) as i32;
    let aop = (eph.aop / POW2_M31 / PI) as i32;
    let omgdot = (eph.omgdot / POW2_M43 / PI) as i32;
    let idot = (eph.idot / POW2_M43 / PI) as i32;
    let af0 = (eph.af0 / POW2_M31) as i32;
    let af1 = (eph.af1 / POW2_M43) as i32;
    let af2 = (eph.af2 / POW2_M55) as i32;
    let tgd = (eph.tgd / POW2_M31) as i32;
    let svhlth = eph.svhlth as u32 as i32;

    #[allow(non_snake_case)]
    let codeL2 = eph.codeL2 as u32 as i32;
    let wna = (eph.toe.week % 256) as u32;
    let toa = (eph.toe.sec / 4096.0) as u32;
    let alpha0 = (ionoutc.alpha0 / POW2_M30).round() as i32;
    let alpha1 = (ionoutc.alpha1 / POW2_M27).round() as i32;
    let alpha2 = (ionoutc.alpha2 / POW2_M24).round() as i32;
    let alpha3 = (ionoutc.alpha3 / POW2_M24).round() as i32;
    let beta0 = (ionoutc.beta0 / 2048.0).round() as i32;
    let beta1 = (ionoutc.beta1 / 16384.0).round() as i32;
    let beta2 = (ionoutc.beta2 / 65536.0).round() as i32;
    let beta3 = (ionoutc.beta3 / 65536.0).round() as i32;

    #[allow(non_snake_case)]
    let A0 = (ionoutc.A0 / POW2_M30).round() as i32;

    #[allow(non_snake_case)]
    let A1 = (ionoutc.A1 / POW2_M50).round() as i32;
    let dtls = ionoutc.dtls;
    let tot = (ionoutc.tot / 4096) as u32;
    let wnt = (ionoutc.wnt % 256) as u32;
    // 2016/12/31 (Sat) -> WNlsf = 1929, DN = 7 (http://navigationservices.agi.com/GNSSWeb/)
    // Days are counted from 1 to 7 (Sunday is 1).
    if ionoutc.leapen == 1 {
        wnlsf = (ionoutc.wnlsf % 256) as u32;
        dn = ionoutc.dn as u32;
        dtlsf = ionoutc.dtlsf as u32;
    } else {
        wnlsf = (1929 % 256) as u32;
        dn = 7;
        dtlsf = 18;
    }
    // Subframe 1
    (sbf[0])[0] = 0x008b_0000 << 6;
    (sbf[0])[1] = 0x1 << 8;
    (sbf[0])[2] = (wn & 0x3ff) << 20
        | (codeL2 as u32 & 0x3) << 18
        | (ura & 0xf) << 14
        | (svhlth as u32 & 0x3f) << 8
        | (iodc >> 8 & 0x3) << 6;
    (sbf[0])[3] = 0;
    (sbf[0])[4] = 0;
    (sbf[0])[5] = 0;
    (sbf[0])[6] = (tgd as u32 & 0xff) << 6;
    (sbf[0])[7] = (iodc & 0xff) << 22 | (toc & 0xffff) << 6;
    (sbf[0])[8] = (af2 as u32 & 0xff) << 22 | (af1 as u32 & 0xffff) << 6;
    (sbf[0])[9] = (af0 as u32 & 0x003f_ffff) << 8;
    // Subframe 2
    (sbf[1])[0] = 0x008b_0000 << 6;
    (sbf[1])[1] = 0x2 << 8;
    (sbf[1])[2] = (iode & 0xff) << 22 | (crs as u32 & 0xffff) << 6;
    (sbf[1])[3] =
        (deltan as u32 & 0xffff) << 14 | ((m0 >> 24) as u32 & 0xff) << 6;
    (sbf[1])[4] = (m0 as u32 & 0x00ff_ffff) << 6;
    (sbf[1])[5] = (cuc as u32 & 0xffff) << 14 | (ecc >> 24 & 0xff) << 6;
    (sbf[1])[6] = (ecc & 0x00ff_ffff) << 6;
    (sbf[1])[7] = (cus as u32 & 0xffff) << 14 | (sqrta >> 24 & 0xff) << 6;
    (sbf[1])[8] = (sqrta & 0x00ff_ffff) << 6;
    (sbf[1])[9] = (toe & 0xffff) << 14;
    // Subframe 3
    (sbf[2])[0] = 0x008b_0000 << 6;
    (sbf[2])[1] = 0x3 << 8;
    (sbf[2])[2] =
        (cic as u32 & 0xffff) << 14 | ((omg0 >> 24) as u32 & 0xff) << 6;
    (sbf[2])[3] = (omg0 as u32 & 0x00ff_ffff) << 6;
    (sbf[2])[4] =
        (cis as u32 & 0xffff) << 14 | ((inc0 >> 24) as u32 & 0xff) << 6;
    (sbf[2])[5] = (inc0 as u32 & 0x00ff_ffff) << 6;
    (sbf[2])[6] =
        (crc as u32 & 0xffff) << 14 | ((aop >> 24) as u32 & 0xff) << 6;
    (sbf[2])[7] = (aop as u32 & 0x00ff_ffff) << 6;
    (sbf[2])[8] = (omgdot as u32 & 0x00ff_ffff) << 6;
    (sbf[2])[9] = (iode & 0xff) << 22 | (idot as u32 & 0x3fff) << 8;
    if ionoutc.vflg {
        // Subframe 4, page 18
        (sbf[3])[0] = 0x008b_0000 << 6;
        (sbf[3])[1] = 0x4 << 8;
        (sbf[3])[2] = data_id << 28
            | sbf4_page18_sv_id << 22
            | (alpha0 as u32 & 0xff) << 14
            | (alpha1 as u32 & 0xff) << 6;
        (sbf[3])[3] = (alpha2 as u32 & 0xff) << 22
            | (alpha3 as u32 & 0xff) << 14
            | (beta0 as u32 & 0xff) << 6;
        (sbf[3])[4] = (beta1 as u32 & 0xff) << 22
            | (beta2 as u32 & 0xff) << 14
            | (beta3 as u32 & 0xff) << 6;
        (sbf[3])[5] = (A1 as u32 & 0x00ff_ffff) << 6;
        (sbf[3])[6] = ((A0 >> 8) as u32 & 0x00ff_ffff) << 6;
        (sbf[3])[7] =
            (A0 as u32 & 0xff) << 22 | (tot & 0xff) << 14 | (wnt & 0xff) << 6;
        (sbf[3])[8] = (dtls as u32 & 0xff) << 22
            | (wnlsf & 0xff) << 14
            | (dn & 0xff) << 6;
        (sbf[3])[9] = (dtlsf & 0xff) << 22;
    } else {
        // Subframe 4, page 25
        (sbf[3])[0] = 0x008b_0000 << 6;
        (sbf[3])[1] = 0x4 << 8;
        (sbf[3])[2] = data_id << 28 | sbf4_page25_sv_id << 22;
        (sbf[3])[3] = 0;
        (sbf[3])[4] = 0;
        (sbf[3])[5] = 0;
        (sbf[3])[6] = 0;
        (sbf[3])[7] = 0;
        (sbf[3])[8] = 0;
        (sbf[3])[9] = 0;
    }
    // Subframe 5, page 25
    (sbf[4])[0] = 0x008b_0000 << 6;
    (sbf[4])[1] = 0x5 << 8;
    (sbf[4])[2] = data_id << 28
        | sbf5_page25_sv_id << 22
        | (toa & 0xff) << 14
        | (wna & 0xff) << 6;
    (sbf[4])[3] = 0;
    (sbf[4])[4] = 0;
    (sbf[4])[5] = 0;
    (sbf[4])[6] = 0;
    (sbf[4])[7] = 0;
    (sbf[4])[8] = 0;
    (sbf[4])[9] = 0;
}

///  \brief Compute range between a satellite and the receiver
///  \param[out] rho The computed range
///  \param[in] eph Ephemeris data of the satellite
///  \param[in] g GPS time at time of receiving the signal
///  \param[in] xyz position of the receiver
pub fn compute_range(
    rho: &mut TimeRange, eph: &Ephemeris, ionoutc: &mut IonoUtc, g: &GpsTime,
    xyz_0: &[f64; 3],
) {
    let mut los: [f64; 3] = [0.; 3];
    let mut llh: [f64; 3] = [0.; 3];
    let mut neu: [f64; 3] = [0.; 3];
    let mut tmat: [[f64; 3]; 3] = [[0.; 3]; 3];
    // SV position at time of the pseudorange observation.
    let (mut pos, vel, clk) = eph.satpos(g);
    // Receiver to satellite vector and light-time.
    sub_vect(&mut los, &pos, xyz_0);
    let tau = norm_vect(&los) / SPEED_OF_LIGHT;
    // Extrapolate the satellite position backwards to the transmission time.
    pos[0] -= vel[0] * tau;
    pos[1] -= vel[1] * tau;
    pos[2] -= vel[2] * tau;
    let xrot = pos[0] + pos[1] * OMEGA_EARTH * tau;
    let yrot = pos[1] - pos[0] * OMEGA_EARTH * tau;
    pos[0] = xrot;
    pos[1] = yrot;
    // New observer to satellite vector and satellite range.
    sub_vect(&mut los, &pos, xyz_0);
    let range = norm_vect(&los);
    rho.d = range;
    // Pseudorange.
    rho.range = range - SPEED_OF_LIGHT * clk[0];
    // Relative velocity of SV and receiver.
    let rate = dot_prod(&vel, &los) / range;
    // Pseudorange rate.
    rho.rate = rate; // - SPEED_OF_LIGHT*clk[1];
    // Time of application.
    rho.g = *g;
    // Azimuth and elevation angles.
    xyz2llh(xyz_0, &mut llh);
    ltcmat(&llh, &mut tmat);
    ecef2neu(&los, &tmat, &mut neu);
    neu2azel(&mut rho.azel, &neu);
    // Add ionospheric delay
    rho.iono_delay = ionospheric_delay(ionoutc, g, &llh, &rho.azel);
    rho.range += rho.iono_delay;
}

pub fn check_sat_visibility(
    eph: &Ephemeris, g: &GpsTime, xyz_0: &[f64; 3], elv_mask: f64,
    azel: &mut [f64; 2],
) -> i32 {
    let mut llh: [f64; 3] = [0.; 3];
    let mut neu: [f64; 3] = [0.; 3];
    // let mut pos: [f64; 3] = [0.; 3];
    // let mut vel: [f64; 3] = [0.; 3];
    // modified from [f64;3] to [f64;2]
    // let mut clk: [f64; 2] = [0.; 2];
    let mut los: [f64; 3] = [0.; 3];
    let mut tmat: [[f64; 3]; 3] = [[0.; 3]; 3];
    if !eph.vflg {
        return -1; // Invalid
    }
    xyz2llh(xyz_0, &mut llh);
    ltcmat(&llh, &mut tmat);
    let (pos, _vel, _clk) = eph.satpos(g);
    sub_vect(&mut los, &pos, xyz_0);
    ecef2neu(&los, &tmat, &mut neu);
    neu2azel(azel, &neu);
    if azel[1] * R2D > elv_mask {
        return 1; // Visible
    }
    0 // Invisible
}

pub fn allocate_channel(
    chan: &mut [Channel; MAX_CHAN], eph: &mut [Ephemeris; MAX_SAT],
    ionoutc: &mut IonoUtc, grx: &GpsTime, xyz_0: &[f64; 3], _elv_mask: f64,
    allocated_sat: &mut [i32; MAX_SAT],
) -> i32 {
    let mut nsat: i32 = 0;
    let mut azel: [f64; 2] = [0.; 2];
    let mut rho: TimeRange = TimeRange {
        g: GpsTime { week: 0, sec: 0. },
        range: 0.,
        rate: 0.,
        d: 0.,
        azel: [0.; 2],
        iono_delay: 0.,
    };
    let ref_0: [f64; 3] = [0., 0., 0.];
    // #[allow(unused_variables)]
    // let mut r_ref: f64 = 0.;
    // #[allow(unused_variables)]
    // let mut r_xyz: f64;
    let mut phase_ini: f64;
    for sv in 0..MAX_SAT {
        if check_sat_visibility(&eph[sv], grx, xyz_0, 0.0, &mut azel) == 1 {
            nsat += 1; // Number of visible satellites
            if allocated_sat[sv] == -1 {
                // Visible but not allocated
                //
                // Allocated new satellite
                let mut i = 0;
                while i < MAX_CHAN {
                    if chan[i].prn == 0 {
                        // Initialize channel
                        chan[i].prn = sv as i32 + 1;
                        chan[i].azel[0] = azel[0];
                        chan[i].azel[1] = azel[1];
                        // C/A code generation
                        codegen(&mut chan[i].ca, chan[i].prn);
                        // Generate subframe
                        eph2sbf(&eph[sv], ionoutc, &mut chan[i].sbf);
                        // Generate navigation message
                        chan[i].generate_nav_msg(grx, true);
                        // Initialize pseudorange
                        compute_range(&mut rho, &eph[sv], ionoutc, grx, xyz_0);
                        chan[i].rho0 = rho;
                        // Initialize carrier phase
                        // r_xyz = rho.range;
                        compute_range(&mut rho, &eph[sv], ionoutc, grx, &ref_0);
                        // r_ref = rho.range;
                        phase_ini = 0.0; // TODO: Must initialize properly
                        //phase_ini = (2.0*r_ref - r_xyz)/LAMBDA_L1;
                        // #ifdef FLOAT_CARR_PHASE
                        //                         chan[i].carr_phase =
                        // phase_ini - floor(phase_ini);
                        // #else
                        phase_ini -= phase_ini.floor();
                        chan[i].carr_phase =
                            (512.0 * 65536.0 * phase_ini) as u32;
                        break;
                    }
                    i += 1;
                }
                // Set satellite allocation channel
                if i < MAX_CHAN {
                    allocated_sat[sv] = i as i32;
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
