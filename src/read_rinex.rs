use crate::{
    FILE, atof, atoi, constants::*, datetime_t, ephem_t, fclose, fgets, fopen, gpstime_t,
    ionoutc_t, process::date2gps, process::subGpsTime, sqrt, strncmp, strncpy,
};
use std::{fs, path::Path};

///  \brief Replace all 'E' exponential designators to 'D'
///  \param str String in which all occurrences of 'E' are replaced with *  'D'
///  \param len Length of input string in bytes
///  \returns Number of characters replaced
pub fn replaceExpDesignator(str: *mut libc::c_char, len: isize) -> i32 {
    unsafe {
        let mut i: isize = 0;
        let mut n: i32 = 0_i32;
        while i < len {
            if *str.offset(i) as i32 == 'D' as i32 {
                n += 1;
                *str.offset(i) = 'E' as i32 as libc::c_char;
            }
            i += 1;
        }
        n
    }
}
///  \brief Read Ephemeris data from the RINEX Navigation file */
///  \param[out] eph Array of Output SV ephemeris data
///  \param[in] fname File name of the RINEX file
///  \returns Number of sets of ephemerides in the file
pub fn readRinexNavAll(
    eph: &mut [[ephem_t; MAX_SAT]; 15],
    ionoutc: &mut ionoutc_t,
    fname: *const libc::c_char,
) -> usize {
    unsafe {
        let mut str: [libc::c_char; 100] = [0; 100];
        let mut tmp: [libc::c_char; 20] = [0; 20];
        let mut t: datetime_t = datetime_t {
            y: 0,
            m: 0,
            d: 0,
            hh: 0,
            mm: 0,
            sec: 0.,
        };
        let mut g: gpstime_t = gpstime_t { week: 0, sec: 0. };
        let mut g0: gpstime_t = gpstime_t { week: 0, sec: 0. };
        let mut flags: i32 = 0_i32;
        let fp: *mut FILE = fopen(fname, b"rt\0" as *const u8 as *const libc::c_char);
        if fp.is_null() {
            return usize::MAX;
        }
        let mut ieph = 0_usize;
        while ieph < EPHEM_ARRAY_SIZE {
            let mut sv = 0_i32;
            while sv < 32_i32 {
                eph[ieph][sv as usize].vflg = 0_i32;
                sv += 1;
            }
            ieph += 1;
        }
        while !(fgets(str.as_mut_ptr(), 100_i32, fp)).is_null() {
            if strncmp(
                str.as_mut_ptr().offset(60),
                b"END OF HEADER\0" as *const u8 as *const libc::c_char,
                13,
            ) == 0_i32
            {
                break;
            }
            if strncmp(
                str.as_mut_ptr().offset(60),
                b"ION ALPHA\0" as *const u8 as *const libc::c_char,
                9,
            ) == 0_i32
            {
                strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(2), 12);
                tmp[12] = 0_i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12);
                ionoutc.alpha0 = atof(tmp.as_mut_ptr());
                strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(14), 12);
                tmp[12] = 0_i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12);
                ionoutc.alpha1 = atof(tmp.as_mut_ptr());
                strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(26), 12);
                tmp[12] = 0_i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12);
                ionoutc.alpha2 = atof(tmp.as_mut_ptr());
                strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(38), 12);
                tmp[12] = 0_i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12);
                ionoutc.alpha3 = atof(tmp.as_mut_ptr());
                flags |= 0x1_i32;
            } else if strncmp(
                str.as_mut_ptr().offset(60),
                b"ION BETA\0" as *const u8 as *const libc::c_char,
                8,
            ) == 0_i32
            {
                strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(2), 12);
                tmp[12] = 0_i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12);
                ionoutc.beta0 = atof(tmp.as_mut_ptr());
                strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(14), 12);
                tmp[12] = 0_i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12);
                ionoutc.beta1 = atof(tmp.as_mut_ptr());
                strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(26), 12);
                tmp[12] = 0_i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12);
                ionoutc.beta2 = atof(tmp.as_mut_ptr());
                strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(38), 12);
                tmp[12] = 0_i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12);
                ionoutc.beta3 = atof(tmp.as_mut_ptr());
                flags |= 0x1_i32 << 1_i32;
            } else if strncmp(
                str.as_mut_ptr().offset(60),
                b"DELTA-UTC\0" as *const u8 as *const libc::c_char,
                9,
            ) == 0_i32
            {
                strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(3), 19);
                tmp[19] = 0_i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 19);
                ionoutc.A0 = atof(tmp.as_mut_ptr());
                strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(22), 19);
                tmp[19] = 0_i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 19);
                ionoutc.A1 = atof(tmp.as_mut_ptr());
                strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(41), 9);
                tmp[9] = 0_i32 as libc::c_char;
                ionoutc.tot = atoi(tmp.as_mut_ptr());
                strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(50), 9);
                tmp[9] = 0_i32 as libc::c_char;
                ionoutc.wnt = atoi(tmp.as_mut_ptr());
                if ionoutc.tot % 4096_i32 == 0_i32 {
                    flags |= 0x1_i32 << 2_i32;
                }
            } else if strncmp(
                str.as_mut_ptr().offset(60),
                b"LEAP SECONDS\0" as *const u8 as *const libc::c_char,
                12,
            ) == 0_i32
            {
                strncpy(tmp.as_mut_ptr(), str.as_mut_ptr(), 6);
                tmp[6] = 0_i32 as libc::c_char;
                ionoutc.dtls = atoi(tmp.as_mut_ptr());
                flags |= 0x1_i32 << 3_i32;
            }
        }
        ionoutc.vflg = 0_i32;
        if flags == 0xf_i32 {
            ionoutc.vflg = 1_i32;
        }
        g0.week = -1_i32;
        ieph = 0_usize;
        while !(fgets(str.as_mut_ptr(), 100_i32, fp)).is_null() {
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr(), 2);
            tmp[2] = 0_i32 as libc::c_char;
            let sv = atoi(tmp.as_mut_ptr()) - 1_i32;
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(3), 2);
            tmp[2] = 0_i32 as libc::c_char;
            t.y = atoi(tmp.as_mut_ptr()) + 2000_i32;
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(6), 2);
            tmp[2] = 0_i32 as libc::c_char;
            t.m = atoi(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(9), 2);
            tmp[2] = 0_i32 as libc::c_char;
            t.d = atoi(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(12), 2);
            tmp[2] = 0_i32 as libc::c_char;
            t.hh = atoi(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(15), 2);
            tmp[2] = 0_i32 as libc::c_char;
            t.mm = atoi(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(18), 4);
            tmp[2] = 0_i32 as libc::c_char;
            t.sec = atof(tmp.as_mut_ptr());
            date2gps(&t, &mut g);
            if g0.week == -1_i32 {
                g0 = g;
            }
            let dt = subGpsTime(g, g0);
            if dt > 3600.0f64 {
                g0 = g;
                ieph += 1;
                if ieph >= 15 {
                    break;
                }
            }
            eph[ieph][sv as usize].t = t;
            eph[ieph][sv as usize].toc = g;
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(22), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].af0 = atof(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(41), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].af1 = atof(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(60), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].af2 = atof(tmp.as_mut_ptr());
            if (fgets(str.as_mut_ptr(), 100_i32, fp)).is_null() {
                break;
            }
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(3), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].iode = atof(tmp.as_mut_ptr()) as i32;
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(22), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].crs = atof(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(41), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].deltan = atof(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(60), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].m0 = atof(tmp.as_mut_ptr());
            if (fgets(str.as_mut_ptr(), 100_i32, fp)).is_null() {
                break;
            }
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(3), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].cuc = atof(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(22), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].ecc = atof(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(41), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].cus = atof(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(60), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].sqrta = atof(tmp.as_mut_ptr());
            if (fgets(str.as_mut_ptr(), 100_i32, fp)).is_null() {
                break;
            }
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(3), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].toe.sec = atof(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(22), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].cic = atof(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(41), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].omg0 = atof(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(60), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].cis = atof(tmp.as_mut_ptr());
            if (fgets(str.as_mut_ptr(), 100_i32, fp)).is_null() {
                break;
            }
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(3), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].inc0 = atof(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(22), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].crc = atof(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(41), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].aop = atof(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(60), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].omgdot = atof(tmp.as_mut_ptr());
            if (fgets(str.as_mut_ptr(), 100_i32, fp)).is_null() {
                break;
            }
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(3), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].idot = atof(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(22), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].codeL2 = atof(tmp.as_mut_ptr()) as i32;
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(41), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].toe.week = atof(tmp.as_mut_ptr()) as i32;
            if (fgets(str.as_mut_ptr(), 100_i32, fp)).is_null() {
                break;
            }
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(22), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].svhlth = atof(tmp.as_mut_ptr()) as i32;
            if eph[ieph][sv as usize].svhlth > 0_i32 && eph[ieph][sv as usize].svhlth < 32_i32 {
                eph[ieph][sv as usize].svhlth += 32_i32;
            }
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(41), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].tgd = atof(tmp.as_mut_ptr());
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr().offset(60), 19);
            tmp[19] = 0_i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19);
            eph[ieph][sv as usize].iodc = atof(tmp.as_mut_ptr()) as i32;
            if (fgets(str.as_mut_ptr(), 100_i32, fp)).is_null() {
                break;
            }
            eph[ieph][sv as usize].vflg = 1_i32;
            eph[ieph][sv as usize].A = eph[ieph][sv as usize].sqrta * eph[ieph][sv as usize].sqrta;
            eph[ieph][sv as usize].n = sqrt(
                3.986005e14f64
                    / (eph[ieph][sv as usize].A
                        * eph[ieph][sv as usize].A
                        * eph[ieph][sv as usize].A),
            ) + eph[ieph][sv as usize].deltan;
            eph[ieph][sv as usize].sq1e2 =
                sqrt(1.0f64 - eph[ieph][sv as usize].ecc * eph[ieph][sv as usize].ecc);
            eph[ieph][sv as usize].omgkdot = eph[ieph][sv as usize].omgdot - 7.2921151467e-5f64;
        }
        fclose(fp);
        if g0.week >= 0_i32 {
            ieph += 1;
        }
        ieph
    }
}

