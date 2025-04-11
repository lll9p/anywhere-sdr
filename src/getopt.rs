use crate::{
    atof, atoi, constants::USER_MOTION_SIZE, date2gps, datetime::gpstime_t, datetime_t, exit,
    gmtime, ionoutc_t, llh2xyz, sscanf, strchr, strcpy, strncmp, time, time_t, tm, usage, utils::*,
};

pub static mut opterr: i32 = 1_i32;
pub static mut optind: i32 = 1_i32;
pub static mut optopt: i32 = 0;
pub static mut optreset: i32 = 0;
pub static mut optarg: *mut libc::c_char = 0 as *const libc::c_char as *mut libc::c_char;

pub fn getopt(
    mut nargc: i32,
    mut nargv: *const *mut libc::c_char,
    mut ostr: *const libc::c_char,
) -> i32 {
    unsafe {
        static mut place: *mut libc::c_char =
            b"\0" as *const u8 as *const libc::c_char as *mut libc::c_char;
        let mut oli: *const libc::c_char = std::ptr::null::<libc::c_char>();
        if optreset != 0 || *place == 0 {
            optreset = 0_i32;
            if optind >= nargc || {
                place = *nargv.offset(optind as isize);
                *place as i32 != '-' as i32
            } {
                place = b"\0" as *const u8 as *const libc::c_char as *mut libc::c_char;
                return -1_i32;
            }
            if *place.offset(1) as i32 != 0 && {
                place = place.offset(1);
                *place as i32 == '-' as i32
            } {
                optind += 1;
                place = b"\0" as *const u8 as *const libc::c_char as *mut libc::c_char;
                return -1_i32;
            }
        }
        let fresh0 = place;
        place = place.offset(1);
        optopt = *fresh0 as i32;
        if optopt == ':' as i32 || {
            oli = strchr(ostr, optopt);
            oli.is_null()
        } {
            if optopt == '-' as i32 {
                return -1_i32;
            }
            if *place == 0 {
                optind += 1;
            }
            if opterr != 0 && *ostr as i32 != ':' as i32 {
                println!("illegal option -- {}", optopt,);
            }
            return '?' as i32;
        }
        oli = oli.offset(1);
        if *oli as i32 != ':' as i32 {
            optarg = std::ptr::null_mut::<libc::c_char>();
            if *place == 0 {
                optind += 1;
            }
        } else {
            if *place != 0 {
                optarg = place;
            } else {
                optind += 1;
                if nargc <= optind {
                    place = b"\0" as *const u8 as *const libc::c_char as *mut libc::c_char;
                    if *ostr as i32 == ':' as i32 {
                        return ':' as i32;
                    }
                    if opterr != 0 {
                        println!("option requires an argument -- {}", optopt);
                    }
                    return '?' as i32;
                } else {
                    optarg = *nargv.offset(optind as isize);
                }
            }
            place = b"\0" as *const u8 as *const libc::c_char as *mut libc::c_char;
            optind += 1;
        }
        optopt
    }
}
#[allow(clippy::too_many_arguments)]
pub unsafe fn loop_through_opts(
    mut argc: i32,
    mut argv: *mut *mut libc::c_char,
    navfile: &mut [libc::c_char; 100],
    umfile: &mut [libc::c_char; 100],
    nmeaGGA: &mut i32,
    umLLH: &mut i32,
    staticLocationMode: &mut i32,
    xyz: &mut [[f64; 3]; USER_MOTION_SIZE],
    llh: &mut [f64; 3],
    outfile: &mut [libc::c_char; 100],
    samp_freq: &mut f64,
    data_format: &mut i32,
    ionoutc: &mut ionoutc_t,
    timeoverwrite: &mut i32,
    t0: &mut datetime_t,
    g0: &mut gpstime_t,
    duration: &mut f64,
    fixed_gain: &mut i32,
    path_loss_enable: &mut i32,
    verb: &mut i32,
) {
    unsafe {
        let mut result: i32 = 0;
        loop {
            result = getopt(
                argc,
                argv as *const *mut libc::c_char,
                b"e:u:x:g:c:l:o:s:b:L:T:t:d:ipv\0" as *const u8 as *const libc::c_char,
            );
            if result == -1_i32 {
                break;
            }
            let mut current_block_85: u64;
            match result {
                101 => {
                    strcpy(navfile.as_mut_ptr(), optarg);
                    current_block_85 = 2750570471926810434;
                }
                117 => {
                    strcpy(umfile.as_mut_ptr(), optarg);
                    *nmeaGGA = 0_i32;
                    *umLLH = 0_i32;
                    current_block_85 = 2750570471926810434;
                }
                120 => {
                    strcpy(umfile.as_mut_ptr(), optarg);
                    *umLLH = 1_i32;
                    current_block_85 = 2750570471926810434;
                }
                103 => {
                    strcpy(umfile.as_mut_ptr(), optarg);
                    *nmeaGGA = 1_i32;
                    current_block_85 = 2750570471926810434;
                }
                99 => {
                    *staticLocationMode = 1_i32;
                    sscanf(
                        optarg,
                        b"%lf,%lf,%lf\0" as *const u8 as *const libc::c_char,
                        &mut *(*xyz.as_mut_ptr().offset(0)).as_mut_ptr().offset(0) as *mut f64,
                        &mut *(*xyz.as_mut_ptr().offset(0)).as_mut_ptr().offset(1) as *mut f64,
                        &mut *(*xyz.as_mut_ptr().offset(0)).as_mut_ptr().offset(2) as *mut f64,
                    );
                    current_block_85 = 2750570471926810434;
                }
                108 => {
                    *staticLocationMode = 1_i32;
                    sscanf(
                        optarg,
                        b"%lf,%lf,%lf\0" as *const u8 as *const libc::c_char,
                        &mut *llh.as_mut_ptr().offset(0) as *mut f64,
                        &mut *llh.as_mut_ptr().offset(1) as *mut f64,
                        &mut *llh.as_mut_ptr().offset(2) as *mut f64,
                    );
                    llh[0] /= 57.2957795131f64;
                    llh[1] /= 57.2957795131f64;
                    llh2xyz(llh, &mut xyz[0]);
                    current_block_85 = 2750570471926810434;
                }
                111 => {
                    strcpy(outfile.as_mut_ptr(), optarg);
                    current_block_85 = 2750570471926810434;
                }
                115 => {
                    *samp_freq = atof(optarg);
                    if *samp_freq < 1.0e6f64 {
                        eprintln!("ERROR: Invalid sampling frequency.\n");
                        exit(1_i32);
                    }
                    current_block_85 = 2750570471926810434;
                }
                98 => {
                    *data_format = atoi(optarg);
                    if *data_format != 1_i32 && *data_format != 8_i32 && *data_format != 16_i32 {
                        eprintln!("ERROR: Invalid I/Q data format.\n");
                        exit(1_i32);
                    }
                    current_block_85 = 2750570471926810434;
                }
                76 => {
                    ionoutc.leapen = 1_i32;
                    sscanf(
                        optarg,
                        b"%d,%d,%d\0" as *const u8 as *const libc::c_char,
                        &mut ionoutc.wnlsf as *mut i32,
                        &mut ionoutc.dn as *mut i32,
                        &mut ionoutc.dtlsf as *mut i32,
                    );
                    // original gps-sdr-sim logical mistake
                    if ionoutc.dn < 1_i32 || ionoutc.dn > 7_i32 {
                        eprintln!("ERROR: Invalid GPS day number");
                        exit(1_i32);
                    }
                    if ionoutc.wnlsf < 0_i32 {
                        eprintln!("ERROR: Invalid GPS week number");
                        exit(1_i32);
                    }
                    // original gps-sdr-sim logical mistake
                    if ionoutc.dtlsf < -128_i32 || ionoutc.dtlsf > 127_i32 {
                        eprintln!("ERROR: Invalid delta leap second");
                        exit(1_i32);
                    }
                    current_block_85 = 2750570471926810434;
                }
                84 => {
                    *timeoverwrite = 1_i32;
                    if strncmp(
                        optarg,
                        b"now\0" as *const u8 as *const libc::c_char,
                        3_i32 as u32,
                    ) == 0_i32
                    {
                        let mut timer: time_t = 0;
                        let mut gmt: *mut tm = std::ptr::null_mut::<tm>();
                        time(&mut timer);
                        gmt = gmtime(&timer);
                        t0.y = (*gmt).tm_year + 1900_i32;
                        t0.m = (*gmt).tm_mon + 1_i32;
                        t0.d = (*gmt).tm_mday;
                        t0.hh = (*gmt).tm_hour;
                        t0.mm = (*gmt).tm_min;
                        t0.sec = (*gmt).tm_sec as f64;
                        date2gps(t0, g0);
                        current_block_85 = 2750570471926810434;
                    } else {
                        current_block_85 = 4676144417340510455;
                    }
                }
                116 => {
                    current_block_85 = 4676144417340510455;
                }
                100 => {
                    *duration = atof(optarg);
                    current_block_85 = 2750570471926810434;
                }
                105 => {
                    ionoutc.enable = 0_i32;
                    current_block_85 = 2750570471926810434;
                }
                112 => {
                    if optind < argc
                        && *(*argv.offset(optind as isize)).offset(0) as i32 != '-' as i32
                    {
                        *fixed_gain = atoi(*argv.offset(optind as isize));
                        if !(1_i32..=128_i32).contains(fixed_gain) {
                            eprintln!("ERROR: Fixed gain must be between 1 and 128.\n");
                            exit(1_i32);
                        }
                        optind += 1;
                    }
                    *path_loss_enable = 0_i32;
                    current_block_85 = 2750570471926810434;
                }
                118 => {
                    *verb = 1_i32;
                    current_block_85 = 2750570471926810434;
                }
                58 | 63 => {
                    usage();
                    exit(1_i32);
                }
                _ => {
                    current_block_85 = 2750570471926810434;
                }
            }
            if current_block_85 == 4676144417340510455 {
                sscanf(
                    optarg,
                    b"%d/%d/%d,%d:%d:%lf\0" as *const u8 as *const libc::c_char,
                    &mut t0.y as *mut i32,
                    &mut t0.m as *mut i32,
                    &mut t0.d as *mut i32,
                    &mut t0.hh as *mut i32,
                    &mut t0.mm as *mut i32,
                    &mut t0.sec as *mut f64,
                );
                if t0.y <= 1980_i32
                    || t0.m < 1_i32
                    || t0.m > 12_i32
                    || t0.d < 1_i32
                    || t0.d > 31_i32
                    || t0.hh < 0_i32
                    || t0.hh > 23_i32
                    || t0.mm < 0_i32
                    || t0.mm > 59_i32
                    || t0.sec < 0.0f64
                    || t0.sec >= 60.0f64
                {
                    eprintln!("ERROR: Invalid date and time.\n");
                    exit(1_i32);
                }
                t0.sec = floor(t0.sec);
                date2gps(t0, g0);
            }
        }
    }
}
