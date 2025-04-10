use crate::strchr;

pub static mut opterr: i32 = 1 as i32;

pub static mut optind: i32 = 1 as i32;

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
            optreset = 0 as i32;
            if optind >= nargc || {
                place = *nargv.offset(optind as isize);
                *place as i32 != '-' as i32
            } {
                place = b"\0" as *const u8 as *const libc::c_char as *mut libc::c_char;
                return -(1 as i32);
            }
            if *place.offset(1) as i32 != 0 && {
                place = place.offset(1);
                *place as i32 == '-' as i32
            } {
                optind += 1;
                place = b"\0" as *const u8 as *const libc::c_char as *mut libc::c_char;
                return -(1 as i32);
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
                return -(1 as i32);
            }
            if *place == 0 {
                optind += 1;
            }
            if opterr != 0 && *ostr as i32 != ':' as i32 {
                println!("illegal option -- {}\n\0", optopt,);
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
                        println!("option requires an argument -- {}\n\0", optopt,);
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
