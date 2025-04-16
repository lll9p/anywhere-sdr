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

/// \brief Compute Satellite position, velocity and clock at given time
///
/// Computing Satellite Velocity using the Broadcast Ephemeris
/// <http://www.ngs.noaa.gov/gps-toolbox/bc_velo.htm>
/// \param[in] eph Ephemeris data of the satellite
/// \param[in] g GPS time at which position is to be computed
/// \param[out] pos Computed position (vector)
/// \param[out] vel Computed velocity (vector)
/// \param[clk] clk Computed clock
pub fn satpos(
    eph: &Ephemeris,
    g: &GpsTime, /* , pos: &mut [f64; 3], vel: &mut [f64; 3],
                 * clk: &mut [f64; 2], */
) -> ([f64; 3], [f64; 3], [f64; 2]) {
    let mut tk = g.sec - eph.toe.sec;
    if tk > SECONDS_IN_HALF_WEEK {
        tk -= SECONDS_IN_WEEK;
    } else if tk < -SECONDS_IN_HALF_WEEK {
        tk += SECONDS_IN_WEEK;
    }
    let mk = eph.m0 + eph.n * tk;
    let mut ek = mk;
    let mut ekold = ek + 1.0;
    let mut one_minusecos_e = 0.0; // Suppress the uninitialized warning.
    while (ek - ekold).abs() > 1.0e-14 {
        ekold = ek;
        one_minusecos_e = 1.0 - eph.ecc * ekold.cos();
        ek += (mk - ekold + eph.ecc * (ekold.sin())) / one_minusecos_e;
    }
    let sek = ek.sin();
    let cek = ek.cos();
    let ekdot = eph.n / one_minusecos_e;
    let relativistic = -4.442_807_633E-10 * eph.ecc * eph.sqrta * sek;
    let pk = (eph.sq1e2 * sek).atan2(cek - eph.ecc) + eph.aop;
    let pkdot = eph.sq1e2 * ekdot / one_minusecos_e;
    let s2pk = (2.0 * pk).sin();
    let c2pk = (2.0 * pk).cos();
    let uk = pk + eph.cus * s2pk + eph.cuc * c2pk;
    let suk = uk.sin();
    let cuk = uk.cos();
    let ukdot = pkdot * (1.0 + 2.0 * (eph.cus * c2pk - eph.cuc * s2pk));
    let rk = eph.A * one_minusecos_e + eph.crc * c2pk + eph.crs * s2pk;
    let rkdot = eph.A * eph.ecc * sek * ekdot
        + 2.0 * pkdot * (eph.crs * c2pk - eph.crc * s2pk);
    let ik = eph.inc0 + eph.idot * tk + eph.cic * c2pk + eph.cis * s2pk;
    let sik = ik.sin();
    let cik = ik.cos();
    let ikdot = eph.idot + 2.0 * pkdot * (eph.cis * c2pk - eph.cic * s2pk);
    let xpk = rk * cuk;
    let ypk = rk * suk;
    let xpkdot = rkdot * cuk - ypk * ukdot;
    let ypkdot = rkdot * suk + xpk * ukdot;
    let ok = eph.omg0 + tk * eph.omgkdot - OMEGA_EARTH * eph.toe.sec;
    let sok = ok.sin();
    let cok = ok.cos();
    let pos = [
        xpk * cok - ypk * cik * sok,
        xpk * sok + ypk * cik * cok,
        ypk * sik,
    ];
    // pos[0] = xpk * cok - ypk * cik * sok;
    // pos[1] = xpk * sok + ypk * cik * cok;
    // pos[2] = ypk * sik;
    let tmp = ypkdot * cik - ypk * sik * ikdot;
    let vel = [
        -eph.omgkdot * pos[1] + xpkdot * cok - tmp * sok,
        eph.omgkdot * pos[0] + xpkdot * sok + tmp * cok,
        ypk * cik * ikdot + ypkdot * sik,
    ];
    // vel[0] = -eph.omgkdot * pos[1] + xpkdot * cok - tmp * sok;
    // vel[1] = eph.omgkdot * pos[0] + xpkdot * sok + tmp * cok;
    // vel[2] = ypk * cik * ikdot + ypkdot * sik;
    let mut tk = g.sec - eph.toc.sec;
    if tk > SECONDS_IN_HALF_WEEK {
        tk -= SECONDS_IN_WEEK;
    } else if tk < -SECONDS_IN_HALF_WEEK {
        tk += SECONDS_IN_WEEK;
    }
    let clk = [
        eph.af0 + tk * (eph.af1 + tk * eph.af2) + relativistic - eph.tgd,
        eph.af1 + 2.0 * tk * eph.af2,
    ];
    (pos, vel, clk)
    // clk[0] = eph.af0 + tk * (eph.af1 + tk * eph.af2) + relativistic -
    // eph.tgd; clk[1] = eph.af1 + 2.0 * tk * eph.af2;
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

///  \brief Compute the Checksum for one given word of a subframe
///  \param[in] source The input data
///  \param[in] nib Does this word contain non-information-bearing bits?
///  \returns Computed Checksum
#[allow(non_snake_case)]
pub fn compute_checksum(source: u32, nib: i32) -> u32 {
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
        0x3b1f_3480,
        0x1d8f_9a40,
        0x2ec7_cd00,
        0x1763_e680,
        0x2bb1_f340,
        0x0b7a_89c0,
    ];
    let mut D: u32;
    let mut d: u32 = source & 0x3fff_ffc0;
    let D29: u32 = source >> 31 & 0x1;
    let D30: u32 = source >> 30 & 0x1;
    if nib != 0 {
        // Non-information bearing bits for word 2 and 10
        /*
        Solve bits 23 and 24 to preserve parity check
        with zeros in bits 29 and 30.
        */
        if D30
            .wrapping_add((bmask[4] & d).count_ones())
            .wrapping_rem(2)
            != 0
        {
            d ^= 0x1 << 6;
        }
        if D29
            .wrapping_add((bmask[5] & d).count_ones())
            .wrapping_rem(2)
            != 0
        {
            d ^= 0x1 << 7;
        }
    }
    D = d;
    if D30 != 0 {
        D ^= 0x3fff_ffc0;
    }
    D |= D29
        .wrapping_add((bmask[0] & d).count_ones())
        .wrapping_rem(2)
        << 5;
    D |= D30
        .wrapping_add((bmask[1] & d).count_ones())
        .wrapping_rem(2)
        << 4;
    D |= D29
        .wrapping_add((bmask[2] & d).count_ones())
        .wrapping_rem(2)
        << 3;
    D |= D30
        .wrapping_add((bmask[3] & d).count_ones())
        .wrapping_rem(2)
        << 2;
    D |= D30
        .wrapping_add((bmask[4] & d).count_ones())
        .wrapping_rem(2)
        << 1;
    D |= D29
        .wrapping_add((bmask[5] & d).count_ones())
        .wrapping_rem(2);
    D &= 0x3fff_ffff;

    //D |= (source & 0xC0000000UL); // Add D29* and D30* from source data bits
    D
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
    let (mut pos, vel, clk) = satpos(eph, g);
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

///  \brief Compute the code phase for a given channel (satellite)
///  \param chan Channel on which we operate (is updated)
///  \param[in] rho1 Current range, after \a dt has expired
///  \param[in dt delta-t (time difference) in seconds
pub fn compute_code_phase(chan: &mut Channel, rho1: TimeRange, dt: f64) {
    // Pseudorange rate.
    let rhorate = (rho1.range - chan.rho0.range) / dt;
    // Carrier and code frequency.
    chan.f_carr = -rhorate / LAMBDA_L1;
    chan.f_code = CODE_FREQ + chan.f_carr * CARR_TO_CODE;
    // Initial code phase and data bit counters.
    let ms = (chan.rho0.g.diff_secs(&chan.g0) + 6.0
        - chan.rho0.range / SPEED_OF_LIGHT)
        * 1000.0;
    let mut ims = ms as i32;
    chan.code_phase = (ms - f64::from(ims)) * CA_SEQ_LEN as f64; // in chip
    chan.iword = ims / 600; // 1 word = 30 bits = 600 ms
    ims -= chan.iword * 600;
    chan.ibit = ims / 20; // 1 bit = 20 code = 20 ms
    ims -= chan.ibit * 20;
    chan.icode = ims; // 1 code = 1 ms
    chan.codeCA = chan.ca[chan.code_phase as usize] * 2 - 1;
    chan.dataBit =
        (chan.dwrd[chan.iword as usize] >> (29 - chan.ibit) & 0x1) as i32 * 2
            - 1;
    // Save current pseudorange
    chan.rho0 = rho1;
}

pub fn generate_nav_msg(g: &GpsTime, chan: &mut Channel, init: bool) {
    let mut g0: GpsTime = GpsTime { week: 0, sec: 0. };
    let mut sbfwrd: u32;
    let mut prevwrd: u32 = 0;
    let mut nib: i32;
    g0.week = g.week;
    g0.sec = f64::from(((g.sec + 0.5) as u32).wrapping_div(30)) * 30.0; // Align with the full frame length = 30 sec
    chan.g0 = g0; // Data bit reference time

    let wn = (g0.week % 1024) as u32;
    let mut tow = (g0.sec as u32).wrapping_div(6);
    if init {
        // Initialize subframe 5
        prevwrd = 0;
        for iwrd in 0..N_DWRD_SBF {
            sbfwrd = chan.sbf[4][iwrd];
            // Add TOW-count message into HOW
            if iwrd == 1 {
                sbfwrd |= (tow & 0x1ffff) << 13;
            }
            // Compute checksum
            sbfwrd |= prevwrd << 30 & 0xc000_0000; // 2 LSBs of the previous transmitted word
            nib = i32::from(iwrd == 1 || iwrd == 9); // Non-information bearing bits for word 2 and 10
            chan.dwrd[iwrd] = compute_checksum(sbfwrd, nib);
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
                sbfwrd |= (wn & 0x3ff) << 20;
            }
            // Add TOW-count message into HOW
            if iwrd == 1 {
                sbfwrd |= (tow & 0x1ffff) << 13;
            }
            // Compute checksum
            sbfwrd |= prevwrd << 30 & 0xc000_0000; // 2 LSBs of the previous transmitted word
            nib = i32::from(iwrd == 1 || iwrd == 9); // Non-information bearing bits for word 2 and 10
            chan.dwrd[(isbf + 1) * N_DWRD_SBF + iwrd] =
                compute_checksum(sbfwrd, nib);
            prevwrd = chan.dwrd[(isbf + 1) * N_DWRD_SBF + iwrd];
        }
    }
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
    let (pos, _vel, _clk) = satpos(eph, g);
    sub_vect(&mut los, &pos, xyz_0);
    ecef2neu(&los, &tmat, &mut neu);
    neu2azel(azel, &neu);
    if azel[1] * R2D > elv_mask {
        return 1; // Visible
    }
    0 // Invisible
}

pub fn allocate_channel(
    chan: &mut [Channel; 16], eph: &mut [Ephemeris; 32], ionoutc: &mut IonoUtc,
    grx: &GpsTime, xyz_0: &[f64; 3], _elv_mask: f64,
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
                        generate_nav_msg(grx, &mut chan[i], true);
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