pub fn parse_f64(num_string: &str) -> Result<f64, std::num::ParseFloatError> {
    num_string.replace('D', "E").parse()
}

pub fn parse_i32(num_string: &str) -> Result<i32, std::num::ParseIntError> {
    num_string.parse()
}
#[allow(clippy::too_many_lines)]
pub fn read_rinex_nav_all(
    data: &mut [[ephem_t; MAX_SAT]; EPHEM_ARRAY_SIZE],
    iono_utc: &mut ionoutc_t,
    fname: &dyn AsRef<Path>,
) -> anyhow::Result<usize> {
    // Ephemeris vec of vec init;
    let mut ieph: usize = 0;
    let mut sv: usize = 0;
    let content = fs::read_to_string(fname)?;
    let mut flags = 0x0;
    // read header
    let lines = content.lines();
    let mut processing_header = true;
    let mut g = gpstime_t::default();
    let mut g0 = gpstime_t::default();
    let mut t = datetime_t::default();
    let mut lines_of_headers: usize = 0;
    for (iline, line) in lines.enumerate() {
        let strncpy = |start, n| &line[start..start + n];
        if line.contains("END OF HEADER") {
            processing_header = false;
            lines_of_headers = iline + 1;
            continue;
        }
        if processing_header {
            if line.contains("ION ALPHA") {
                iono_utc.alpha0 = parse_f64(strncpy(2, 12).trim())?;
                iono_utc.alpha1 = parse_f64(strncpy(14, 12).trim())?;
                iono_utc.alpha2 = parse_f64(strncpy(26, 12).trim())?;
                iono_utc.alpha3 = parse_f64(strncpy(38, 12).trim())?;
                flags |= 0x1;
            }
            if line.contains("ION BETA") {
                iono_utc.beta0 = parse_f64(strncpy(2, 12).trim())?;
                iono_utc.beta1 = parse_f64(strncpy(14, 12).trim())?;
                iono_utc.beta2 = parse_f64(strncpy(26, 12).trim())?;
                iono_utc.beta3 = parse_f64(strncpy(38, 12).trim())?;
                flags |= 0x1 << 1;
            }
            if line.contains("DELTA-UTC") {
                iono_utc.A0 = parse_f64(strncpy(3, 19).trim())?;
                iono_utc.A1 = parse_f64(strncpy(22, 19).trim())?;
                iono_utc.tot = parse_i32(strncpy(41, 9).trim())?;
                iono_utc.wnt = parse_i32(strncpy(50, 9).trim())?;
                if iono_utc.tot % 4096 == 0 {
                    flags |= 0x1 << 2;
                }
            }
            if line.contains("LEAP SECONDS") {
                iono_utc.dtls = parse_i32(strncpy(0, 6).trim())?;
                flags |= 0x1 << 3;
            }
        }
        if !processing_header {
            iono_utc.vflg = 0;
            // Read all Iono/UTC lines
            if flags == 0xF {
                iono_utc.vflg = 1;
            }
            g0.week = -1;
            ieph = 0;
        }
    }
    let lines = content.lines();
    for (iline, line) in lines.skip(lines_of_headers).enumerate() {
        let strncpy = |start, n| &line[start..start + n];
        if iline % 8 == 0 {
            sv = strncpy(0, 2).trim().parse::<usize>()? - 1;
            t.y = strncpy(3, 2).trim().parse::<i32>()? + 2000;
            t.m = strncpy(6, 2).trim().parse::<i32>()?;
            t.d = strncpy(9, 2).trim().parse::<i32>()?;
            t.hh = strncpy(12, 2).trim().parse::<i32>()?;
            t.mm = strncpy(15, 2).trim().parse::<i32>()?;
            t.sec = strncpy(18, 2).trim().parse::<f64>()?;
            date2gps(&t, &mut g);
            // let g = GpsTime::from(&t);

            // if first line of block
            if iline == 0 {
                g0 = g;
            }
            // Check current time of clock
            let dt = subGpsTime(g, g0);
            if dt > SECONDS_IN_HOUR {
                g0 = g;
                ieph += 1; // a new set of ephemerides
                if ieph >= EPHEM_ARRAY_SIZE {
                    break;
                }
            }
            // Date and time
            data[ieph][sv].t = t;
            // SV CLK
            data[ieph][sv].toc = g;
            data[ieph][sv].af0 = parse_f64(strncpy(22, 19).trim())?;
            data[ieph][sv].af1 = parse_f64(strncpy(41, 19).trim())?;
            data[ieph][sv].af2 = parse_f64(strncpy(60, 19).trim())?;
        }
        if iline % 8 == 1 {
            data[ieph][sv].iode = parse_f64(strncpy(3, 19).trim())? as i32;
            data[ieph][sv].crs = parse_f64(strncpy(22, 19).trim())?;
            data[ieph][sv].deltan = parse_f64(strncpy(41, 19).trim())?;
            data[ieph][sv].m0 = parse_f64(strncpy(60, 19).trim())?;
        }
        if iline % 8 == 2 {
            data[ieph][sv].cuc = parse_f64(strncpy(3, 19).trim())?;
            data[ieph][sv].ecc = parse_f64(strncpy(22, 19).trim())?;
            data[ieph][sv].cus = parse_f64(strncpy(41, 19).trim())?;
            data[ieph][sv].sqrta = parse_f64(strncpy(60, 19).trim())?;
        }
        if iline % 8 == 3 {
            data[ieph][sv].toe.sec = parse_f64(strncpy(3, 19).trim())?;
            data[ieph][sv].cic = parse_f64(strncpy(22, 19).trim())?;
            data[ieph][sv].omg0 = parse_f64(strncpy(41, 19).trim())?;
            data[ieph][sv].cis = parse_f64(strncpy(60, 19).trim())?;
        }
        if iline % 8 == 4 {
            data[ieph][sv].inc0 = parse_f64(strncpy(3, 19).trim())?;
            data[ieph][sv].crc = parse_f64(strncpy(22, 19).trim())?;
            data[ieph][sv].aop = parse_f64(strncpy(41, 19).trim())?;
            data[ieph][sv].omgdot = parse_f64(strncpy(60, 19).trim())?;
        }
        if iline % 8 == 5 {
            data[ieph][sv].idot = parse_f64(strncpy(3, 19).trim())?;
            data[ieph][sv].codeL2 = parse_f64(strncpy(22, 19).trim())? as i32;
            data[ieph][sv].toe.week = parse_f64(strncpy(41, 19).trim())? as i32;
        }
        if iline % 8 == 6 {
            data[ieph][sv].svhlth = parse_f64(strncpy(22, 19).trim())? as i32;
            if data[ieph][sv].svhlth > 0 && data[ieph][sv].svhlth < 32 {
                data[ieph][sv].svhlth += 32;
            }
            data[ieph][sv].tgd = parse_f64(strncpy(41, 19).trim())?;
            data[ieph][sv].iodc = parse_f64(strncpy(60, 19).trim())? as i32;
        }
        if iline % 8 == 7 {
            // Set valid flag
            data[ieph][sv].vflg = 1;

            // Update the working variables
            data[ieph][sv].A = data[ieph][sv].sqrta * data[ieph][sv].sqrta;
            data[ieph][sv].n =
                (GM_EARTH / (data[ieph][sv].A * data[ieph][sv].A * data[ieph][sv].A)).sqrt()
                    + data[ieph][sv].deltan;
            data[ieph][sv].sq1e2 = (1.0 - data[ieph][sv].ecc * data[ieph][sv].ecc).sqrt();
            data[ieph][sv].omgkdot = data[ieph][sv].omgdot - OMEGA_EARTH;
        }
    }
    if g0.week >= 0 {
        ieph += 1;
    }
    Ok(ieph)
}
