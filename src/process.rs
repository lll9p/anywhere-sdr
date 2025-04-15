use crate::{
    cli::Params,
    constants::*,
    datetime::{datetime_t, gpstime_t},
    eph::ephem_t,
    ionoutc::ionoutc_t,
    read_nmea_gga::read_Nmea_GGA,
    read_rinex::read_rinex_nav_all,
    read_user_motion::{read_user_motion, read_user_motion_LLH},
    table::{ANT_PAT_DB, COS_TABLE512, SIN_TABLE512},
};
use std::{io::Write, time::Instant};

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct range_t {
    pub g: gpstime_t,
    // pseudorange
    pub range: f64,
    pub rate: f64,
    // geometric distance
    pub d: f64,
    pub azel: [f64; 2],
    pub iono_delay: f64,
}

//  Structure representing a Channel
#[allow(non_snake_case)]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct channel_t {
    //< PRN Number
    pub prn: i32,
    //< C/A Sequence
    pub ca: [i32; CA_SEQ_LEN],
    //< Carrier frequency
    pub f_carr: f64,
    //< Code frequency
    pub f_code: f64,
    /* #ifdef FLOAT_CARR_PHASE
        double carr_phase;
    #endif */
    //< Carrier phase
    pub carr_phase: u32,
    //< Carrier phasestep
    pub carr_phasestep: i32,
    //< Code phase
    pub code_phase: f64,
    // < GPS time at start
    pub g0: gpstime_t,
    // < current subframe
    pub sbf: [[u32; N_DWRD_SBF]; 5],
    // < Data words of sub-frame
    pub dwrd: [u32; N_DWRD],
    // < initial word
    pub iword: i32,
    // < initial bit
    pub ibit: i32,
    // < initial code
    pub icode: i32,
    // < current data bit
    pub dataBit: i32,
    // < current C/A code
    pub codeCA: i32,
    pub azel: [f64; 2],
    pub rho0: range_t,
}

pub fn subVect(y: &mut [f64; 3], x1: &[f64; 3], x2: &[f64; 3]) {
    y[0] = x1[0] - x2[0];
    y[1] = x1[1] - x2[1];
    y[2] = x1[2] - x2[2];
}

