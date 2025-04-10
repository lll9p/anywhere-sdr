use crate::{
    FILE, atof, atoi, date2gps, datetime_t, ephem_t, fclose, fgets, fopen, gpstime_t, ionoutc_t,
    sqrt, strncmp, strncpy, subGpsTime,
};

pub fn replaceExpDesignator(mut str: *mut libc::c_char, mut len: i32) -> i32 {
    unsafe {
        let mut i: i32 = 0;
        let mut n: i32 = 0 as i32;
        i = 0 as i32;
        while i < len {
            if *str.offset(i as isize) as i32 == 'D' as i32 {
                n += 1;
                *str.offset(i as isize) = 'E' as i32 as libc::c_char;
            }
            i += 1;
        }
        n
    }
}

pub fn readRinexNavAll(
    mut eph: *mut [ephem_t; 32],
    mut ionoutc: *mut ionoutc_t,
    mut fname: *const libc::c_char,
) -> i32 {
    unsafe {
        let mut fp: *mut FILE = std::ptr::null_mut::<FILE>();
        let mut ieph: i32 = 0;
        let mut sv: i32 = 0;
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
        let mut dt: f64 = 0.;
        let mut flags: i32 = 0 as i32;
        fp = fopen(fname, b"rt\0" as *const u8 as *const libc::c_char);
        if fp.is_null() {
            return -(1 as i32);
        }
        ieph = 0 as i32;
        while ieph < 15 as i32 {
            sv = 0 as i32;
            while sv < 32 as i32 {
                (*eph.offset(ieph as isize))[sv as usize].vflg = 0 as i32;
                sv += 1;
            }
            ieph += 1;
        }
        while !(fgets(str.as_mut_ptr(), 100 as i32, fp)).is_null() {
            if strncmp(
                str.as_mut_ptr().offset(60),
                b"END OF HEADER\0" as *const u8 as *const libc::c_char,
                13 as i32 as u32,
            ) == 0 as i32
            {
                break;
            }
            if strncmp(
                str.as_mut_ptr().offset(60),
                b"ION ALPHA\0" as *const u8 as *const libc::c_char,
                9 as i32 as u32,
            ) == 0 as i32
            {
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(2),
                    12 as i32 as u32,
                );
                tmp[12 as i32 as usize] = 0 as i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12 as i32);
                (*ionoutc).alpha0 = atof(tmp.as_mut_ptr());
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(14),
                    12 as i32 as u32,
                );
                tmp[12 as i32 as usize] = 0 as i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12 as i32);
                (*ionoutc).alpha1 = atof(tmp.as_mut_ptr());
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(26),
                    12 as i32 as u32,
                );
                tmp[12 as i32 as usize] = 0 as i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12 as i32);
                (*ionoutc).alpha2 = atof(tmp.as_mut_ptr());
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(38),
                    12 as i32 as u32,
                );
                tmp[12 as i32 as usize] = 0 as i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12 as i32);
                (*ionoutc).alpha3 = atof(tmp.as_mut_ptr());
                flags |= 0x1 as i32;
            } else if strncmp(
                str.as_mut_ptr().offset(60),
                b"ION BETA\0" as *const u8 as *const libc::c_char,
                8 as i32 as u32,
            ) == 0 as i32
            {
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(2),
                    12 as i32 as u32,
                );
                tmp[12 as i32 as usize] = 0 as i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12 as i32);
                (*ionoutc).beta0 = atof(tmp.as_mut_ptr());
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(14),
                    12 as i32 as u32,
                );
                tmp[12 as i32 as usize] = 0 as i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12 as i32);
                (*ionoutc).beta1 = atof(tmp.as_mut_ptr());
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(26),
                    12 as i32 as u32,
                );
                tmp[12 as i32 as usize] = 0 as i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12 as i32);
                (*ionoutc).beta2 = atof(tmp.as_mut_ptr());
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(38),
                    12 as i32 as u32,
                );
                tmp[12 as i32 as usize] = 0 as i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12 as i32);
                (*ionoutc).beta3 = atof(tmp.as_mut_ptr());
                flags |= (0x1 as i32) << 1 as i32;
            } else if strncmp(
                str.as_mut_ptr().offset(60),
                b"DELTA-UTC\0" as *const u8 as *const libc::c_char,
                9 as i32 as u32,
            ) == 0 as i32
            {
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(3),
                    19 as i32 as u32,
                );
                tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
                (*ionoutc).A0 = atof(tmp.as_mut_ptr());
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(22),
                    19 as i32 as u32,
                );
                tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
                (*ionoutc).A1 = atof(tmp.as_mut_ptr());
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(41),
                    9 as i32 as u32,
                );
                tmp[9 as i32 as usize] = 0 as i32 as libc::c_char;
                (*ionoutc).tot = atoi(tmp.as_mut_ptr());
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(50),
                    9 as i32 as u32,
                );
                tmp[9 as i32 as usize] = 0 as i32 as libc::c_char;
                (*ionoutc).wnt = atoi(tmp.as_mut_ptr());
                if (*ionoutc).tot % 4096 as i32 == 0 as i32 {
                    flags |= (0x1 as i32) << 2 as i32;
                }
            } else if strncmp(
                str.as_mut_ptr().offset(60),
                b"LEAP SECONDS\0" as *const u8 as *const libc::c_char,
                12 as i32 as u32,
            ) == 0 as i32
            {
                strncpy(tmp.as_mut_ptr(), str.as_mut_ptr(), 6 as i32 as u32);
                tmp[6 as i32 as usize] = 0 as i32 as libc::c_char;
                (*ionoutc).dtls = atoi(tmp.as_mut_ptr());
                flags |= (0x1 as i32) << 3 as i32;
            }
        }
        (*ionoutc).vflg = 0 as i32;
        if flags == 0xf as i32 {
            (*ionoutc).vflg = 1 as i32;
        }
        g0.week = -(1 as i32);
        ieph = 0 as i32;
        while !(fgets(str.as_mut_ptr(), 100 as i32, fp)).is_null() {
            strncpy(tmp.as_mut_ptr(), str.as_mut_ptr(), 2 as i32 as u32);
            tmp[2 as i32 as usize] = 0 as i32 as libc::c_char;
            sv = atoi(tmp.as_mut_ptr()) - 1 as i32;
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(3),
                2 as i32 as u32,
            );
            tmp[2 as i32 as usize] = 0 as i32 as libc::c_char;
            t.y = atoi(tmp.as_mut_ptr()) + 2000 as i32;
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(6),
                2 as i32 as u32,
            );
            tmp[2 as i32 as usize] = 0 as i32 as libc::c_char;
            t.m = atoi(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(9),
                2 as i32 as u32,
            );
            tmp[2 as i32 as usize] = 0 as i32 as libc::c_char;
            t.d = atoi(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(12),
                2 as i32 as u32,
            );
            tmp[2 as i32 as usize] = 0 as i32 as libc::c_char;
            t.hh = atoi(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(15),
                2 as i32 as u32,
            );
            tmp[2 as i32 as usize] = 0 as i32 as libc::c_char;
            t.mm = atoi(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(18),
                4 as i32 as u32,
            );
            tmp[2 as i32 as usize] = 0 as i32 as libc::c_char;
            t.sec = atof(tmp.as_mut_ptr());
            date2gps(&mut t, &mut g);
            if g0.week == -(1 as i32) {
                g0 = g;
            }
            dt = subGpsTime(g, g0);
            if dt > 3600.0f64 {
                g0 = g;
                ieph += 1;
                if ieph >= 15 as i32 {
                    break;
                }
            }
            (*eph.offset(ieph as isize))[sv as usize].t = t;
            (*eph.offset(ieph as isize))[sv as usize].toc = g;
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(22),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].af0 = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(41),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].af1 = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(60),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].af2 = atof(tmp.as_mut_ptr());
            if (fgets(str.as_mut_ptr(), 100 as i32, fp)).is_null() {
                break;
            }
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(3),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].iode = atof(tmp.as_mut_ptr()) as i32;
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(22),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].crs = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(41),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].deltan = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(60),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].m0 = atof(tmp.as_mut_ptr());
            if (fgets(str.as_mut_ptr(), 100 as i32, fp)).is_null() {
                break;
            }
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(3),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].cuc = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(22),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].ecc = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(41),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].cus = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(60),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].sqrta = atof(tmp.as_mut_ptr());
            if (fgets(str.as_mut_ptr(), 100 as i32, fp)).is_null() {
                break;
            }
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(3),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].toe.sec = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(22),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].cic = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(41),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].omg0 = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(60),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].cis = atof(tmp.as_mut_ptr());
            if (fgets(str.as_mut_ptr(), 100 as i32, fp)).is_null() {
                break;
            }
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(3),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].inc0 = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(22),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].crc = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(41),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].aop = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(60),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].omgdot = atof(tmp.as_mut_ptr());
            if (fgets(str.as_mut_ptr(), 100 as i32, fp)).is_null() {
                break;
            }
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(3),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].idot = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(22),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].codeL2 = atof(tmp.as_mut_ptr()) as i32;
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(41),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].toe.week = atof(tmp.as_mut_ptr()) as i32;
            if (fgets(str.as_mut_ptr(), 100 as i32, fp)).is_null() {
                break;
            }
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(22),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].svhlth = atof(tmp.as_mut_ptr()) as i32;
            if (*eph.offset(ieph as isize))[sv as usize].svhlth > 0 as i32
                && (*eph.offset(ieph as isize))[sv as usize].svhlth < 32 as i32
            {
                (*eph.offset(ieph as isize))[sv as usize].svhlth += 32 as i32;
            }
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(41),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].tgd = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(60),
                19 as i32 as u32,
            );
            tmp[19 as i32 as usize] = 0 as i32 as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as i32);
            (*eph.offset(ieph as isize))[sv as usize].iodc = atof(tmp.as_mut_ptr()) as i32;
            if (fgets(str.as_mut_ptr(), 100 as i32, fp)).is_null() {
                break;
            }
            (*eph.offset(ieph as isize))[sv as usize].vflg = 1 as i32;
            (*eph.offset(ieph as isize))[sv as usize].A = (*eph.offset(ieph as isize))[sv as usize]
                .sqrta
                * (*eph.offset(ieph as isize))[sv as usize].sqrta;
            (*eph.offset(ieph as isize))[sv as usize].n =
                sqrt(
                    3.986005e14f64
                        / ((*eph.offset(ieph as isize))[sv as usize].A
                            * (*eph.offset(ieph as isize))[sv as usize].A
                            * (*eph.offset(ieph as isize))[sv as usize].A),
                ) + (*eph.offset(ieph as isize))[sv as usize].deltan;
            (*eph.offset(ieph as isize))[sv as usize].sq1e2 = sqrt(
                1.0f64
                    - (*eph.offset(ieph as isize))[sv as usize].ecc
                        * (*eph.offset(ieph as isize))[sv as usize].ecc,
            );
            (*eph.offset(ieph as isize))[sv as usize].omgkdot =
                (*eph.offset(ieph as isize))[sv as usize].omgdot - 7.2921151467e-5f64;
        }
        fclose(fp);
        if g0.week >= 0 as i32 {
            ieph += 1 as i32;
        }
        ieph
    }
}
