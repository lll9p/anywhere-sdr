use crate::{
    FILE, atof, atoi, constants::*, date2gps, datetime_t, ephem_t, fclose, fgets, fopen, gpstime_t,
    ionoutc_t, sqrt, strncmp, strncpy, subGpsTime,
};

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
        while ieph < 15 {
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