pub fn normVect(x: &[f64; 3]) -> f64 {
    (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt()
}

pub fn dotProd(x1: &[f64; 3], x2: &[f64; 3]) -> f64 {
    x1[0] * x2[0] + x1[1] * x2[1] + x1[2] * x2[2]
}
/// !generate the C/A code sequence for a given Satellite Vehicle PRN
///  \param[in] prn PRN number of the Satellite Vehicle
///  \param[out] ca Caller-allocated integer array of 1023 bytes
pub fn codegen(ca: &mut [i32; CA_SEQ_LEN], prn: i32) {
    let delay: [usize; 32] = [
        5, 6, 7, 8, 17, 18, 139, 140, 141, 251, 252, 254, 255, 256, 257, 258, 469, 470, 471, 472,
        473, 474, 509, 512, 513, 514, 515, 516, 859, 860, 861, 862,
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
        r2[i] = -1_i32;
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
        ca[i] = (1_i32 - g1[i] * g2[j % CA_SEQ_LEN]) / 2_i32;
        j += 1;
    }
}

//  Convert a UTC date into a GPS date
pub fn date2gps(t: &datetime_t, g: &mut gpstime_t) {
    let doy: [i32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let ye = (t).y - 1980_i32;

    // Compute the number of leap days since Jan 5/Jan 6, 1980.
    let mut lpdays = ye / 4_i32 + 1_i32;
    if ye % 4_i32 == 0_i32 && (t).m <= 2_i32 {
        lpdays -= 1;
    }

    // Compute the number of days elapsed since Jan 5/Jan 6, 1980.
    let de = ye * 365_i32 + doy[((t).m - 1_i32) as usize] + (t).d + lpdays - 6_i32;

    // Convert time to GPS weeks and seconds.
    (g).week = de / 7_i32;
    (g).sec = (de % 7_i32) as f64 * SECONDS_IN_DAY
        + (t).hh as f64 * SECONDS_IN_HOUR
        + (t).mm as f64 * SECONDS_IN_MINUTE
        + (t).sec;
}

// Convert Julian day number to calendar date
pub fn gps2date(g: &gpstime_t, t: &mut datetime_t) {
    let c: i32 = ((7_i32 * (g).week) as f64 + ((g).sec / 86400.0f64).floor() + 2444245.0f64) as i32
        + 1537_i32;
    let d: i32 = ((c as f64 - 122.1f64) / 365.25f64) as i32;
    let e: i32 = 365_i32 * d + d / 4_i32;
    let f: i32 = ((c - e) as f64 / 30.6001f64) as i32;
    (t).d = c - e - (30.6001f64 * f as f64) as i32;
    (t).m = f - 1_i32 - 12_i32 * (f / 14_i32);
    (t).y = d - 4715_i32 - (7_i32 + (t).m) / 10_i32;
    (t).hh = ((g).sec / 3600.0f64) as i32 % 24_i32;
    (t).mm = ((g).sec / 60.0f64) as i32 % 60_i32;
    (t).sec = g.sec - 60.0f64 * ((g).sec / 60.0f64).floor();
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
    let eps = 1.0e-3f64;
    let e2 = e * e;
    if normVect(xyz_0) < eps {
        // Invalid ECEF vector
        llh[0] = 0.0f64;
        llh[1] = 0.0f64;
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
        n = a / (1.0f64 - e2 * slat * slat).sqrt();
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
    let clat = (llh[0]).cos();
    let slat = (llh[0]).sin();
    let clon = (llh[1]).cos();
    let slon = (llh[1]).sin();
    let d = e * slat;
    let n = a / (1.0f64 - d * d).sqrt();
    let nph = n + llh[2];
    let tmp = nph * clat;
    xyz_0[0] = tmp * clon;
    xyz_0[1] = tmp * slon;
    xyz_0[2] = ((1.0f64 - e2) * n + llh[2]) * slat;
}

///  \brief Compute the intermediate matrix for LLH to ECEF
///  \param[in] llh Input position in Latitude-Longitude-Height format
///  \param[out] t Three-by-Three output matrix
pub fn ltcmat(llh: &[f64; 3], t: &mut [[f64; 3]; 3]) {
    let slat = (llh[0]).sin();
    let clat = (llh[0]).cos();
    let slon = (llh[1]).sin();
    let clon = (llh[1]).cos();
    t[0][0] = -slat * clon;
    t[0][1] = -slat * slon;
    t[0][2] = clat;
    t[1][0] = -slon;
    t[1][1] = clon;
    t[1][2] = 0.0f64;
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
///
pub fn neu2azel(azel: &mut [f64; 2], neu: &[f64; 3]) {
    azel[0] = neu[1].atan2(neu[0]);
    if azel[0] < 0.0f64 {
        azel[0] += 2.0f64 * PI;
    }
    let ne = (neu[0] * neu[0] + neu[1] * neu[1]).sqrt();
    azel[1] = neu[2].atan2(ne);
}

/// \brief Compute Satellite position, velocity and clock at given time
///
/// Computing Satellite Velocity using the Broadcast Ephemeris
/// http://www.ngs.noaa.gov/gps-toolbox/bc_velo.htm
/// \param[in] eph Ephemeris data of the satellite
/// \param[in] g GPS time at which position is to be computed
/// \param[out] pos Computed position (vector)
/// \param[out] vel Computed velocity (vector)
/// \param[clk] clk Computed clock
///
pub fn satpos(
    eph: &ephem_t,
    g: &gpstime_t,
    pos: &mut [f64; 3],
    vel: &mut [f64; 3],
    clk: &mut [f64; 2],
) {
    let mut tk = g.sec - eph.toe.sec;
    if tk > SECONDS_IN_HALF_WEEK {
        tk -= SECONDS_IN_WEEK;
    } else if tk < -SECONDS_IN_HALF_WEEK {
        tk += SECONDS_IN_WEEK;
    }
    let mk = eph.m0 + eph.n * tk;
    let mut ek = mk;
    let mut ekold = ek + 1.0f64;
    let mut OneMinusecosE = 0_i32 as f64; // Suppress the uninitialized warning.
    while (ek - ekold).abs() > 1.0E-14f64 {
        ekold = ek;
        OneMinusecosE = 1.0f64 - eph.ecc * (ekold).cos();
        ek += (mk - ekold + eph.ecc * (ekold.sin())) / OneMinusecosE;
    }
    let sek = (ek).sin();
    let cek = (ek).cos();
    let ekdot = eph.n / OneMinusecosE;
    let relativistic = -4.442807633E-10f64 * eph.ecc * eph.sqrta * sek;
    let pk = (eph.sq1e2 * sek).atan2(cek - eph.ecc) + eph.aop;
    let pkdot = eph.sq1e2 * ekdot / OneMinusecosE;
    let s2pk = (2.0f64 * pk).sin();
    let c2pk = (2.0f64 * pk).cos();
    let uk = pk + eph.cus * s2pk + eph.cuc * c2pk;
    let suk = (uk).sin();
    let cuk = (uk).cos();
    let ukdot = pkdot * (1.0f64 + 2.0f64 * (eph.cus * c2pk - eph.cuc * s2pk));
    let rk = eph.A * OneMinusecosE + eph.crc * c2pk + eph.crs * s2pk;
    let rkdot = eph.A * eph.ecc * sek * ekdot + 2.0f64 * pkdot * (eph.crs * c2pk - eph.crc * s2pk);
    let ik = eph.inc0 + eph.idot * tk + eph.cic * c2pk + eph.cis * s2pk;
    let sik = (ik).sin();
    let cik = (ik).cos();
    let ikdot = eph.idot + 2.0f64 * pkdot * (eph.cis * c2pk - eph.cic * s2pk);
    let xpk = rk * cuk;
    let ypk = rk * suk;
    let xpkdot = rkdot * cuk - ypk * ukdot;
    let ypkdot = rkdot * suk + xpk * ukdot;
    let ok = eph.omg0 + tk * eph.omgkdot - OMEGA_EARTH * eph.toe.sec;
    let sok = (ok).sin();
    let cok = (ok).cos();
    pos[0] = xpk * cok - ypk * cik * sok;
    pos[1] = xpk * sok + ypk * cik * cok;
    pos[2] = ypk * sik;
    let tmp = ypkdot * cik - ypk * sik * ikdot;
    vel[0] = -eph.omgkdot * pos[1] + xpkdot * cok - tmp * sok;
    vel[1] = eph.omgkdot * pos[0] + xpkdot * sok + tmp * cok;
    vel[2] = ypk * cik * ikdot + ypkdot * sik;
    let mut tk = g.sec - eph.toc.sec;
    if tk > SECONDS_IN_HALF_WEEK {
        tk -= SECONDS_IN_WEEK;
    } else if tk < -SECONDS_IN_HALF_WEEK {
        tk += SECONDS_IN_WEEK;
    }
    clk[0] = eph.af0 + tk * (eph.af1 + tk * eph.af2) + relativistic - eph.tgd;
    clk[1] = eph.af1 + 2.0f64 * tk * eph.af2;
}

/// \brief Compute Subframe from Ephemeris
/// \param[in] eph Ephemeris of given SV
/// \param[out] sbf Array of five sub-frames, 10 long words each
///
pub fn eph2sbf(eph: ephem_t, ionoutc: &ionoutc_t, sbf: &mut [[u32; N_DWRD_SBF]; 5]) {
    let ura: u32 = 0_u32;
    let dataId: u32 = 1_u32;
    let sbf4_page25_svId: u32 = 63_u32;
    let sbf5_page25_svId: u32 = 51_u32;
    let wnlsf: u32;
    let dtlsf: u32;
    let dn: u32;
    let sbf4_page18_svId: u32 = 56_u32;

    // FIXED: This has to be the "transmission" week number, not for the ephemeris reference time
    //wn = (unsigned long)(eph.toe.week%1024);
    let wn = 0_u32;
    let toe = (eph.toe.sec / 16.0f64) as u32;
    let toc = (eph.toc.sec / 16.0f64) as u32;
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
    let codeL2 = eph.codeL2 as u32 as i32;
    let wna = (eph.toe.week % 256_i32) as u32;
    let toa = (eph.toe.sec / 4096.0f64) as u32;
    let alpha0 = (ionoutc.alpha0 / POW2_M30).round() as i32;
    let alpha1 = (ionoutc.alpha1 / POW2_M27).round() as i32;
    let alpha2 = (ionoutc.alpha2 / POW2_M24).round() as i32;
    let alpha3 = (ionoutc.alpha3 / POW2_M24).round() as i32;
    let beta0 = (ionoutc.beta0 / 2048.0f64).round() as i32;
    let beta1 = (ionoutc.beta1 / 16384.0f64).round() as i32;
    let beta2 = (ionoutc.beta2 / 65536.0f64).round() as i32;
    let beta3 = (ionoutc.beta3 / 65536.0f64).round() as i32;
    let A0 = (ionoutc.A0 / POW2_M30).round() as i32;
    let A1 = (ionoutc.A1 / POW2_M50).round() as i32;
    let dtls = ionoutc.dtls;
    let tot = (ionoutc.tot / 4096_i32) as u32;
    let wnt = (ionoutc.wnt % 256_i32) as u32;
    // 2016/12/31 (Sat) -> WNlsf = 1929, DN = 7 (http://navigationservices.agi.com/GNSSWeb/)
    // Days are counted from 1 to 7 (Sunday is 1).
    if ionoutc.leapen == 1_i32 {
        wnlsf = (ionoutc.wnlsf % 256_i32) as u32;
        dn = ionoutc.dn as u32;
        dtlsf = ionoutc.dtlsf as u32;
    } else {
        wnlsf = (1929_i32 % 256_i32) as u32;
        dn = 7_i32 as u32;
        dtlsf = 18_i32 as u32;
    }
    // Subframe 1
    (sbf[0])[0] = 0x8b0000_u32 << 6_i32;
    (sbf[0])[1] = 0x1_u32 << 8_i32;
    (sbf[0])[2] = (wn & 0x3ff_u32) << 20_i32
        | (codeL2 as u32 & 0x3_u32) << 18_i32
        | (ura & 0xf_u32) << 14_i32
        | (svhlth as u32 & 0x3f_u32) << 8_i32
        | (iodc >> 8_i32 & 0x3_u32) << 6_i32;
    (sbf[0])[3] = 0_u32;
    (sbf[0])[4] = 0_u32;
    (sbf[0])[5] = 0_u32;
    (sbf[0])[6] = (tgd as u32 & 0xff_u32) << 6_i32;
    (sbf[0])[7] = (iodc & 0xff_u32) << 22_i32 | (toc & 0xffff_u32) << 6_i32;
    (sbf[0])[8] = (af2 as u32 & 0xff_u32) << 22_i32 | (af1 as u32 & 0xffff_u32) << 6_i32;
    (sbf[0])[9] = (af0 as u32 & 0x3fffff_u32) << 8_i32;
    // Subframe 2
    (sbf[1])[0] = 0x8b0000_u32 << 6_i32;
    (sbf[1])[1] = 0x2_u32 << 8_i32;
    (sbf[1])[2] = (iode & 0xff_u32) << 22_i32 | (crs as u32 & 0xffff_u32) << 6_i32;
    (sbf[1])[3] =
        (deltan as u32 & 0xffff_u32) << 14_i32 | ((m0 >> 24_i32) as u32 & 0xff_u32) << 6_i32;
    (sbf[1])[4] = (m0 as u32 & 0xffffff_u32) << 6_i32;
    (sbf[1])[5] = (cuc as u32 & 0xffff_u32) << 14_i32 | (ecc >> 24_i32 & 0xff_u32) << 6_i32;
    (sbf[1])[6] = (ecc & 0xffffff_u32) << 6_i32;
    (sbf[1])[7] = (cus as u32 & 0xffff_u32) << 14_i32 | (sqrta >> 24_i32 & 0xff_u32) << 6_i32;
    (sbf[1])[8] = (sqrta & 0xffffff_u32) << 6_i32;
    (sbf[1])[9] = (toe & 0xffff_u32) << 14_i32;
    // Subframe 3
    (sbf[2])[0] = 0x8b0000_u32 << 6_i32;
    (sbf[2])[1] = 0x3_u32 << 8_i32;
    (sbf[2])[2] =
        (cic as u32 & 0xffff_u32) << 14_i32 | ((omg0 >> 24_i32) as u32 & 0xff_u32) << 6_i32;
    (sbf[2])[3] = (omg0 as u32 & 0xffffff_u32) << 6_i32;
    (sbf[2])[4] =
        (cis as u32 & 0xffff_u32) << 14_i32 | ((inc0 >> 24_i32) as u32 & 0xff_u32) << 6_i32;
    (sbf[2])[5] = (inc0 as u32 & 0xffffff_u32) << 6_i32;
    (sbf[2])[6] =
        (crc as u32 & 0xffff_u32) << 14_i32 | ((aop >> 24_i32) as u32 & 0xff_u32) << 6_i32;
    (sbf[2])[7] = (aop as u32 & 0xffffff_u32) << 6_i32;
    (sbf[2])[8] = (omgdot as u32 & 0xffffff_u32) << 6_i32;
    (sbf[2])[9] = (iode & 0xff_u32) << 22_i32 | (idot as u32 & 0x3fff_u32) << 8_i32;
    if ionoutc.vflg == 1_i32 {
        // Subframe 4, page 18
        (sbf[3])[0] = 0x8b0000_u32 << 6_i32;
        (sbf[3])[1] = 0x4_u32 << 8_i32;
        (sbf[3])[2] = dataId << 28_i32
            | sbf4_page18_svId << 22_i32
            | (alpha0 as u32 & 0xff_u32) << 14_i32
            | (alpha1 as u32 & 0xff_u32) << 6_i32;
        (sbf[3])[3] = (alpha2 as u32 & 0xff_u32) << 22_i32
            | (alpha3 as u32 & 0xff_u32) << 14_i32
            | (beta0 as u32 & 0xff_u32) << 6_i32;
        (sbf[3])[4] = (beta1 as u32 & 0xff_u32) << 22_i32
            | (beta2 as u32 & 0xff_u32) << 14_i32
            | (beta3 as u32 & 0xff_u32) << 6_i32;
        (sbf[3])[5] = (A1 as u32 & 0xffffff_u32) << 6_i32;
        (sbf[3])[6] = ((A0 >> 8_i32) as u32 & 0xffffff_u32) << 6_i32;
        (sbf[3])[7] = (A0 as u32 & 0xff_u32) << 22_i32
            | (tot & 0xff_u32) << 14_i32
            | (wnt & 0xff_u32) << 6_i32;
        (sbf[3])[8] = (dtls as u32 & 0xff_u32) << 22_i32
            | (wnlsf & 0xff_u32) << 14_i32
            | (dn & 0xff_u32) << 6_i32;
        (sbf[3])[9] = (dtlsf & 0xff_u32) << 22_i32;
    } else {
        // Subframe 4, page 25
        (sbf[3])[0] = 0x8b0000_u32 << 6_i32;
        (sbf[3])[1] = 0x4_u32 << 8_i32;
        (sbf[3])[2] = dataId << 28_i32 | sbf4_page25_svId << 22_i32;
        (sbf[3])[3] = 0_u32;
        (sbf[3])[4] = 0_u32;
        (sbf[3])[5] = 0_u32;
        (sbf[3])[6] = 0_u32;
        (sbf[3])[7] = 0_u32;
        (sbf[3])[8] = 0_u32;
        (sbf[3])[9] = 0_u32;
    }
    // Subframe 5, page 25
    (sbf[4])[0] = 0x8b0000_u32 << 6_i32;
    (sbf[4])[1] = 0x5_u32 << 8_i32;
    (sbf[4])[2] = dataId << 28_i32
        | sbf5_page25_svId << 22_i32
        | (toa & 0xff_u32) << 14_i32
        | (wna & 0xff_u32) << 6_i32;
    (sbf[4])[3] = 0_u32;
    (sbf[4])[4] = 0_u32;
    (sbf[4])[5] = 0_u32;
    (sbf[4])[6] = 0_u32;
    (sbf[4])[7] = 0_u32;
    (sbf[4])[8] = 0_u32;
    (sbf[4])[9] = 0_u32;
}

/// \brief Count number of bits set to 1
/// \param[in] v long word in which bits are counted
/// \returns Count of bits set to 1
pub fn countBits(v: u32) -> u32 {
    let S: [i32; 5] = [1_i32, 2_i32, 4_i32, 8_i32, 16_i32];
    let B: [u32; 5] = [
        0x55555555_i32 as u32,
        0x33333333_i32 as u32,
        0xf0f0f0f_i32 as u32,
        0xff00ff_i32 as u32,
        0xffff_i32 as u32,
    ];
    let mut c = v;
    c = (c >> S[0] & B[0]).wrapping_add(c & B[0]);
    c = (c >> S[1] & B[1]).wrapping_add(c & B[1]);
    c = (c >> S[2] & B[2]).wrapping_add(c & B[2]);
    c = (c >> S[3] & B[3]).wrapping_add(c & B[3]);
    c = (c >> S[4] & B[4]).wrapping_add(c & B[4]);
    c
}

///  \brief Compute the Checksum for one given word of a subframe
///  \param[in] source The input data
///  \param[in] nib Does this word contain non-information-bearing bits?
///  \returns Computed Checksum
pub fn computeChecksum(source: u32, nib: i32) -> u32 {
    /*
    Bits 31 to 30 = 2 LSBs of the previous transmitted word, D29* and D30*
    Bits 29 to  6 = Source data bits, d1, d2, ..., d24
    Bits  5 to  0 = Empty parity bits
    */

    /*
    Bits 31 to 30 = 2 LSBs of the previous transmitted word, D29* and D30*
    Bits 29 to  6 = Data bits transmitted by the SV, D1, D2, ..., D24
    Bits  5 to  0 = Computed parity bits, D25, D26, ..., D30
    */

    /*
                      1            2           3
    bit    12 3456 7890 1234 5678 9012 3456 7890
    ---    -------------------------------------
    D25    11 1011 0001 1111 0011 0100 1000 0000
    D26    01 1101 1000 1111 1001 1010 0100 0000
    D27    10 1110 1100 0111 1100 1101 0000 0000
    D28    01 0111 0110 0011 1110 0110 1000 0000
    D29    10 1011 1011 0001 1111 0011 0100 0000
    D30    00 1011 0111 1010 1000 1001 1100 0000
    */
    let bmask: [u32; 6] = [
        0x3b1f3480_u32,
        0x1d8f9a40_u32,
        0x2ec7cd00_u32,
        0x1763e680_u32,
        0x2bb1f340_u32,
        0xb7a89c0_u32,
    ];
    let mut D: u32;
    let mut d: u32 = source & 0x3fffffc0_u32;
    let D29: u32 = source >> 31_i32 & 0x1_u32;
    let D30: u32 = source >> 30_i32 & 0x1_u32;
    if nib != 0 {
        // Non-information bearing bits for word 2 and 10
        /*
        Solve bits 23 and 24 to preserve parity check
        with zeros in bits 29 and 30.
        */
        if D30
            .wrapping_add(countBits(bmask[4] & d))
            .wrapping_rem(2_i32 as u32)
            != 0
        {
            d ^= 0x1_u32 << 6_i32;
        }
        if D29
            .wrapping_add(countBits(bmask[5] & d))
            .wrapping_rem(2_i32 as u32)
            != 0
        {
            d ^= 0x1_u32 << 7_i32;
        }
    }
    D = d;
    if D30 != 0 {
        D ^= 0x3fffffc0_u32;
    }
    D |= D29
        .wrapping_add(countBits(bmask[0] & d))
        .wrapping_rem(2_i32 as u32)
        << 5_i32;
    D |= D30
        .wrapping_add(countBits(bmask[1] & d))
        .wrapping_rem(2_i32 as u32)
        << 4_i32;
    D |= D29
        .wrapping_add(countBits(bmask[2] & d))
        .wrapping_rem(2_i32 as u32)
        << 3_i32;
    D |= D30
        .wrapping_add(countBits(bmask[3] & d))
        .wrapping_rem(2_i32 as u32)
        << 2_i32;
    D |= D30
        .wrapping_add(countBits(bmask[4] & d))
        .wrapping_rem(2_i32 as u32)
        << 1_i32;
    D |= D29
        .wrapping_add(countBits(bmask[5] & d))
        .wrapping_rem(2_i32 as u32);
    D &= 0x3fffffff_u32;

    //D |= (source & 0xC0000000UL); // Add D29* and D30* from source data bits
    D
}

pub fn subGpsTime(g1: gpstime_t, g0: gpstime_t) -> f64 {
    let mut dt = g1.sec - g0.sec;
    dt += (g1.week - g0.week) as f64 * SECONDS_IN_WEEK;
    dt
}

pub fn incGpsTime(g0: gpstime_t, dt: f64) -> gpstime_t {
    let mut g1: gpstime_t = gpstime_t { week: 0, sec: 0. };
    g1.week = g0.week;
    g1.sec = g0.sec + dt;
    g1.sec = (g1.sec * 1000.0f64).round() / 1000.0f64; // Avoid rounding error
    while g1.sec >= SECONDS_IN_WEEK {
        g1.sec -= SECONDS_IN_WEEK;
        g1.week += 1;
    }
    while g1.sec < 0.0f64 {
        g1.sec += SECONDS_IN_WEEK;
        g1.week -= 1;
    }
    g1
}

pub fn ionosphericDelay(
    ionoutc: &ionoutc_t,
    g: &gpstime_t,
    llh: &[f64; 3],
    azel: &[f64; 2],
) -> f64 {
    let iono_delay: f64;
    if ionoutc.enable == 0_i32 {
        // No ionospheric delay
        return 0.0f64;
    }
    let E = azel[1] / PI;
    let phi_u = llh[0] / PI;
    let lam_u = llh[1] / PI;
    let F = 1.0f64 + 16.0f64 * (0.53f64 - E).powf(3.0f64);
    if ionoutc.vflg == 0_i32 {
        iono_delay = F * 5.0e-9f64 * SPEED_OF_LIGHT;
    } else {
        let mut PER: f64;

        // Earth's central angle between the user position and the earth projection of
        // ionospheric intersection point (semi-circles)
        let psi = 0.0137f64 / (E + 0.11f64) - 0.022f64;

        // Geodetic latitude of the earth projection of the ionospheric intersection point
        // (semi-circles)
        let phi_i = phi_u + psi * (azel[0]).cos();
        let phi_i = phi_i.clamp(-0.416f64, 0.416f64);

        // Geodetic longitude of the earth projection of the ionospheric intersection point
        // (semi-circles)
        let lam_i = lam_u + psi * (azel[0]).sin() / (phi_i * PI).cos();
        // Geomagnetic latitude of the earth projection of the ionospheric intersection
        // point (mean ionospheric height assumed 350 km) (semi-circles)
        let phi_m = phi_i + 0.064f64 * ((lam_i - 1.617f64) * PI).cos();
        let phi_m2 = phi_m * phi_m;
        let phi_m3 = phi_m2 * phi_m;
        let mut AMP = ionoutc.alpha0
            + ionoutc.alpha1 * phi_m
            + ionoutc.alpha2 * phi_m2
            + ionoutc.alpha3 * phi_m3;
        if AMP < 0.0f64 {
            AMP = 0.0f64;
        }
        PER =
            ionoutc.beta0 + ionoutc.beta1 * phi_m + ionoutc.beta2 * phi_m2 + ionoutc.beta3 * phi_m3;
        if PER < 72000.0f64 {
            PER = 72000.0f64;
        }
        // Local time (sec)
        let mut t = SECONDS_IN_DAY / 2.0f64 * lam_i + g.sec;
        while t >= SECONDS_IN_DAY {
            t -= SECONDS_IN_DAY;
        }
        while t < 0_i32 as f64 {
            t += SECONDS_IN_DAY;
        }
        // Phase (radians)
        let X = 2.0f64 * PI * (t - 50400.0f64) / PER;
        if (X).abs() < 1.57f64 {
            let X2 = X * X;
            let X4 = X2 * X2;
            iono_delay =
                F * (5.0e-9f64 + AMP * (1.0f64 - X2 / 2.0f64 + X4 / 24.0f64)) * SPEED_OF_LIGHT;
        } else {
            iono_delay = F * 5.0e-9f64 * SPEED_OF_LIGHT;
        }
    }
    iono_delay
}

///  \brief Compute range between a satellite and the receiver
///  \param[out] rho The computed range
///  \param[in] eph Ephemeris data of the satellite
///  \param[in] g GPS time at time of receiving the signal
///  \param[in] xyz position of the receiver
pub fn computeRange(
    rho: &mut range_t,
    eph: &ephem_t,
    ionoutc: &mut ionoutc_t,
    g: &gpstime_t,
    xyz_0: &[f64; 3],
) {
    let mut pos: [f64; 3] = [0.; 3];
    let mut vel: [f64; 3] = [0.; 3];
    let mut clk: [f64; 2] = [0.; 2];
    let mut los: [f64; 3] = [0.; 3];
    let mut llh: [f64; 3] = [0.; 3];
    let mut neu: [f64; 3] = [0.; 3];
    let mut tmat: [[f64; 3]; 3] = [[0.; 3]; 3];
    // SV position at time of the pseudorange observation.
    satpos(eph, g, &mut pos, &mut vel, &mut clk);
    // Receiver to satellite vector and light-time.
    subVect(&mut los, &pos, xyz_0);
    let tau = normVect(&los) / SPEED_OF_LIGHT;
    // Extrapolate the satellite position backwards to the transmission time.
    pos[0] -= vel[0] * tau;
    pos[1] -= vel[1] * tau;
    pos[2] -= vel[2] * tau;
    let xrot = pos[0] + pos[1] * OMEGA_EARTH * tau;
    let yrot = pos[1] - pos[0] * OMEGA_EARTH * tau;
    pos[0] = xrot;
    pos[1] = yrot;
    // New observer to satellite vector and satellite range.
    subVect(&mut los, &pos, xyz_0);
    let range = normVect(&los);
    (rho).d = range;
    // Pseudorange.
    (rho).range = range - SPEED_OF_LIGHT * clk[0];
    // Relative velocity of SV and receiver.
    let rate = dotProd(&vel, &los) / range;
    // Pseudorange rate.
    (rho).rate = rate; // - SPEED_OF_LIGHT*clk[1];
    // Time of application.
    rho.g = *g;
    // Azimuth and elevation angles.
    xyz2llh(xyz_0, &mut llh);
    ltcmat(&llh, &mut tmat);
    ecef2neu(&los, &tmat, &mut neu);
    neu2azel(&mut (rho).azel, &neu);
    // Add ionospheric delay
    (rho).iono_delay = ionosphericDelay(ionoutc, g, &llh, &(rho).azel);
    (rho).range += (rho).iono_delay;
}

///  \brief Compute the code phase for a given channel (satellite)
///  \param chan Channel on which we operate (is updated)
///  \param[in] rho1 Current range, after \a dt has expired
///  \param[in dt delta-t (time difference) in seconds
pub fn computeCodePhase(chan: &mut channel_t, rho1: range_t, dt: f64) {
    // Pseudorange rate.
    let rhorate = (rho1.range - chan.rho0.range) / dt;
    // Carrier and code frequency.
    chan.f_carr = -rhorate / LAMBDA_L1;
    chan.f_code = CODE_FREQ + chan.f_carr * CARR_TO_CODE;
    // Initial code phase and data bit counters.
    let ms =
        (subGpsTime(chan.rho0.g, chan.g0) + 6.0f64 - chan.rho0.range / SPEED_OF_LIGHT) * 1000.0f64;
    let mut ims = ms as i32;
    chan.code_phase = (ms - ims as f64) * CA_SEQ_LEN as f64; // in chip
    chan.iword = ims / 600_i32; // 1 word = 30 bits = 600 ms
    ims -= chan.iword * 600_i32;
    chan.ibit = ims / 20_i32; // 1 bit = 20 code = 20 ms
    ims -= chan.ibit * 20_i32;
    chan.icode = ims; // 1 code = 1 ms
    chan.codeCA = chan.ca[chan.code_phase as i32 as usize] * 2_i32 - 1_i32;
    chan.dataBit =
        (chan.dwrd[chan.iword as usize] >> (29_i32 - chan.ibit) & 0x1_u32) as i32 * 2_i32 - 1_i32;
    // Save current pseudorange
    chan.rho0 = rho1;
}

pub fn generateNavMsg(g: &gpstime_t, chan: &mut channel_t, init: i32) -> i32 {
    let mut g0: gpstime_t = gpstime_t { week: 0, sec: 0. };
    let mut sbfwrd: u32;
    let mut prevwrd: u32 = 0;
    let mut nib: i32;
    g0.week = g.week;
    g0.sec = ((g.sec + 0.5f64) as u32).wrapping_div(30) as f64 * 30.0f64; // Align with the full frame length = 30 sec
    chan.g0 = g0; // Data bit reference time

    let wn = (g0.week % 1024_i32) as u32;
    let mut tow = (g0.sec as u32).wrapping_div(6);
    if init == 1_i32 {
        // Initialize subframe 5
        prevwrd = 0_u32;
        for iwrd in 0..N_DWRD_SBF {
            sbfwrd = chan.sbf[4][iwrd];
            // Add TOW-count message into HOW
            if iwrd == 1 {
                sbfwrd |= (tow & 0x1ffff_u32) << 13_i32;
            }
            // Compute checksum
            sbfwrd |= prevwrd << 30_i32 & 0xc0000000_u32; // 2 LSBs of the previous transmitted word
            nib = if iwrd == 1 || iwrd == 9 { 1 } else { 0 }; // Non-information bearing bits for word 2 and 10
            chan.dwrd[iwrd] = computeChecksum(sbfwrd, nib);
            prevwrd = chan.dwrd[iwrd];
        }
    } else {
        // Save subframe 5
        for iwrd in 0..N_DWRD_SBF {
            chan.dwrd[iwrd] = chan.dwrd[N_DWRD_SBF * N_SBF + iwrd];
            prevwrd = chan.dwrd[iwrd];
        }
        /*
        // Sanity check
        if (((chan->dwrd[1])&(0x1FFFFUL<<13)) != ((tow&0x1FFFFUL)<<13))
        {
            fprintf(stderr, "\nWARNING: Invalid TOW in subframe 5.\n");
            return(0);
        }
        */
    }
    for isbf in 0..N_SBF {
        tow = tow.wrapping_add(1);

        for iwrd in 0..N_DWRD_SBF {
            sbfwrd = chan.sbf[isbf][iwrd];
            // Add transmission week number to Subframe 1
            if isbf == 0 && iwrd == 2 {
                sbfwrd |= (wn & 0x3ff_u32) << 20_i32;
            }
            // Add TOW-count message into HOW
            if iwrd == 1 {
                sbfwrd |= (tow & 0x1ffff_u32) << 13_i32;
            }
            // Compute checksum
            sbfwrd |= prevwrd << 30_i32 & 0xc0000000_u32; // 2 LSBs of the previous transmitted word
            nib = if iwrd == 1 || iwrd == 9 { 1 } else { 0 }; // Non-information bearing bits for word 2 and 10
            chan.dwrd[(isbf + 1) * N_DWRD_SBF + iwrd] = computeChecksum(sbfwrd, nib);
            prevwrd = chan.dwrd[(isbf + 1) * N_DWRD_SBF + iwrd];
        }
    }
    1_i32
}

pub fn checkSatVisibility(
    eph: ephem_t,
    g: &gpstime_t,
    xyz_0: &[f64; 3],
    elvMask: f64,
    azel: &mut [f64; 2],
) -> i32 {
    let mut llh: [f64; 3] = [0.; 3];
    let mut neu: [f64; 3] = [0.; 3];
    let mut pos: [f64; 3] = [0.; 3];
    let mut vel: [f64; 3] = [0.; 3];
    // modified from [f64;3] to [f64;2]
    let mut clk: [f64; 2] = [0.; 2];
    let mut los: [f64; 3] = [0.; 3];
    let mut tmat: [[f64; 3]; 3] = [[0.; 3]; 3];
    if eph.vflg != 1_i32 {
        return -1_i32; // Invalid
    }
    xyz2llh(xyz_0, &mut llh);
    ltcmat(&llh, &mut tmat);
    satpos(&eph, g, &mut pos, &mut vel, &mut clk);
    subVect(&mut los, &pos, xyz_0);
    ecef2neu(&los, &tmat, &mut neu);
    neu2azel(azel, &neu);
    if azel[1] * R2D > elvMask {
        return 1_i32; // Visible
    }
    0_i32 // Invisible
}

pub fn allocateChannel(
    chan: &mut [channel_t; 16],
    eph: &mut [ephem_t; 32],
    ionoutc: &mut ionoutc_t,
    grx: &gpstime_t,
    xyz_0: &[f64; 3],
    mut _elvMask: f64,
    allocatedSat: &mut [i32; MAX_SAT],
) -> i32 {
    let mut nsat: i32 = 0_i32;
    let mut azel: [f64; 2] = [0.; 2];
    let mut rho: range_t = range_t {
        g: gpstime_t { week: 0, sec: 0. },
        range: 0.,
        rate: 0.,
        d: 0.,
        azel: [0.; 2],
        iono_delay: 0.,
    };
    let ref_0: [f64; 3] = [0.0f64, 0., 0.];
    // #[allow(unused_variables)]
    // let mut r_ref: f64 = 0.;
    // #[allow(unused_variables)]
    // let mut r_xyz: f64;
    let mut phase_ini: f64;
    for sv in 0..MAX_SAT {
        if checkSatVisibility(eph[sv], grx, xyz_0, 0.0f64, &mut azel) == 1_i32 {
            nsat += 1; // Number of visible satellites
            if allocatedSat[sv] == -1_i32 {
                // Visible but not allocated
                //
                // Allocated new satellite
                let mut i = 0;
                while i < MAX_CHAN {
                    if chan[i].prn == 0_i32 {
                        // Initialize channel
                        chan[i].prn = sv as i32 + 1;
                        chan[i].azel[0] = azel[0];
                        chan[i].azel[1] = azel[1];
                        // C/A code generation
                        codegen(&mut chan[i].ca, (chan[i]).prn);
                        // Generate subframe
                        eph2sbf(eph[sv], ionoutc, &mut chan[i].sbf);
                        // Generate navigation message
                        generateNavMsg(grx, &mut chan[i], 1_i32);
                        // Initialize pseudorange
                        computeRange(&mut rho, &eph[sv], ionoutc, grx, xyz_0);
                        (chan[i]).rho0 = rho;
                        // Initialize carrier phase
                        // r_xyz = rho.range;
                        computeRange(&mut rho, &eph[sv], ionoutc, grx, &ref_0);
                        // r_ref = rho.range;
                        phase_ini = 0.0f64; // TODO: Must initialize properly
                        //phase_ini = (2.0*r_ref - r_xyz)/LAMBDA_L1;
                        // #ifdef FLOAT_CARR_PHASE
                        //                         chan[i].carr_phase = phase_ini - floor(phase_ini);
                        // #else
                        phase_ini -= (phase_ini).floor();
                        (chan[i]).carr_phase = (512.0f64 * 65536.0f64 * phase_ini) as u32;
                        break;
                    } else {
                        i += 1;
                    }
                }
                // Set satellite allocation channel
                if i < MAX_CHAN {
                    allocatedSat[sv] = i as i32;
                }
            }
        } else if allocatedSat[sv] >= 0_i32 {
            // Not visible but allocated
            // Clear channel
            (chan[allocatedSat[sv] as usize]).prn = 0_i32;
            // Clear satellite allocation flag
            allocatedSat[sv] = -1_i32;
        }
    }
    nsat
}

pub fn process(params: Params) -> i32 {
    let mut allocatedSat: [i32; MAX_SAT] = [0; 32];

    let mut fp_out: Option<std::fs::File>;
    let mut eph: [[ephem_t; MAX_SAT]; EPHEM_ARRAY_SIZE] =
        [[ephem_t::default(); MAX_SAT]; EPHEM_ARRAY_SIZE];
    let mut chan: [channel_t; 16] = [channel_t {
        prn: 0,
        ca: [0; 1023],
        f_carr: 0.,
        f_code: 0.,
        carr_phase: 0,
        carr_phasestep: 0,
        code_phase: 0.,
        g0: gpstime_t::default(),
        sbf: [[0; 10]; 5],
        dwrd: [0; 60],
        iword: 0,
        ibit: 0,
        icode: 0,
        dataBit: 0,
        codeCA: 0,
        azel: [0.; 2],
        rho0: range_t::default(),
    }; 16];
    let elvmask: f64 = 0.0f64;

    // Default options
    // let mut umfile: [libc::c_char; 100] = [0; 100];
    // let mut navfile: [libc::c_char; 100] = [0; 100];
    // let mut outfile: [libc::c_char; 100] = [0; 100];
    let mut gain: [i32; 16] = [0; 16];
    let mut ant_pat: [f64; 37] = [0.; 37];
    let mut tmin: datetime_t = datetime_t::default();
    let mut tmax: datetime_t = datetime_t::default();
    let mut gmin: gpstime_t = gpstime_t { week: 0, sec: 0. };
    let mut gmax: gpstime_t = gpstime_t { week: 0, sec: 0. };
    let navfile = params.navfile;
    let umfile = params.umfile;
    let nmeaGGA = params.nmeaGGA;
    let umLLH = params.umLLH;
    let mut staticLocationMode = params.staticLocationMode;
    let mut xyz = params.xyz;
    let mut llh = params.llh;
    let outfile = params.outfile;
    let samp_freq = params.samp_freq;
    let data_format = params.data_format;
    let mut ionoutc = params.ionoutc;
    let timeoverwrite = params.timeoverwrite;
    let mut t0 = params.t0;
    let mut g0 = params.g0;
    let duration = params.duration;
    let fixed_gain = params.fixed_gain;
    let path_loss_enable = params.path_loss_enable;
    let verb = params.verb;

    if umfile.is_none() && !staticLocationMode {
        // Default static location; Tokyo
        staticLocationMode = true;
        llh[0] = 35.681298f64 / R2D;
        llh[1] = 139.766247f64 / R2D;
        llh[2] = 10.0f64;
    }
    if duration < 0.0f64
        || duration > USER_MOTION_SIZE as i32 as f64 / 10.0f64 && !staticLocationMode
        || duration > STATIC_MAX_DURATION as f64 && staticLocationMode
    {
        eprintln!("ERROR: Invalid duration.");
        panic!();
    }
    let iduration = (duration * 10.0f64 + 0.5f64) as i32;
    let mut samp_freq = (samp_freq / 10.0f64).floor();
    let iq_buff_size = samp_freq as usize; // samples per 0.1sec
    samp_freq *= 10.0f64;
    // let delt = 1.0f64 / samp_freq;
    let delt = samp_freq.recip();

    ////////////////////////////////////////////////////////////
    // Receiver position
    ////////////////////////////////////////////////////////////
    let mut numd: i32;
    if !staticLocationMode {
        let umfilex = umfile.clone().unwrap();
        if nmeaGGA {
            numd = read_Nmea_GGA(&mut xyz, &umfilex).unwrap();
            // numd = readNmeaGGA(&mut xyz, umfile);
        } else if umLLH {
            numd = read_user_motion_LLH(&mut xyz, &umfilex).unwrap();
            // numd = unsafe { readUserMotionLLH(&mut xyz, umfile) };
        } else {
            numd = read_user_motion(&mut xyz, &umfilex).unwrap();
            // numd = unsafe { readUserMotion(&mut xyz, umfile) };
        }
        if numd == -1_i32 {
            eprintln!("ERROR: Failed to open user motion / NMEA GGA file.");
            panic!();
        } else if numd == 0_i32 {
            eprintln!("ERROR: Failed to read user motion / NMEA GGA data.");
            panic!();
        }
        // Set simulation duration
        if numd > iduration {
            numd = iduration;
        }
        // Set user initial position
        xyz2llh(&xyz[0], &mut llh);
    } else {
        // Static geodetic coordinates input mode: "-l"
        // Added by scateu@gmail.com
        eprintln!("Using static location mode.");
        // Set simulation duration
        numd = iduration;
        // Set user initial position
        llh2xyz(&llh, &mut xyz[0]);
    }

    eprintln!("xyz = {}, {}, {}", xyz[0][0], xyz[0][1], xyz[0][2],);

    eprintln!("llh = {}, {}, {}", llh[0] * R2D, llh[1] * R2D, llh[2],);

    ////////////////////////////////////////////////////////////
    // Read ephemeris
    ////////////////////////////////////////////////////////////
    // let navfile = navfile.to_str().unwrap_or("");
    // let c_string = CString::new(navfile).unwrap();
    // let navff = c_string.into_raw();
    // let neph = readRinexNavAll(&mut eph, &mut ionoutc, navff);
    let neph = read_rinex_nav_all(&mut eph, &mut ionoutc, &navfile).unwrap();
    if neph == 0 {
        eprintln!("ERROR: No ephemeris available.",);
        panic!();
    } else if neph == usize::MAX {
        eprintln!("ERROR: ephemeris file not found.");
        panic!();
    }
    if verb && ionoutc.vflg == 1_i32 {
        eprintln!(
            "  {:12.3e} {:12.3e} {:12.3e} {:12.3e}",
            ionoutc.alpha0, ionoutc.alpha1, ionoutc.alpha2, ionoutc.alpha3,
        );

        eprintln!(
            "  {:12.3e} {:12.3e} {:12.3e} {:12.3e}",
            ionoutc.beta0, ionoutc.beta1, ionoutc.beta2, ionoutc.beta3,
        );

        eprintln!(
            "   {:19.11e} {:19.11e}  {:9} {:9}",
            ionoutc.A0, ionoutc.A1, ionoutc.tot, ionoutc.wnt,
        );

        eprintln!("{:6}", ionoutc.dtls,);
    }
    for sv in 0..MAX_SAT {
        if eph[0][sv].vflg == 1_i32 {
            gmin = eph[0][sv].toc;
            tmin = eph[0][sv].t;
            break;
        }
    }
    gmax.sec = 0_i32 as f64;
    gmax.week = 0_i32;
    tmax.sec = 0_i32 as f64;
    tmax.mm = 0_i32;
    tmax.hh = 0_i32;
    tmax.d = 0_i32;
    tmax.m = 0_i32;
    tmax.y = 0_i32;

    for sv in 0..MAX_SAT {
        if eph[neph - 1][sv].vflg == 1_i32 {
            gmax = eph[neph - 1][sv].toc;
            tmax = eph[neph - 1][sv].t;
            break;
        }
    }
    if g0.week >= 0_i32 {
        // Scenario start time has been set.
        if timeoverwrite {
            let mut gtmp: gpstime_t = gpstime_t::default();
            let mut ttmp: datetime_t = datetime_t::default();
            gtmp.week = g0.week;
            gtmp.sec = (g0.sec as i32 / 7200_i32) as f64 * 7200.0f64;
            // Overwrite the UTC reference week number
            let dsec = subGpsTime(gtmp, gmin);
            ionoutc.wnt = gtmp.week;
            ionoutc.tot = gtmp.sec as i32;
            // Iono/UTC parameters may no longer valid
            //ionoutc.vflg = FALSE;
            for sv in 0..MAX_SAT {
                for i_eph in eph.iter_mut().take(neph) {
                    if i_eph[sv].vflg == 1_i32 {
                        gtmp = incGpsTime(i_eph[sv].toc, dsec);
                        gps2date(&gtmp, &mut ttmp);
                        i_eph[sv].toc = gtmp;
                        i_eph[sv].t = ttmp;
                        gtmp = incGpsTime(i_eph[sv].toe, dsec);
                        i_eph[sv].toe = gtmp;
                    }
                }
            }
        } else if subGpsTime(g0, gmin) < 0.0f64 || subGpsTime(gmax, g0) < 0.0f64 {
            eprintln!("ERROR: Invalid start time.");
            eprintln!(
                "tmin = {:4}/{:02}/{:02},{:02}:{:02}:{:0>2.0} ({}:{:.0})",
                tmin.y, tmin.m, tmin.d, tmin.hh, tmin.mm, tmin.sec, gmin.week, gmin.sec,
            );
            eprintln!(
                "tmax = {:4}/{:02}/{:02},{:02}:{:02}:{:0>2.0} ({}:{:.0})",
                tmax.y, tmax.m, tmax.d, tmax.hh, tmax.mm, tmax.sec, gmax.week, gmax.sec,
            );
            panic!();
        }
    } else {
        g0 = gmin;
        t0 = tmin;
    }

    eprintln!(
        "Start time = {:4}/{:02}/{:02},{:02}:{:02}:{:0>2.0} ({}:{:.0})",
        t0.y, t0.m, t0.d, t0.hh, t0.mm, t0.sec, g0.week, g0.sec,
    );

    eprintln!("Duration = {:.1} [sec]", numd as f64 / 10.0f64);

    // Select the current set of ephemerides
    let mut ieph = usize::MAX;
    for (i, eph_item) in eph.iter().enumerate().take(neph) {
        for e in eph_item.iter().take(MAX_SAT) {
            if e.vflg == 1_i32 {
                let dt = subGpsTime(g0, e.toc);
                if (-SECONDS_IN_HOUR..SECONDS_IN_HOUR).contains(&dt) {
                    ieph = i;
                    break;
                }
            }
        }
        if ieph != usize::MAX {
            // ieph has been set
            break;
        }
        // if ieph >= 0 {
        //     break;
        // }
    }

    if ieph == usize::MAX {
        eprintln!("ERROR: No current set of ephemerides has been found.",);
        panic!();
    }

    ////////////////////////////////////////////////////////////
    // Baseband signal buffer and output file
    ////////////////////////////////////////////////////////////

    // Allocate I/Q buffer
    let mut iq_buff: Vec<i16> = vec![0i16; 2 * iq_buff_size];
    let mut iq8_buff: Vec<i8> = vec![0i8; 2 * iq_buff_size];
    if data_format == SC08 {
        iq8_buff = vec![0i8; 2 * iq_buff_size];
    } else if data_format == SC01 {
        iq8_buff = vec![0i8; iq_buff_size / 4]; // byte = {I0, Q0, I1, Q1, I2, Q2, I3, Q3}
    }

    // Open output file
    // "-" can be used as name for stdout
    // if strcmp(
    //     b"-\0" as *const u8 as *const libc::c_char,
    //     outfile.as_mut_ptr(),
    // ) != 0
    // {
    //     fp = fopen(
    //         outfile.as_mut_ptr(),
    //         b"wb\0" as *const u8 as *const libc::c_char,
    //     );
    //     if fp.is_null() {
    //         eprintln!("ERROR: Failed to open output file.");
    //         panic!();
    //     }
    // } else {
    //     // todo: temporarily disable
    //     // fp = stdout;
    // }
    // let out_file = String::from_utf8(outfile.iter().map(|&c| c as u8).collect());
    // if let Ok(out_file) = out_file {
    //     if out_file != "-" {
    //         let file_name = out_file.trim_end_matches("\0");
    fp_out = std::fs::File::create(outfile).ok();
    //     } else {
    //         // use stdout
    //         unimplemented!()
    //     }
    // }

    ////////////////////////////////////////////////////////////
    // Initialize channels
    ////////////////////////////////////////////////////////////

    // Clear all channels
    chan.iter_mut().take(MAX_CHAN).for_each(|ch| ch.prn = 0);
    // Clear satellite allocation flag
    allocatedSat.iter_mut().take(MAX_SAT).for_each(|s| *s = -1);
    // Initial reception time
    let mut grx = incGpsTime(g0, 0.0f64);
    // Allocate visible satellites
    allocateChannel(
        &mut chan,
        &mut eph[ieph],
        &mut ionoutc,
        &grx,
        &xyz[0],
        elvmask,
        &mut allocatedSat,
    );
    // for i in 0..MAX_CHAN {
    for ichan in chan.iter().take(MAX_CHAN) {
        if ichan.prn > 0_i32 {
            eprintln!(
                "{:02} {:6.1} {:5.1} {:11.1} {:5.1}",
                ichan.prn,
                ichan.azel[0] * R2D,
                ichan.azel[1] * R2D,
                ichan.rho0.d,
                ichan.rho0.iono_delay,
            );
        }
    }

    ////////////////////////////////////////////////////////////
    // Receiver antenna gain pattern
    ////////////////////////////////////////////////////////////
    for i in 0..37 {
        ant_pat[i] = 10.0f64.powf(-ANT_PAT_DB[i] / 20.0f64);
    }

    ////////////////////////////////////////////////////////////
    // Generate baseband signals
    ////////////////////////////////////////////////////////////
    let time_start = Instant::now();
    grx = incGpsTime(grx, 0.1f64);
    for iumd in 1..numd {
        for i in 0..MAX_CHAN {
            if chan[i].prn > 0 {
                // Refresh code phase and data bit counters
                let mut rho: range_t = range_t {
                    g: gpstime_t { week: 0, sec: 0. },
                    range: 0.,
                    rate: 0.,
                    d: 0.,
                    azel: [0.; 2],
                    iono_delay: 0.,
                };
                let sv = chan[i].prn - 1;
                // Current pseudorange
                if !staticLocationMode {
                    computeRange(
                        &mut rho,
                        &eph[ieph][sv as usize],
                        &mut ionoutc,
                        &grx,
                        &xyz[iumd as usize],
                    );
                } else {
                    computeRange(
                        &mut rho,
                        &eph[ieph][sv as usize],
                        &mut ionoutc,
                        &grx,
                        &xyz[0],
                    );
                }
                // Update code phase and data bit counters
                chan[i].azel[0] = rho.azel[0];
                chan[i].azel[1] = rho.azel[1];
                computeCodePhase(&mut chan[i], rho, 0.1f64);
                chan[i].carr_phasestep =
                    (512.0f64 * 65536.0f64 * chan[i].f_carr * delt).round() as i32;

                // Path loss
                let path_loss = 20200000.0f64 / rho.d;
                // Receiver antenna gain
                let ibs = ((90.0 - rho.azel[1] * R2D) / 5.0) as usize; // covert elevation to boresight
                let ant_gain = ant_pat[ibs];
                // Signal gain
                if path_loss_enable {
                    gain[i] = (path_loss * ant_gain * 128.0f64) as i32; // scaled by 2^7
                } else {
                    gain[i] = fixed_gain; // hold the power level constant
                }
            }
        }
        for isamp in 0..iq_buff_size {
            let mut i_acc: i32 = 0_i32;
            let mut q_acc: i32 = 0_i32;
            for i in 0..16 {
                if chan[i].prn > 0_i32 {
                    // #ifdef FLOAT_CARR_PHASE
                    //                     iTable = (int)floor(chan[i].carr_phase*512.0);
                    // #else
                    let iTable = (chan[i].carr_phase >> 16_i32 & 0x1ff_i32 as u32) as usize; // 9-bit index
                    let ip = chan[i].dataBit * chan[i].codeCA * COS_TABLE512[iTable] * gain[i];
                    let qp = chan[i].dataBit * chan[i].codeCA * SIN_TABLE512[iTable] * gain[i];
                    // Accumulate for all visible satellites
                    i_acc += ip;
                    q_acc += qp;
                    // Update code phase
                    chan[i].code_phase += chan[i].f_code * delt;
                    if chan[i].code_phase >= CA_SEQ_LEN as f64 {
                        chan[i].code_phase -= CA_SEQ_LEN as f64;
                        chan[i].icode += 1;
                        if chan[i].icode >= 20_i32 {
                            // 20 C/A codes = 1 navigation data bit
                            chan[i].icode = 0_i32;
                            chan[i].ibit += 1;
                            if chan[i].ibit >= 30_i32 {
                                // 30 navigation data bits = 1 word
                                chan[i].ibit = 0_i32;
                                chan[i].iword += 1;

                                /*
                                if (chan[i].iword>=N_DWRD)
                                    fprintf(stderr, "\nWARNING: Subframe word buffer overflow.\n");
                                */
                            }
                            // Set new navigation data bit
                            chan[i].dataBit = (chan[i].dwrd[chan[i].iword as usize]
                                >> (29_i32 - chan[i].ibit)
                                & 0x1_u32) as i32
                                * 2_i32
                                - 1_i32;
                        }
                    }
                    // Set current code chip
                    chan[i].codeCA = chan[i].ca[chan[i].code_phase as i32 as usize] * 2_i32 - 1_i32;
                    // Update carrier phase
                    // #ifdef FLOAT_CARR_PHASE
                    //                     chan[i].carr_phase += chan[i].f_carr * delt;
                    //
                    //                     if (chan[i].carr_phase >= 1.0)
                    //                         chan[i].carr_phase -= 1.0;
                    //                     else if (chan[i].carr_phase<0.0)
                    //                         chan[i].carr_phase += 1.0;
                    // #else
                    chan[i].carr_phase =
                        (chan[i].carr_phase).wrapping_add(chan[i].carr_phasestep as u32);
                }
            }
            // Scaled by 2^7
            i_acc = (i_acc + 64_i32) >> 7_i32;
            q_acc = (q_acc + 64_i32) >> 7_i32;
            // Store I/Q samples into buffer
            iq_buff[isamp * 2] = i_acc as i16;
            iq_buff[isamp * 2 + 1] = q_acc as i16;
        }
        if data_format == SC01 {
            for isamp in 0..2 * iq_buff_size {
                if isamp % 8 == 0 {
                    iq8_buff[isamp / 8] = 0i8;
                }
                let fresh1_new = &mut iq8_buff[isamp / 8];

                *fresh1_new = (*fresh1_new as i32
                    | (if iq_buff[isamp] as i32 > 0_i32 {
                        0x1_i32
                    } else {
                        0_i32
                    }) << (7_i32 - isamp as i32 % 8_i32))
                    as libc::c_schar;
            }

            if let Some(file) = &mut fp_out {
                unsafe {
                    file.write_all(std::slice::from_raw_parts(
                        iq8_buff.as_ptr() as *const u8,
                        (iq_buff_size as i32 / 4_i32) as usize,
                    ))
                    .ok();
                }
            }
        } else if data_format == SC08 {
            for isamp in 0..2 * iq_buff_size {
                iq8_buff[isamp] = (iq_buff[isamp] as i32 >> 4_i32) as libc::c_schar;
                // 12-bit bladeRF -> 8-bit HackRF
                //iq8_buff[isamp] = iq_buff[isamp] >> 8; // for PocketSDR
            }

            if let Some(file) = &mut fp_out {
                unsafe {
                    file.write_all(std::slice::from_raw_parts(
                        iq8_buff.as_ptr() as *const u8,
                        (2_i32 * iq_buff_size as i32) as usize,
                    ))
                    .ok();
                }
            }
        } else if let Some(file) = &mut fp_out {
            // data_format==SC16
            let byte_slice = unsafe {
                std::slice::from_raw_parts(
                    iq_buff.as_ptr() as *const u8,
                    (2_i32 * iq_buff_size as i32 * 2) as usize, // 2 bytes per sample
                )
            };
            file.write_all(byte_slice).ok();
        }
        //
        // Update navigation message and channel allocation every 30 seconds
        //
        let igrx = (grx.sec * 10.0f64 + 0.5f64) as i32;
        if igrx % 300 == 0 {
            // Every 30 seconds
            // for i in 0..MAX_CHAN {
            for ichan in chan.iter_mut().take(MAX_CHAN) {
                if ichan.prn > 0_i32 {
                    generateNavMsg(&grx, ichan, 0_i32);
                }
            }
            // Refresh ephemeris and subframes
            // Quick and dirty fix. Need more elegant way.
            for sv in 0..MAX_SAT {
                if eph[ieph + 1][sv].vflg == 1_i32 {
                    let dt = subGpsTime(eph[ieph + 1][sv].toc, grx);
                    if dt < SECONDS_IN_HOUR {
                        ieph += 1;
                        // for i in 0..MAX_CHAN {
                        for ichan in chan.iter_mut().take(MAX_CHAN) {
                            // Generate new subframes if allocated
                            if ichan.prn != 0_i32 {
                                eph2sbf(
                                    eph[ieph][(ichan.prn - 1_i32) as usize],
                                    &ionoutc,
                                    &mut ichan.sbf,
                                );
                            }
                        }
                    }
                    break;
                }
            }
            // Update channel allocation
            if !staticLocationMode {
                allocateChannel(
                    &mut chan,
                    &mut eph[ieph],
                    &mut ionoutc,
                    &grx,
                    &xyz[iumd as usize],
                    elvmask,
                    &mut allocatedSat,
                );
            } else {
                allocateChannel(
                    &mut chan,
                    &mut eph[ieph],
                    &mut ionoutc,
                    &grx,
                    &xyz[0],
                    elvmask,
                    &mut allocatedSat,
                );
            }
            // Show details about simulated channels
            if verb {
                eprintln!();
                // for i in 0..MAX_CHAN {
                for ichan in chan.iter().take(MAX_CHAN) {
                    if ichan.prn > 0_i32 {
                        eprintln!(
                            "{:02} {:6.1} {:5.1} {:11.1} {:5.1}",
                            ichan.prn,
                            ichan.azel[0] * R2D,
                            ichan.azel[1] * R2D,
                            ichan.rho0.d,
                            ichan.rho0.iono_delay,
                        );
                    }
                }
            }
        }
        // Update receiver time
        grx = incGpsTime(grx, 0.1f64);

        // Update time counter
        eprint!("\rTime into run = {:4.1}\0", subGpsTime(grx, g0));
        // todo: temporarily disable
        // fflush(stdout);
        // iumd += 1;
    }

    eprintln!("\nDone!");
    eprintln!(
        "Process time = {:.1} [sec]",
        time_start.elapsed().as_secs_f32()
    );
    0_i32
}
