use constants::{
    PI, POW2_M5, POW2_M19, POW2_M24, POW2_M27, POW2_M29, POW2_M30, POW2_M31,
    POW2_M33, POW2_M43, POW2_M50, POW2_M55,
};

use super::Channel;
use crate::{ephemeris::Ephemeris, ionoutc::IonoUtc};

impl Channel {
    /// Constructs the GPS navigation message subframes from ephemeris and UTC
    /// parameters.
    ///
    /// This method implements the detailed bit-level formatting of the GPS
    /// navigation message as specified in the Interface Specification
    /// IS-GPS-200. It converts the satellite ephemeris and ionospheric/UTC
    /// parameters into the binary format transmitted by GPS satellites.
    ///
    /// The navigation message consists of 5 subframes, each containing 10 words
    /// of 30 bits:
    /// - Subframe 1: Satellite clock parameters, GPS week number, and satellite
    ///   health
    /// - Subframe 2: Ephemeris parameters (first part)
    /// - Subframe 3: Ephemeris parameters (second part)
    /// - Subframe 4: Almanac, ionospheric model, UTC parameters (uses page 18)
    /// - Subframe 5: Almanac for other satellites (uses page 25)
    ///
    /// Each parameter is scaled according to the GPS ICD specifications and
    /// placed in the appropriate bit positions within each word. The method
    /// also handles special cases such as:
    /// - Proper scaling of floating-point values to integer representations
    /// - Handling of sign bits for signed parameters
    /// - Inclusion of preamble and telemetry words
    /// - Formatting of Time of Week (TOW) counters
    ///
    /// # Arguments
    /// * `eph` - Satellite ephemeris containing orbital parameters and clock
    ///   corrections
    /// * `ionoutc` - Ionospheric delay model and UTC time conversion parameters
    ///
    /// # Implementation Details
    /// - Subframes 1-3 contain the fundamental ephemeris and clock correction
    ///   data needed for precise positioning
    /// - Subframe 4 page 18 includes:
    ///   - Ionospheric α/β coefficients (Klobuchar model parameters)
    ///   - UTC parameters (`A0`, `A1`, `ΔtLS`)
    ///   - Leap second transition parameters
    /// - Subframe 5 page 25 is reserved (zero-filled in this implementation)
    /// - All value conversions follow GPS-ICD-defined scaling factors and
    ///   bit-field layouts
    /// - The constructed subframes are stored in the channel's `subframes`
    ///   field
    #[allow(clippy::too_many_lines)]
    pub fn generate_navigation_subframes(
        &mut self, eph: &Ephemeris, ionoutc: &IonoUtc,
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
        // ephemeris reference time wn = (unsigned long)(self.toe.week%1024);
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
        let week_number = (ionoutc.week_number % 256) as u32;
        // 2016/12/31 (Sat) -> WNlsf = 1929, DN = 7 (http://navigationservices.agi.com/GNSSWeb/)
        // Days are counted from 1 to 7 (Sunday is 1).
        if ionoutc.leapen == 1 {
            wnlsf = (ionoutc.wnlsf % 256) as u32;
            dn = ionoutc.day_number as u32;
            dtlsf = ionoutc.dtlsf as u32;
        } else {
            wnlsf = (1929 % 256) as u32;
            dn = 7;
            dtlsf = 18;
        }
        // Subframe 1
        self.subframes[0] = [
            0x008b_0000 << 6,
            0x1 << 8,
            (wn & 0x3ff) << 20
                | (codeL2 as u32 & 0x3) << 18
                | (ura & 0xf) << 14
                | (svhlth as u32 & 0x3f) << 8
                | (iodc >> 8 & 0x3) << 6,
            0,
            0,
            0,
            (tgd as u32 & 0xff) << 6,
            (iodc & 0xff) << 22 | (toc & 0xffff) << 6,
            (af2 as u32 & 0xff) << 22 | (af1 as u32 & 0xffff) << 6,
            (af0 as u32 & 0x003f_ffff) << 8,
        ];
        // Subframe 2
        self.subframes[1] = [
            0x008b_0000 << 6,
            0x2 << 8,
            (iode & 0xff) << 22 | (crs as u32 & 0xffff) << 6,
            (deltan as u32 & 0xffff) << 14 | ((m0 >> 24) as u32 & 0xff) << 6,
            (m0 as u32 & 0x00ff_ffff) << 6,
            (cuc as u32 & 0xffff) << 14 | (ecc >> 24 & 0xff) << 6,
            (ecc & 0x00ff_ffff) << 6,
            (cus as u32 & 0xffff) << 14 | (sqrta >> 24 & 0xff) << 6,
            (sqrta & 0x00ff_ffff) << 6,
            (toe & 0xffff) << 14,
        ];
        // Subframe 3
        self.subframes[2] = [
            0x008b_0000 << 6,
            0x3 << 8,
            (cic as u32 & 0xffff) << 14 | ((omg0 >> 24) as u32 & 0xff) << 6,
            (omg0 as u32 & 0x00ff_ffff) << 6,
            (cis as u32 & 0xffff) << 14 | ((inc0 >> 24) as u32 & 0xff) << 6,
            (inc0 as u32 & 0x00ff_ffff) << 6,
            (crc as u32 & 0xffff) << 14 | ((aop >> 24) as u32 & 0xff) << 6,
            (aop as u32 & 0x00ff_ffff) << 6,
            (omgdot as u32 & 0x00ff_ffff) << 6,
            (iode & 0xff) << 22 | (idot as u32 & 0x3fff) << 8,
        ];
        if ionoutc.vflg {
            // Subframe 4, page 18
            self.subframes[3] = [
                0x008b_0000 << 6,
                0x4 << 8,
                data_id << 28
                    | sbf4_page18_sv_id << 22
                    | (alpha0 as u32 & 0xff) << 14
                    | (alpha1 as u32 & 0xff) << 6,
                (alpha2 as u32 & 0xff) << 22
                    | (alpha3 as u32 & 0xff) << 14
                    | (beta0 as u32 & 0xff) << 6,
                (beta1 as u32 & 0xff) << 22
                    | (beta2 as u32 & 0xff) << 14
                    | (beta3 as u32 & 0xff) << 6,
                (A1 as u32 & 0x00ff_ffff) << 6,
                ((A0 >> 8) as u32 & 0x00ff_ffff) << 6,
                (A0 as u32 & 0xff) << 22
                    | (tot & 0xff) << 14
                    | (week_number & 0xff) << 6,
                (dtls as u32 & 0xff) << 22
                    | (wnlsf & 0xff) << 14
                    | (dn & 0xff) << 6,
                (dtlsf & 0xff) << 22,
            ];
        } else {
            // Subframe 4, page 25
            self.subframes[3] = [
                0x008b_0000 << 6,
                0x4 << 8,
                data_id << 28 | sbf4_page25_sv_id << 22,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ];
        }
        // Subframe 5, page 25
        self.subframes[4] = [
            0x008b_0000 << 6,
            0x5 << 8,
            data_id << 28
                | sbf5_page25_sv_id << 22
                | (toa & 0xff) << 14
                | (wna & 0xff) << 6,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ];
    }
}
