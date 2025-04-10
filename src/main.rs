#![allow(
    dead_code,
    mutable_transmutes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut,
    static_mut_refs
)]
#![feature(extern_types)]
unsafe extern "C" {
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    static mut stdout: *mut FILE;

    fn fclose(__stream: *mut FILE) -> libc::c_int;
    fn fflush(__stream: *mut FILE) -> libc::c_int;
    fn fopen(_: *const libc::c_char, _: *const libc::c_char) -> *mut FILE;

    fn sscanf(_: *const libc::c_char, _: *const libc::c_char, _: ...) -> libc::c_int;
    fn fgets(__s: *mut libc::c_char, __n: libc::c_int, __stream: *mut FILE) -> *mut libc::c_char;
    fn fwrite(
        _: *const libc::c_void,
        _: libc::c_ulong,
        _: libc::c_ulong,
        _: *mut FILE,
    ) -> libc::c_ulong;
    fn atof(__nptr: *const libc::c_char) -> libc::c_double;
    fn atoi(__nptr: *const libc::c_char) -> libc::c_int;
    fn calloc(_: libc::c_ulong, _: libc::c_ulong) -> *mut libc::c_void;
    fn free(_: *mut libc::c_void);
    fn exit(_: libc::c_int) -> !;
    fn strcpy(_: *mut libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    fn strncpy(_: *mut libc::c_char, _: *const libc::c_char, _: libc::c_ulong)
    -> *mut libc::c_char;
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    fn strncmp(_: *const libc::c_char, _: *const libc::c_char, _: libc::c_ulong) -> libc::c_int;
    fn strchr(_: *const libc::c_char, _: libc::c_int) -> *mut libc::c_char;
    fn strtok(_: *mut libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    fn atan2(_: libc::c_double, _: libc::c_double) -> libc::c_double;
    fn cos(_: libc::c_double) -> libc::c_double;
    fn sin(_: libc::c_double) -> libc::c_double;
    fn pow(_: libc::c_double, _: libc::c_double) -> libc::c_double;
    fn sqrt(_: libc::c_double) -> libc::c_double;
    fn fabs(_: libc::c_double) -> libc::c_double;
    fn floor(_: libc::c_double) -> libc::c_double;
    fn round(_: libc::c_double) -> libc::c_double;
    fn clock() -> clock_t;
    fn time(__timer: *mut time_t) -> time_t;
    fn gmtime(__timer: *const time_t) -> *mut tm;
}
pub type size_t = libc::c_ulong;
pub type __off_t = libc::c_long;
pub type __off64_t = libc::c_long;
pub type __clock_t = libc::c_long;
pub type __time_t = libc::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: libc::c_int,
    pub _IO_read_ptr: *mut libc::c_char,
    pub _IO_read_end: *mut libc::c_char,
    pub _IO_read_base: *mut libc::c_char,
    pub _IO_write_base: *mut libc::c_char,
    pub _IO_write_ptr: *mut libc::c_char,
    pub _IO_write_end: *mut libc::c_char,
    pub _IO_buf_base: *mut libc::c_char,
    pub _IO_buf_end: *mut libc::c_char,
    pub _IO_save_base: *mut libc::c_char,
    pub _IO_backup_base: *mut libc::c_char,
    pub _IO_save_end: *mut libc::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: libc::c_int,
    pub _flags2: libc::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: libc::c_ushort,
    pub _vtable_offset: libc::c_schar,
    pub _shortbuf: [libc::c_char; 1],
    pub _lock: *mut libc::c_void,
    pub _offset: __off64_t,
    pub _codecvt: *mut _IO_codecvt,
    pub _wide_data: *mut _IO_wide_data,
    pub _freeres_list: *mut _IO_FILE,
    pub _freeres_buf: *mut libc::c_void,
    pub __pad5: size_t,
    pub _mode: libc::c_int,
    pub _unused2: [libc::c_char; 20],
}
pub type _IO_lock_t = ();
pub type FILE = _IO_FILE;
pub type clock_t = __clock_t;
pub type time_t = __time_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tm {
    pub tm_sec: libc::c_int,
    pub tm_min: libc::c_int,
    pub tm_hour: libc::c_int,
    pub tm_mday: libc::c_int,
    pub tm_mon: libc::c_int,
    pub tm_year: libc::c_int,
    pub tm_wday: libc::c_int,
    pub tm_yday: libc::c_int,
    pub tm_isdst: libc::c_int,
    pub tm_gmtoff: libc::c_long,
    pub tm_zone: *const libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct gpstime_t {
    pub week: libc::c_int,
    pub sec: libc::c_double,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct datetime_t {
    pub y: libc::c_int,
    pub m: libc::c_int,
    pub d: libc::c_int,
    pub hh: libc::c_int,
    pub mm: libc::c_int,
    pub sec: libc::c_double,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ephem_t {
    pub vflg: libc::c_int,
    pub t: datetime_t,
    pub toc: gpstime_t,
    pub toe: gpstime_t,
    pub iodc: libc::c_int,
    pub iode: libc::c_int,
    pub deltan: libc::c_double,
    pub cuc: libc::c_double,
    pub cus: libc::c_double,
    pub cic: libc::c_double,
    pub cis: libc::c_double,
    pub crc: libc::c_double,
    pub crs: libc::c_double,
    pub ecc: libc::c_double,
    pub sqrta: libc::c_double,
    pub m0: libc::c_double,
    pub omg0: libc::c_double,
    pub inc0: libc::c_double,
    pub aop: libc::c_double,
    pub omgdot: libc::c_double,
    pub idot: libc::c_double,
    pub af0: libc::c_double,
    pub af1: libc::c_double,
    pub af2: libc::c_double,
    pub tgd: libc::c_double,
    pub svhlth: libc::c_int,
    pub codeL2: libc::c_int,
    pub n: libc::c_double,
    pub sq1e2: libc::c_double,
    pub A: libc::c_double,
    pub omgkdot: libc::c_double,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ionoutc_t {
    pub enable: libc::c_int,
    pub vflg: libc::c_int,
    pub alpha0: libc::c_double,
    pub alpha1: libc::c_double,
    pub alpha2: libc::c_double,
    pub alpha3: libc::c_double,
    pub beta0: libc::c_double,
    pub beta1: libc::c_double,
    pub beta2: libc::c_double,
    pub beta3: libc::c_double,
    pub A0: libc::c_double,
    pub A1: libc::c_double,
    pub dtls: libc::c_int,
    pub tot: libc::c_int,
    pub wnt: libc::c_int,
    pub dtlsf: libc::c_int,
    pub dn: libc::c_int,
    pub wnlsf: libc::c_int,
    pub leapen: libc::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct range_t {
    pub g: gpstime_t,
    pub range: libc::c_double,
    pub rate: libc::c_double,
    pub d: libc::c_double,
    pub azel: [libc::c_double; 2],
    pub iono_delay: libc::c_double,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct channel_t {
    pub prn: libc::c_int,
    pub ca: [libc::c_int; 1023],
    pub f_carr: libc::c_double,
    pub f_code: libc::c_double,
    pub carr_phase: libc::c_uint,
    pub carr_phasestep: libc::c_int,
    pub code_phase: libc::c_double,
    pub g0: gpstime_t,
    pub sbf: [[libc::c_ulong; 10]; 5],
    pub dwrd: [libc::c_ulong; 60],
    pub iword: libc::c_int,
    pub ibit: libc::c_int,
    pub icode: libc::c_int,
    pub dataBit: libc::c_int,
    pub codeCA: libc::c_int,
    pub azel: [libc::c_double; 2],
    pub rho0: range_t,
}
#[unsafe(no_mangle)]
pub static mut opterr: libc::c_int = 1 as libc::c_int;
#[unsafe(no_mangle)]
pub static mut optind: libc::c_int = 1 as libc::c_int;
#[unsafe(no_mangle)]
pub static mut optopt: libc::c_int = 0;
#[unsafe(no_mangle)]
pub static mut optreset: libc::c_int = 0;
#[unsafe(no_mangle)]
pub static mut optarg: *mut libc::c_char = 0 as *const libc::c_char as *mut libc::c_char;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getopt(
    mut nargc: libc::c_int,
    mut nargv: *const *mut libc::c_char,
    mut ostr: *const libc::c_char,
) -> libc::c_int {
    unsafe {
        static mut place: *mut libc::c_char =
            b"\0" as *const u8 as *const libc::c_char as *mut libc::c_char;
        let mut oli: *const libc::c_char = std::ptr::null::<libc::c_char>();
        if optreset != 0 || *place == 0 {
            optreset = 0 as libc::c_int;
            if optind >= nargc || {
                place = *nargv.offset(optind as isize);
                *place as libc::c_int != '-' as i32
            } {
                place = b"\0" as *const u8 as *const libc::c_char as *mut libc::c_char;
                return -(1 as libc::c_int);
            }
            if *place.offset(1 as libc::c_int as isize) as libc::c_int != 0 && {
                place = place.offset(1);
                *place as libc::c_int == '-' as i32
            } {
                optind += 1;
                place = b"\0" as *const u8 as *const libc::c_char as *mut libc::c_char;
                return -(1 as libc::c_int);
            }
        }
        let fresh0 = place;
        place = place.offset(1);
        optopt = *fresh0 as libc::c_int;
        if optopt == ':' as i32 || {
            oli = strchr(ostr, optopt);
            oli.is_null()
        } {
            if optopt == '-' as i32 {
                return -(1 as libc::c_int);
            }
            if *place == 0 {
                optind += 1;
            }
            if opterr != 0 && *ostr as libc::c_int != ':' as i32 {
                println!("illegal option -- {}\n\0", optopt,);
            }
            return '?' as i32;
        }
        oli = oli.offset(1);
        if *oli as libc::c_int != ':' as i32 {
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
                    if *ostr as libc::c_int == ':' as i32 {
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
#[unsafe(no_mangle)]
pub static mut sinTable512: [libc::c_int; 512] = [
    2 as libc::c_int,
    5 as libc::c_int,
    8 as libc::c_int,
    11 as libc::c_int,
    14 as libc::c_int,
    17 as libc::c_int,
    20 as libc::c_int,
    23 as libc::c_int,
    26 as libc::c_int,
    29 as libc::c_int,
    32 as libc::c_int,
    35 as libc::c_int,
    38 as libc::c_int,
    41 as libc::c_int,
    44 as libc::c_int,
    47 as libc::c_int,
    50 as libc::c_int,
    53 as libc::c_int,
    56 as libc::c_int,
    59 as libc::c_int,
    62 as libc::c_int,
    65 as libc::c_int,
    68 as libc::c_int,
    71 as libc::c_int,
    74 as libc::c_int,
    77 as libc::c_int,
    80 as libc::c_int,
    83 as libc::c_int,
    86 as libc::c_int,
    89 as libc::c_int,
    91 as libc::c_int,
    94 as libc::c_int,
    97 as libc::c_int,
    100 as libc::c_int,
    103 as libc::c_int,
    105 as libc::c_int,
    108 as libc::c_int,
    111 as libc::c_int,
    114 as libc::c_int,
    116 as libc::c_int,
    119 as libc::c_int,
    122 as libc::c_int,
    125 as libc::c_int,
    127 as libc::c_int,
    130 as libc::c_int,
    132 as libc::c_int,
    135 as libc::c_int,
    138 as libc::c_int,
    140 as libc::c_int,
    143 as libc::c_int,
    145 as libc::c_int,
    148 as libc::c_int,
    150 as libc::c_int,
    153 as libc::c_int,
    155 as libc::c_int,
    157 as libc::c_int,
    160 as libc::c_int,
    162 as libc::c_int,
    164 as libc::c_int,
    167 as libc::c_int,
    169 as libc::c_int,
    171 as libc::c_int,
    173 as libc::c_int,
    176 as libc::c_int,
    178 as libc::c_int,
    180 as libc::c_int,
    182 as libc::c_int,
    184 as libc::c_int,
    186 as libc::c_int,
    188 as libc::c_int,
    190 as libc::c_int,
    192 as libc::c_int,
    194 as libc::c_int,
    196 as libc::c_int,
    198 as libc::c_int,
    200 as libc::c_int,
    202 as libc::c_int,
    204 as libc::c_int,
    205 as libc::c_int,
    207 as libc::c_int,
    209 as libc::c_int,
    210 as libc::c_int,
    212 as libc::c_int,
    214 as libc::c_int,
    215 as libc::c_int,
    217 as libc::c_int,
    218 as libc::c_int,
    220 as libc::c_int,
    221 as libc::c_int,
    223 as libc::c_int,
    224 as libc::c_int,
    225 as libc::c_int,
    227 as libc::c_int,
    228 as libc::c_int,
    229 as libc::c_int,
    230 as libc::c_int,
    232 as libc::c_int,
    233 as libc::c_int,
    234 as libc::c_int,
    235 as libc::c_int,
    236 as libc::c_int,
    237 as libc::c_int,
    238 as libc::c_int,
    239 as libc::c_int,
    240 as libc::c_int,
    241 as libc::c_int,
    241 as libc::c_int,
    242 as libc::c_int,
    243 as libc::c_int,
    244 as libc::c_int,
    244 as libc::c_int,
    245 as libc::c_int,
    245 as libc::c_int,
    246 as libc::c_int,
    247 as libc::c_int,
    247 as libc::c_int,
    248 as libc::c_int,
    248 as libc::c_int,
    248 as libc::c_int,
    249 as libc::c_int,
    249 as libc::c_int,
    249 as libc::c_int,
    249 as libc::c_int,
    250 as libc::c_int,
    250 as libc::c_int,
    250 as libc::c_int,
    250 as libc::c_int,
    250 as libc::c_int,
    250 as libc::c_int,
    250 as libc::c_int,
    250 as libc::c_int,
    250 as libc::c_int,
    250 as libc::c_int,
    249 as libc::c_int,
    249 as libc::c_int,
    249 as libc::c_int,
    249 as libc::c_int,
    248 as libc::c_int,
    248 as libc::c_int,
    248 as libc::c_int,
    247 as libc::c_int,
    247 as libc::c_int,
    246 as libc::c_int,
    245 as libc::c_int,
    245 as libc::c_int,
    244 as libc::c_int,
    244 as libc::c_int,
    243 as libc::c_int,
    242 as libc::c_int,
    241 as libc::c_int,
    241 as libc::c_int,
    240 as libc::c_int,
    239 as libc::c_int,
    238 as libc::c_int,
    237 as libc::c_int,
    236 as libc::c_int,
    235 as libc::c_int,
    234 as libc::c_int,
    233 as libc::c_int,
    232 as libc::c_int,
    230 as libc::c_int,
    229 as libc::c_int,
    228 as libc::c_int,
    227 as libc::c_int,
    225 as libc::c_int,
    224 as libc::c_int,
    223 as libc::c_int,
    221 as libc::c_int,
    220 as libc::c_int,
    218 as libc::c_int,
    217 as libc::c_int,
    215 as libc::c_int,
    214 as libc::c_int,
    212 as libc::c_int,
    210 as libc::c_int,
    209 as libc::c_int,
    207 as libc::c_int,
    205 as libc::c_int,
    204 as libc::c_int,
    202 as libc::c_int,
    200 as libc::c_int,
    198 as libc::c_int,
    196 as libc::c_int,
    194 as libc::c_int,
    192 as libc::c_int,
    190 as libc::c_int,
    188 as libc::c_int,
    186 as libc::c_int,
    184 as libc::c_int,
    182 as libc::c_int,
    180 as libc::c_int,
    178 as libc::c_int,
    176 as libc::c_int,
    173 as libc::c_int,
    171 as libc::c_int,
    169 as libc::c_int,
    167 as libc::c_int,
    164 as libc::c_int,
    162 as libc::c_int,
    160 as libc::c_int,
    157 as libc::c_int,
    155 as libc::c_int,
    153 as libc::c_int,
    150 as libc::c_int,
    148 as libc::c_int,
    145 as libc::c_int,
    143 as libc::c_int,
    140 as libc::c_int,
    138 as libc::c_int,
    135 as libc::c_int,
    132 as libc::c_int,
    130 as libc::c_int,
    127 as libc::c_int,
    125 as libc::c_int,
    122 as libc::c_int,
    119 as libc::c_int,
    116 as libc::c_int,
    114 as libc::c_int,
    111 as libc::c_int,
    108 as libc::c_int,
    105 as libc::c_int,
    103 as libc::c_int,
    100 as libc::c_int,
    97 as libc::c_int,
    94 as libc::c_int,
    91 as libc::c_int,
    89 as libc::c_int,
    86 as libc::c_int,
    83 as libc::c_int,
    80 as libc::c_int,
    77 as libc::c_int,
    74 as libc::c_int,
    71 as libc::c_int,
    68 as libc::c_int,
    65 as libc::c_int,
    62 as libc::c_int,
    59 as libc::c_int,
    56 as libc::c_int,
    53 as libc::c_int,
    50 as libc::c_int,
    47 as libc::c_int,
    44 as libc::c_int,
    41 as libc::c_int,
    38 as libc::c_int,
    35 as libc::c_int,
    32 as libc::c_int,
    29 as libc::c_int,
    26 as libc::c_int,
    23 as libc::c_int,
    20 as libc::c_int,
    17 as libc::c_int,
    14 as libc::c_int,
    11 as libc::c_int,
    8 as libc::c_int,
    5 as libc::c_int,
    2 as libc::c_int,
    -(2 as libc::c_int),
    -(5 as libc::c_int),
    -(8 as libc::c_int),
    -(11 as libc::c_int),
    -(14 as libc::c_int),
    -(17 as libc::c_int),
    -(20 as libc::c_int),
    -(23 as libc::c_int),
    -(26 as libc::c_int),
    -(29 as libc::c_int),
    -(32 as libc::c_int),
    -(35 as libc::c_int),
    -(38 as libc::c_int),
    -(41 as libc::c_int),
    -(44 as libc::c_int),
    -(47 as libc::c_int),
    -(50 as libc::c_int),
    -(53 as libc::c_int),
    -(56 as libc::c_int),
    -(59 as libc::c_int),
    -(62 as libc::c_int),
    -(65 as libc::c_int),
    -(68 as libc::c_int),
    -(71 as libc::c_int),
    -(74 as libc::c_int),
    -(77 as libc::c_int),
    -(80 as libc::c_int),
    -(83 as libc::c_int),
    -(86 as libc::c_int),
    -(89 as libc::c_int),
    -(91 as libc::c_int),
    -(94 as libc::c_int),
    -(97 as libc::c_int),
    -(100 as libc::c_int),
    -(103 as libc::c_int),
    -(105 as libc::c_int),
    -(108 as libc::c_int),
    -(111 as libc::c_int),
    -(114 as libc::c_int),
    -(116 as libc::c_int),
    -(119 as libc::c_int),
    -(122 as libc::c_int),
    -(125 as libc::c_int),
    -(127 as libc::c_int),
    -(130 as libc::c_int),
    -(132 as libc::c_int),
    -(135 as libc::c_int),
    -(138 as libc::c_int),
    -(140 as libc::c_int),
    -(143 as libc::c_int),
    -(145 as libc::c_int),
    -(148 as libc::c_int),
    -(150 as libc::c_int),
    -(153 as libc::c_int),
    -(155 as libc::c_int),
    -(157 as libc::c_int),
    -(160 as libc::c_int),
    -(162 as libc::c_int),
    -(164 as libc::c_int),
    -(167 as libc::c_int),
    -(169 as libc::c_int),
    -(171 as libc::c_int),
    -(173 as libc::c_int),
    -(176 as libc::c_int),
    -(178 as libc::c_int),
    -(180 as libc::c_int),
    -(182 as libc::c_int),
    -(184 as libc::c_int),
    -(186 as libc::c_int),
    -(188 as libc::c_int),
    -(190 as libc::c_int),
    -(192 as libc::c_int),
    -(194 as libc::c_int),
    -(196 as libc::c_int),
    -(198 as libc::c_int),
    -(200 as libc::c_int),
    -(202 as libc::c_int),
    -(204 as libc::c_int),
    -(205 as libc::c_int),
    -(207 as libc::c_int),
    -(209 as libc::c_int),
    -(210 as libc::c_int),
    -(212 as libc::c_int),
    -(214 as libc::c_int),
    -(215 as libc::c_int),
    -(217 as libc::c_int),
    -(218 as libc::c_int),
    -(220 as libc::c_int),
    -(221 as libc::c_int),
    -(223 as libc::c_int),
    -(224 as libc::c_int),
    -(225 as libc::c_int),
    -(227 as libc::c_int),
    -(228 as libc::c_int),
    -(229 as libc::c_int),
    -(230 as libc::c_int),
    -(232 as libc::c_int),
    -(233 as libc::c_int),
    -(234 as libc::c_int),
    -(235 as libc::c_int),
    -(236 as libc::c_int),
    -(237 as libc::c_int),
    -(238 as libc::c_int),
    -(239 as libc::c_int),
    -(240 as libc::c_int),
    -(241 as libc::c_int),
    -(241 as libc::c_int),
    -(242 as libc::c_int),
    -(243 as libc::c_int),
    -(244 as libc::c_int),
    -(244 as libc::c_int),
    -(245 as libc::c_int),
    -(245 as libc::c_int),
    -(246 as libc::c_int),
    -(247 as libc::c_int),
    -(247 as libc::c_int),
    -(248 as libc::c_int),
    -(248 as libc::c_int),
    -(248 as libc::c_int),
    -(249 as libc::c_int),
    -(249 as libc::c_int),
    -(249 as libc::c_int),
    -(249 as libc::c_int),
    -(250 as libc::c_int),
    -(250 as libc::c_int),
    -(250 as libc::c_int),
    -(250 as libc::c_int),
    -(250 as libc::c_int),
    -(250 as libc::c_int),
    -(250 as libc::c_int),
    -(250 as libc::c_int),
    -(250 as libc::c_int),
    -(250 as libc::c_int),
    -(249 as libc::c_int),
    -(249 as libc::c_int),
    -(249 as libc::c_int),
    -(249 as libc::c_int),
    -(248 as libc::c_int),
    -(248 as libc::c_int),
    -(248 as libc::c_int),
    -(247 as libc::c_int),
    -(247 as libc::c_int),
    -(246 as libc::c_int),
    -(245 as libc::c_int),
    -(245 as libc::c_int),
    -(244 as libc::c_int),
    -(244 as libc::c_int),
    -(243 as libc::c_int),
    -(242 as libc::c_int),
    -(241 as libc::c_int),
    -(241 as libc::c_int),
    -(240 as libc::c_int),
    -(239 as libc::c_int),
    -(238 as libc::c_int),
    -(237 as libc::c_int),
    -(236 as libc::c_int),
    -(235 as libc::c_int),
    -(234 as libc::c_int),
    -(233 as libc::c_int),
    -(232 as libc::c_int),
    -(230 as libc::c_int),
    -(229 as libc::c_int),
    -(228 as libc::c_int),
    -(227 as libc::c_int),
    -(225 as libc::c_int),
    -(224 as libc::c_int),
    -(223 as libc::c_int),
    -(221 as libc::c_int),
    -(220 as libc::c_int),
    -(218 as libc::c_int),
    -(217 as libc::c_int),
    -(215 as libc::c_int),
    -(214 as libc::c_int),
    -(212 as libc::c_int),
    -(210 as libc::c_int),
    -(209 as libc::c_int),
    -(207 as libc::c_int),
    -(205 as libc::c_int),
    -(204 as libc::c_int),
    -(202 as libc::c_int),
    -(200 as libc::c_int),
    -(198 as libc::c_int),
    -(196 as libc::c_int),
    -(194 as libc::c_int),
    -(192 as libc::c_int),
    -(190 as libc::c_int),
    -(188 as libc::c_int),
    -(186 as libc::c_int),
    -(184 as libc::c_int),
    -(182 as libc::c_int),
    -(180 as libc::c_int),
    -(178 as libc::c_int),
    -(176 as libc::c_int),
    -(173 as libc::c_int),
    -(171 as libc::c_int),
    -(169 as libc::c_int),
    -(167 as libc::c_int),
    -(164 as libc::c_int),
    -(162 as libc::c_int),
    -(160 as libc::c_int),
    -(157 as libc::c_int),
    -(155 as libc::c_int),
    -(153 as libc::c_int),
    -(150 as libc::c_int),
    -(148 as libc::c_int),
    -(145 as libc::c_int),
    -(143 as libc::c_int),
    -(140 as libc::c_int),
    -(138 as libc::c_int),
    -(135 as libc::c_int),
    -(132 as libc::c_int),
    -(130 as libc::c_int),
    -(127 as libc::c_int),
    -(125 as libc::c_int),
    -(122 as libc::c_int),
    -(119 as libc::c_int),
    -(116 as libc::c_int),
    -(114 as libc::c_int),
    -(111 as libc::c_int),
    -(108 as libc::c_int),
    -(105 as libc::c_int),
    -(103 as libc::c_int),
    -(100 as libc::c_int),
    -(97 as libc::c_int),
    -(94 as libc::c_int),
    -(91 as libc::c_int),
    -(89 as libc::c_int),
    -(86 as libc::c_int),
    -(83 as libc::c_int),
    -(80 as libc::c_int),
    -(77 as libc::c_int),
    -(74 as libc::c_int),
    -(71 as libc::c_int),
    -(68 as libc::c_int),
    -(65 as libc::c_int),
    -(62 as libc::c_int),
    -(59 as libc::c_int),
    -(56 as libc::c_int),
    -(53 as libc::c_int),
    -(50 as libc::c_int),
    -(47 as libc::c_int),
    -(44 as libc::c_int),
    -(41 as libc::c_int),
    -(38 as libc::c_int),
    -(35 as libc::c_int),
    -(32 as libc::c_int),
    -(29 as libc::c_int),
    -(26 as libc::c_int),
    -(23 as libc::c_int),
    -(20 as libc::c_int),
    -(17 as libc::c_int),
    -(14 as libc::c_int),
    -(11 as libc::c_int),
    -(8 as libc::c_int),
    -(5 as libc::c_int),
    -(2 as libc::c_int),
];
#[unsafe(no_mangle)]
pub static mut cosTable512: [libc::c_int; 512] = [
    250 as libc::c_int,
    250 as libc::c_int,
    250 as libc::c_int,
    250 as libc::c_int,
    250 as libc::c_int,
    249 as libc::c_int,
    249 as libc::c_int,
    249 as libc::c_int,
    249 as libc::c_int,
    248 as libc::c_int,
    248 as libc::c_int,
    248 as libc::c_int,
    247 as libc::c_int,
    247 as libc::c_int,
    246 as libc::c_int,
    245 as libc::c_int,
    245 as libc::c_int,
    244 as libc::c_int,
    244 as libc::c_int,
    243 as libc::c_int,
    242 as libc::c_int,
    241 as libc::c_int,
    241 as libc::c_int,
    240 as libc::c_int,
    239 as libc::c_int,
    238 as libc::c_int,
    237 as libc::c_int,
    236 as libc::c_int,
    235 as libc::c_int,
    234 as libc::c_int,
    233 as libc::c_int,
    232 as libc::c_int,
    230 as libc::c_int,
    229 as libc::c_int,
    228 as libc::c_int,
    227 as libc::c_int,
    225 as libc::c_int,
    224 as libc::c_int,
    223 as libc::c_int,
    221 as libc::c_int,
    220 as libc::c_int,
    218 as libc::c_int,
    217 as libc::c_int,
    215 as libc::c_int,
    214 as libc::c_int,
    212 as libc::c_int,
    210 as libc::c_int,
    209 as libc::c_int,
    207 as libc::c_int,
    205 as libc::c_int,
    204 as libc::c_int,
    202 as libc::c_int,
    200 as libc::c_int,
    198 as libc::c_int,
    196 as libc::c_int,
    194 as libc::c_int,
    192 as libc::c_int,
    190 as libc::c_int,
    188 as libc::c_int,
    186 as libc::c_int,
    184 as libc::c_int,
    182 as libc::c_int,
    180 as libc::c_int,
    178 as libc::c_int,
    176 as libc::c_int,
    173 as libc::c_int,
    171 as libc::c_int,
    169 as libc::c_int,
    167 as libc::c_int,
    164 as libc::c_int,
    162 as libc::c_int,
    160 as libc::c_int,
    157 as libc::c_int,
    155 as libc::c_int,
    153 as libc::c_int,
    150 as libc::c_int,
    148 as libc::c_int,
    145 as libc::c_int,
    143 as libc::c_int,
    140 as libc::c_int,
    138 as libc::c_int,
    135 as libc::c_int,
    132 as libc::c_int,
    130 as libc::c_int,
    127 as libc::c_int,
    125 as libc::c_int,
    122 as libc::c_int,
    119 as libc::c_int,
    116 as libc::c_int,
    114 as libc::c_int,
    111 as libc::c_int,
    108 as libc::c_int,
    105 as libc::c_int,
    103 as libc::c_int,
    100 as libc::c_int,
    97 as libc::c_int,
    94 as libc::c_int,
    91 as libc::c_int,
    89 as libc::c_int,
    86 as libc::c_int,
    83 as libc::c_int,
    80 as libc::c_int,
    77 as libc::c_int,
    74 as libc::c_int,
    71 as libc::c_int,
    68 as libc::c_int,
    65 as libc::c_int,
    62 as libc::c_int,
    59 as libc::c_int,
    56 as libc::c_int,
    53 as libc::c_int,
    50 as libc::c_int,
    47 as libc::c_int,
    44 as libc::c_int,
    41 as libc::c_int,
    38 as libc::c_int,
    35 as libc::c_int,
    32 as libc::c_int,
    29 as libc::c_int,
    26 as libc::c_int,
    23 as libc::c_int,
    20 as libc::c_int,
    17 as libc::c_int,
    14 as libc::c_int,
    11 as libc::c_int,
    8 as libc::c_int,
    5 as libc::c_int,
    2 as libc::c_int,
    -(2 as libc::c_int),
    -(5 as libc::c_int),
    -(8 as libc::c_int),
    -(11 as libc::c_int),
    -(14 as libc::c_int),
    -(17 as libc::c_int),
    -(20 as libc::c_int),
    -(23 as libc::c_int),
    -(26 as libc::c_int),
    -(29 as libc::c_int),
    -(32 as libc::c_int),
    -(35 as libc::c_int),
    -(38 as libc::c_int),
    -(41 as libc::c_int),
    -(44 as libc::c_int),
    -(47 as libc::c_int),
    -(50 as libc::c_int),
    -(53 as libc::c_int),
    -(56 as libc::c_int),
    -(59 as libc::c_int),
    -(62 as libc::c_int),
    -(65 as libc::c_int),
    -(68 as libc::c_int),
    -(71 as libc::c_int),
    -(74 as libc::c_int),
    -(77 as libc::c_int),
    -(80 as libc::c_int),
    -(83 as libc::c_int),
    -(86 as libc::c_int),
    -(89 as libc::c_int),
    -(91 as libc::c_int),
    -(94 as libc::c_int),
    -(97 as libc::c_int),
    -(100 as libc::c_int),
    -(103 as libc::c_int),
    -(105 as libc::c_int),
    -(108 as libc::c_int),
    -(111 as libc::c_int),
    -(114 as libc::c_int),
    -(116 as libc::c_int),
    -(119 as libc::c_int),
    -(122 as libc::c_int),
    -(125 as libc::c_int),
    -(127 as libc::c_int),
    -(130 as libc::c_int),
    -(132 as libc::c_int),
    -(135 as libc::c_int),
    -(138 as libc::c_int),
    -(140 as libc::c_int),
    -(143 as libc::c_int),
    -(145 as libc::c_int),
    -(148 as libc::c_int),
    -(150 as libc::c_int),
    -(153 as libc::c_int),
    -(155 as libc::c_int),
    -(157 as libc::c_int),
    -(160 as libc::c_int),
    -(162 as libc::c_int),
    -(164 as libc::c_int),
    -(167 as libc::c_int),
    -(169 as libc::c_int),
    -(171 as libc::c_int),
    -(173 as libc::c_int),
    -(176 as libc::c_int),
    -(178 as libc::c_int),
    -(180 as libc::c_int),
    -(182 as libc::c_int),
    -(184 as libc::c_int),
    -(186 as libc::c_int),
    -(188 as libc::c_int),
    -(190 as libc::c_int),
    -(192 as libc::c_int),
    -(194 as libc::c_int),
    -(196 as libc::c_int),
    -(198 as libc::c_int),
    -(200 as libc::c_int),
    -(202 as libc::c_int),
    -(204 as libc::c_int),
    -(205 as libc::c_int),
    -(207 as libc::c_int),
    -(209 as libc::c_int),
    -(210 as libc::c_int),
    -(212 as libc::c_int),
    -(214 as libc::c_int),
    -(215 as libc::c_int),
    -(217 as libc::c_int),
    -(218 as libc::c_int),
    -(220 as libc::c_int),
    -(221 as libc::c_int),
    -(223 as libc::c_int),
    -(224 as libc::c_int),
    -(225 as libc::c_int),
    -(227 as libc::c_int),
    -(228 as libc::c_int),
    -(229 as libc::c_int),
    -(230 as libc::c_int),
    -(232 as libc::c_int),
    -(233 as libc::c_int),
    -(234 as libc::c_int),
    -(235 as libc::c_int),
    -(236 as libc::c_int),
    -(237 as libc::c_int),
    -(238 as libc::c_int),
    -(239 as libc::c_int),
    -(240 as libc::c_int),
    -(241 as libc::c_int),
    -(241 as libc::c_int),
    -(242 as libc::c_int),
    -(243 as libc::c_int),
    -(244 as libc::c_int),
    -(244 as libc::c_int),
    -(245 as libc::c_int),
    -(245 as libc::c_int),
    -(246 as libc::c_int),
    -(247 as libc::c_int),
    -(247 as libc::c_int),
    -(248 as libc::c_int),
    -(248 as libc::c_int),
    -(248 as libc::c_int),
    -(249 as libc::c_int),
    -(249 as libc::c_int),
    -(249 as libc::c_int),
    -(249 as libc::c_int),
    -(250 as libc::c_int),
    -(250 as libc::c_int),
    -(250 as libc::c_int),
    -(250 as libc::c_int),
    -(250 as libc::c_int),
    -(250 as libc::c_int),
    -(250 as libc::c_int),
    -(250 as libc::c_int),
    -(250 as libc::c_int),
    -(250 as libc::c_int),
    -(249 as libc::c_int),
    -(249 as libc::c_int),
    -(249 as libc::c_int),
    -(249 as libc::c_int),
    -(248 as libc::c_int),
    -(248 as libc::c_int),
    -(248 as libc::c_int),
    -(247 as libc::c_int),
    -(247 as libc::c_int),
    -(246 as libc::c_int),
    -(245 as libc::c_int),
    -(245 as libc::c_int),
    -(244 as libc::c_int),
    -(244 as libc::c_int),
    -(243 as libc::c_int),
    -(242 as libc::c_int),
    -(241 as libc::c_int),
    -(241 as libc::c_int),
    -(240 as libc::c_int),
    -(239 as libc::c_int),
    -(238 as libc::c_int),
    -(237 as libc::c_int),
    -(236 as libc::c_int),
    -(235 as libc::c_int),
    -(234 as libc::c_int),
    -(233 as libc::c_int),
    -(232 as libc::c_int),
    -(230 as libc::c_int),
    -(229 as libc::c_int),
    -(228 as libc::c_int),
    -(227 as libc::c_int),
    -(225 as libc::c_int),
    -(224 as libc::c_int),
    -(223 as libc::c_int),
    -(221 as libc::c_int),
    -(220 as libc::c_int),
    -(218 as libc::c_int),
    -(217 as libc::c_int),
    -(215 as libc::c_int),
    -(214 as libc::c_int),
    -(212 as libc::c_int),
    -(210 as libc::c_int),
    -(209 as libc::c_int),
    -(207 as libc::c_int),
    -(205 as libc::c_int),
    -(204 as libc::c_int),
    -(202 as libc::c_int),
    -(200 as libc::c_int),
    -(198 as libc::c_int),
    -(196 as libc::c_int),
    -(194 as libc::c_int),
    -(192 as libc::c_int),
    -(190 as libc::c_int),
    -(188 as libc::c_int),
    -(186 as libc::c_int),
    -(184 as libc::c_int),
    -(182 as libc::c_int),
    -(180 as libc::c_int),
    -(178 as libc::c_int),
    -(176 as libc::c_int),
    -(173 as libc::c_int),
    -(171 as libc::c_int),
    -(169 as libc::c_int),
    -(167 as libc::c_int),
    -(164 as libc::c_int),
    -(162 as libc::c_int),
    -(160 as libc::c_int),
    -(157 as libc::c_int),
    -(155 as libc::c_int),
    -(153 as libc::c_int),
    -(150 as libc::c_int),
    -(148 as libc::c_int),
    -(145 as libc::c_int),
    -(143 as libc::c_int),
    -(140 as libc::c_int),
    -(138 as libc::c_int),
    -(135 as libc::c_int),
    -(132 as libc::c_int),
    -(130 as libc::c_int),
    -(127 as libc::c_int),
    -(125 as libc::c_int),
    -(122 as libc::c_int),
    -(119 as libc::c_int),
    -(116 as libc::c_int),
    -(114 as libc::c_int),
    -(111 as libc::c_int),
    -(108 as libc::c_int),
    -(105 as libc::c_int),
    -(103 as libc::c_int),
    -(100 as libc::c_int),
    -(97 as libc::c_int),
    -(94 as libc::c_int),
    -(91 as libc::c_int),
    -(89 as libc::c_int),
    -(86 as libc::c_int),
    -(83 as libc::c_int),
    -(80 as libc::c_int),
    -(77 as libc::c_int),
    -(74 as libc::c_int),
    -(71 as libc::c_int),
    -(68 as libc::c_int),
    -(65 as libc::c_int),
    -(62 as libc::c_int),
    -(59 as libc::c_int),
    -(56 as libc::c_int),
    -(53 as libc::c_int),
    -(50 as libc::c_int),
    -(47 as libc::c_int),
    -(44 as libc::c_int),
    -(41 as libc::c_int),
    -(38 as libc::c_int),
    -(35 as libc::c_int),
    -(32 as libc::c_int),
    -(29 as libc::c_int),
    -(26 as libc::c_int),
    -(23 as libc::c_int),
    -(20 as libc::c_int),
    -(17 as libc::c_int),
    -(14 as libc::c_int),
    -(11 as libc::c_int),
    -(8 as libc::c_int),
    -(5 as libc::c_int),
    -(2 as libc::c_int),
    2 as libc::c_int,
    5 as libc::c_int,
    8 as libc::c_int,
    11 as libc::c_int,
    14 as libc::c_int,
    17 as libc::c_int,
    20 as libc::c_int,
    23 as libc::c_int,
    26 as libc::c_int,
    29 as libc::c_int,
    32 as libc::c_int,
    35 as libc::c_int,
    38 as libc::c_int,
    41 as libc::c_int,
    44 as libc::c_int,
    47 as libc::c_int,
    50 as libc::c_int,
    53 as libc::c_int,
    56 as libc::c_int,
    59 as libc::c_int,
    62 as libc::c_int,
    65 as libc::c_int,
    68 as libc::c_int,
    71 as libc::c_int,
    74 as libc::c_int,
    77 as libc::c_int,
    80 as libc::c_int,
    83 as libc::c_int,
    86 as libc::c_int,
    89 as libc::c_int,
    91 as libc::c_int,
    94 as libc::c_int,
    97 as libc::c_int,
    100 as libc::c_int,
    103 as libc::c_int,
    105 as libc::c_int,
    108 as libc::c_int,
    111 as libc::c_int,
    114 as libc::c_int,
    116 as libc::c_int,
    119 as libc::c_int,
    122 as libc::c_int,
    125 as libc::c_int,
    127 as libc::c_int,
    130 as libc::c_int,
    132 as libc::c_int,
    135 as libc::c_int,
    138 as libc::c_int,
    140 as libc::c_int,
    143 as libc::c_int,
    145 as libc::c_int,
    148 as libc::c_int,
    150 as libc::c_int,
    153 as libc::c_int,
    155 as libc::c_int,
    157 as libc::c_int,
    160 as libc::c_int,
    162 as libc::c_int,
    164 as libc::c_int,
    167 as libc::c_int,
    169 as libc::c_int,
    171 as libc::c_int,
    173 as libc::c_int,
    176 as libc::c_int,
    178 as libc::c_int,
    180 as libc::c_int,
    182 as libc::c_int,
    184 as libc::c_int,
    186 as libc::c_int,
    188 as libc::c_int,
    190 as libc::c_int,
    192 as libc::c_int,
    194 as libc::c_int,
    196 as libc::c_int,
    198 as libc::c_int,
    200 as libc::c_int,
    202 as libc::c_int,
    204 as libc::c_int,
    205 as libc::c_int,
    207 as libc::c_int,
    209 as libc::c_int,
    210 as libc::c_int,
    212 as libc::c_int,
    214 as libc::c_int,
    215 as libc::c_int,
    217 as libc::c_int,
    218 as libc::c_int,
    220 as libc::c_int,
    221 as libc::c_int,
    223 as libc::c_int,
    224 as libc::c_int,
    225 as libc::c_int,
    227 as libc::c_int,
    228 as libc::c_int,
    229 as libc::c_int,
    230 as libc::c_int,
    232 as libc::c_int,
    233 as libc::c_int,
    234 as libc::c_int,
    235 as libc::c_int,
    236 as libc::c_int,
    237 as libc::c_int,
    238 as libc::c_int,
    239 as libc::c_int,
    240 as libc::c_int,
    241 as libc::c_int,
    241 as libc::c_int,
    242 as libc::c_int,
    243 as libc::c_int,
    244 as libc::c_int,
    244 as libc::c_int,
    245 as libc::c_int,
    245 as libc::c_int,
    246 as libc::c_int,
    247 as libc::c_int,
    247 as libc::c_int,
    248 as libc::c_int,
    248 as libc::c_int,
    248 as libc::c_int,
    249 as libc::c_int,
    249 as libc::c_int,
    249 as libc::c_int,
    249 as libc::c_int,
    250 as libc::c_int,
    250 as libc::c_int,
    250 as libc::c_int,
    250 as libc::c_int,
    250 as libc::c_int,
];
#[unsafe(no_mangle)]
pub static mut ant_pat_db: [libc::c_double; 37] = [
    0.00f64, 0.00f64, 0.22f64, 0.44f64, 0.67f64, 1.11f64, 1.56f64, 2.00f64, 2.44f64, 2.89f64,
    3.56f64, 4.22f64, 4.89f64, 5.56f64, 6.22f64, 6.89f64, 7.56f64, 8.22f64, 8.89f64, 9.78f64,
    10.67f64, 11.56f64, 12.44f64, 13.33f64, 14.44f64, 15.56f64, 16.67f64, 17.78f64, 18.89f64,
    20.00f64, 21.33f64, 22.67f64, 24.00f64, 25.56f64, 27.33f64, 29.33f64, 31.56f64,
];
#[unsafe(no_mangle)]
pub static mut allocatedSat: [libc::c_int; 32] = [0; 32];
#[unsafe(no_mangle)]
pub static mut xyz: [[libc::c_double; 3]; 3000] = [[0.; 3]; 3000];
#[unsafe(no_mangle)]
pub unsafe extern "C" fn subVect(
    mut y: *mut libc::c_double,
    mut x1: *const libc::c_double,
    mut x2: *const libc::c_double,
) {
    unsafe {
        *y.offset(0 as libc::c_int as isize) =
            *x1.offset(0 as libc::c_int as isize) - *x2.offset(0 as libc::c_int as isize);
        *y.offset(1 as libc::c_int as isize) =
            *x1.offset(1 as libc::c_int as isize) - *x2.offset(1 as libc::c_int as isize);
        *y.offset(2 as libc::c_int as isize) =
            *x1.offset(2 as libc::c_int as isize) - *x2.offset(2 as libc::c_int as isize);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn normVect(mut x: *const libc::c_double) -> libc::c_double {
    unsafe {
        sqrt(
            *x.offset(0 as libc::c_int as isize) * *x.offset(0 as libc::c_int as isize)
                + *x.offset(1 as libc::c_int as isize) * *x.offset(1 as libc::c_int as isize)
                + *x.offset(2 as libc::c_int as isize) * *x.offset(2 as libc::c_int as isize),
        )
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dotProd(
    mut x1: *const libc::c_double,
    mut x2: *const libc::c_double,
) -> libc::c_double {
    unsafe {
        *x1.offset(0 as libc::c_int as isize) * *x2.offset(0 as libc::c_int as isize)
            + *x1.offset(1 as libc::c_int as isize) * *x2.offset(1 as libc::c_int as isize)
            + *x1.offset(2 as libc::c_int as isize) * *x2.offset(2 as libc::c_int as isize)
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn codegen(mut ca: *mut libc::c_int, mut prn: libc::c_int) {
    unsafe {
        let mut delay: [libc::c_int; 32] = [
            5 as libc::c_int,
            6 as libc::c_int,
            7 as libc::c_int,
            8 as libc::c_int,
            17 as libc::c_int,
            18 as libc::c_int,
            139 as libc::c_int,
            140 as libc::c_int,
            141 as libc::c_int,
            251 as libc::c_int,
            252 as libc::c_int,
            254 as libc::c_int,
            255 as libc::c_int,
            256 as libc::c_int,
            257 as libc::c_int,
            258 as libc::c_int,
            469 as libc::c_int,
            470 as libc::c_int,
            471 as libc::c_int,
            472 as libc::c_int,
            473 as libc::c_int,
            474 as libc::c_int,
            509 as libc::c_int,
            512 as libc::c_int,
            513 as libc::c_int,
            514 as libc::c_int,
            515 as libc::c_int,
            516 as libc::c_int,
            859 as libc::c_int,
            860 as libc::c_int,
            861 as libc::c_int,
            862 as libc::c_int,
        ];
        let mut g1: [libc::c_int; 1023] = [0; 1023];
        let mut g2: [libc::c_int; 1023] = [0; 1023];
        let mut r1: [libc::c_int; 10] = [0; 10];
        let mut r2: [libc::c_int; 10] = [0; 10];
        let mut c1: libc::c_int = 0;
        let mut c2: libc::c_int = 0;
        let mut i: libc::c_int = 0;
        let mut j: libc::c_int = 0;
        if prn < 1 as libc::c_int || prn > 32 as libc::c_int {
            return;
        }
        i = 0 as libc::c_int;
        while i < 10 as libc::c_int {
            r2[i as usize] = -(1 as libc::c_int);
            r1[i as usize] = r2[i as usize];
            i += 1;
        }
        i = 0 as libc::c_int;
        while i < 1023 as libc::c_int {
            g1[i as usize] = r1[9 as libc::c_int as usize];
            g2[i as usize] = r2[9 as libc::c_int as usize];
            c1 = r1[2 as libc::c_int as usize] * r1[9 as libc::c_int as usize];
            c2 = r2[1 as libc::c_int as usize]
                * r2[2 as libc::c_int as usize]
                * r2[5 as libc::c_int as usize]
                * r2[7 as libc::c_int as usize]
                * r2[8 as libc::c_int as usize]
                * r2[9 as libc::c_int as usize];
            j = 9 as libc::c_int;
            while j > 0 as libc::c_int {
                r1[j as usize] = r1[(j - 1 as libc::c_int) as usize];
                r2[j as usize] = r2[(j - 1 as libc::c_int) as usize];
                j -= 1;
            }
            r1[0 as libc::c_int as usize] = c1;
            r2[0 as libc::c_int as usize] = c2;
            i += 1;
        }
        i = 0 as libc::c_int;
        j = 1023 as libc::c_int - delay[(prn - 1 as libc::c_int) as usize];
        while i < 1023 as libc::c_int {
            *ca.offset(i as isize) = (1 as libc::c_int
                - g1[i as usize] * g2[(j % 1023 as libc::c_int) as usize])
                / 2 as libc::c_int;
            i += 1;
            j += 1;
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn date2gps(mut t: *const datetime_t, mut g: *mut gpstime_t) {
    unsafe {
        let mut doy: [libc::c_int; 12] = [
            0 as libc::c_int,
            31 as libc::c_int,
            59 as libc::c_int,
            90 as libc::c_int,
            120 as libc::c_int,
            151 as libc::c_int,
            181 as libc::c_int,
            212 as libc::c_int,
            243 as libc::c_int,
            273 as libc::c_int,
            304 as libc::c_int,
            334 as libc::c_int,
        ];
        let mut ye: libc::c_int = 0;
        let mut de: libc::c_int = 0;
        let mut lpdays: libc::c_int = 0;
        ye = (*t).y - 1980 as libc::c_int;
        lpdays = ye / 4 as libc::c_int + 1 as libc::c_int;
        if ye % 4 as libc::c_int == 0 as libc::c_int && (*t).m <= 2 as libc::c_int {
            lpdays -= 1;
        }
        de = ye * 365 as libc::c_int + doy[((*t).m - 1 as libc::c_int) as usize] + (*t).d + lpdays
            - 6 as libc::c_int;
        (*g).week = de / 7 as libc::c_int;
        (*g).sec = (de % 7 as libc::c_int) as libc::c_double * 86400.0f64
            + (*t).hh as libc::c_double * 3600.0f64
            + (*t).mm as libc::c_double * 60.0f64
            + (*t).sec;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gps2date(mut g: *const gpstime_t, mut t: *mut datetime_t) {
    unsafe {
        let mut c: libc::c_int = ((7 as libc::c_int * (*g).week) as libc::c_double
            + floor((*g).sec / 86400.0f64)
            + 2444245.0f64) as libc::c_int
            + 1537 as libc::c_int;
        let mut d: libc::c_int = ((c as libc::c_double - 122.1f64) / 365.25f64) as libc::c_int;
        let mut e: libc::c_int = 365 as libc::c_int * d + d / 4 as libc::c_int;
        let mut f: libc::c_int = ((c - e) as libc::c_double / 30.6001f64) as libc::c_int;
        (*t).d = c - e - (30.6001f64 * f as libc::c_double) as libc::c_int;
        (*t).m = f - 1 as libc::c_int - 12 as libc::c_int * (f / 14 as libc::c_int);
        (*t).y = d - 4715 as libc::c_int - (7 as libc::c_int + (*t).m) / 10 as libc::c_int;
        (*t).hh = ((*g).sec / 3600.0f64) as libc::c_int % 24 as libc::c_int;
        (*t).mm = ((*g).sec / 60.0f64) as libc::c_int % 60 as libc::c_int;
        (*t).sec = (*g).sec - 60.0f64 * floor((*g).sec / 60.0f64);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xyz2llh(mut xyz_0: *const libc::c_double, mut llh: *mut libc::c_double) {
    unsafe {
        let mut a: libc::c_double = 0.;
        let mut eps: libc::c_double = 0.;
        let mut e: libc::c_double = 0.;
        let mut e2: libc::c_double = 0.;
        let mut x: libc::c_double = 0.;
        let mut y: libc::c_double = 0.;
        let mut z: libc::c_double = 0.;
        let mut rho2: libc::c_double = 0.;
        let mut dz: libc::c_double = 0.;
        let mut zdz: libc::c_double = 0.;
        let mut nh: libc::c_double = 0.;
        let mut slat: libc::c_double = 0.;
        let mut n: libc::c_double = 0.;
        let mut dz_new: libc::c_double = 0.;
        a = 6378137.0f64;
        e = 0.0818191908426f64;
        eps = 1.0e-3f64;
        e2 = e * e;
        if normVect(xyz_0) < eps {
            *llh.offset(0 as libc::c_int as isize) = 0.0f64;
            *llh.offset(1 as libc::c_int as isize) = 0.0f64;
            *llh.offset(2 as libc::c_int as isize) = -a;
            return;
        }
        x = *xyz_0.offset(0 as libc::c_int as isize);
        y = *xyz_0.offset(1 as libc::c_int as isize);
        z = *xyz_0.offset(2 as libc::c_int as isize);
        rho2 = x * x + y * y;
        dz = e2 * z;
        loop {
            zdz = z + dz;
            nh = sqrt(rho2 + zdz * zdz);
            slat = zdz / nh;
            n = a / sqrt(1.0f64 - e2 * slat * slat);
            dz_new = n * e2 * slat;
            if fabs(dz - dz_new) < eps {
                break;
            }
            dz = dz_new;
        }
        *llh.offset(0 as libc::c_int as isize) = atan2(zdz, sqrt(rho2));
        *llh.offset(1 as libc::c_int as isize) = atan2(y, x);
        *llh.offset(2 as libc::c_int as isize) = nh - n;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn llh2xyz(mut llh: *const libc::c_double, mut xyz_0: *mut libc::c_double) {
    unsafe {
        let mut n: libc::c_double = 0.;
        let mut a: libc::c_double = 0.;
        let mut e: libc::c_double = 0.;
        let mut e2: libc::c_double = 0.;
        let mut clat: libc::c_double = 0.;
        let mut slat: libc::c_double = 0.;
        let mut clon: libc::c_double = 0.;
        let mut slon: libc::c_double = 0.;
        let mut d: libc::c_double = 0.;
        let mut nph: libc::c_double = 0.;
        let mut tmp: libc::c_double = 0.;
        a = 6378137.0f64;
        e = 0.0818191908426f64;
        e2 = e * e;
        clat = cos(*llh.offset(0 as libc::c_int as isize));
        slat = sin(*llh.offset(0 as libc::c_int as isize));
        clon = cos(*llh.offset(1 as libc::c_int as isize));
        slon = sin(*llh.offset(1 as libc::c_int as isize));
        d = e * slat;
        n = a / sqrt(1.0f64 - d * d);
        nph = n + *llh.offset(2 as libc::c_int as isize);
        tmp = nph * clat;
        *xyz_0.offset(0 as libc::c_int as isize) = tmp * clon;
        *xyz_0.offset(1 as libc::c_int as isize) = tmp * slon;
        *xyz_0.offset(2 as libc::c_int as isize) =
            ((1.0f64 - e2) * n + *llh.offset(2 as libc::c_int as isize)) * slat;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ltcmat(mut llh: *const libc::c_double, mut t: *mut [libc::c_double; 3]) {
    unsafe {
        let mut slat: libc::c_double = 0.;
        let mut clat: libc::c_double = 0.;
        let mut slon: libc::c_double = 0.;
        let mut clon: libc::c_double = 0.;
        slat = sin(*llh.offset(0 as libc::c_int as isize));
        clat = cos(*llh.offset(0 as libc::c_int as isize));
        slon = sin(*llh.offset(1 as libc::c_int as isize));
        clon = cos(*llh.offset(1 as libc::c_int as isize));
        (*t.offset(0 as libc::c_int as isize))[0 as libc::c_int as usize] = -slat * clon;
        (*t.offset(0 as libc::c_int as isize))[1 as libc::c_int as usize] = -slat * slon;
        (*t.offset(0 as libc::c_int as isize))[2 as libc::c_int as usize] = clat;
        (*t.offset(1 as libc::c_int as isize))[0 as libc::c_int as usize] = -slon;
        (*t.offset(1 as libc::c_int as isize))[1 as libc::c_int as usize] = clon;
        (*t.offset(1 as libc::c_int as isize))[2 as libc::c_int as usize] = 0.0f64;
        (*t.offset(2 as libc::c_int as isize))[0 as libc::c_int as usize] = clat * clon;
        (*t.offset(2 as libc::c_int as isize))[1 as libc::c_int as usize] = clat * slon;
        (*t.offset(2 as libc::c_int as isize))[2 as libc::c_int as usize] = slat;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ecef2neu(
    mut xyz_0: *const libc::c_double,
    mut t: *mut [libc::c_double; 3],
    mut neu: *mut libc::c_double,
) {
    unsafe {
        *neu.offset(0 as libc::c_int as isize) = (*t.offset(0 as libc::c_int as isize))
            [0 as libc::c_int as usize]
            * *xyz_0.offset(0 as libc::c_int as isize)
            + (*t.offset(0 as libc::c_int as isize))[1 as libc::c_int as usize]
                * *xyz_0.offset(1 as libc::c_int as isize)
            + (*t.offset(0 as libc::c_int as isize))[2 as libc::c_int as usize]
                * *xyz_0.offset(2 as libc::c_int as isize);
        *neu.offset(1 as libc::c_int as isize) = (*t.offset(1 as libc::c_int as isize))
            [0 as libc::c_int as usize]
            * *xyz_0.offset(0 as libc::c_int as isize)
            + (*t.offset(1 as libc::c_int as isize))[1 as libc::c_int as usize]
                * *xyz_0.offset(1 as libc::c_int as isize)
            + (*t.offset(1 as libc::c_int as isize))[2 as libc::c_int as usize]
                * *xyz_0.offset(2 as libc::c_int as isize);
        *neu.offset(2 as libc::c_int as isize) = (*t.offset(2 as libc::c_int as isize))
            [0 as libc::c_int as usize]
            * *xyz_0.offset(0 as libc::c_int as isize)
            + (*t.offset(2 as libc::c_int as isize))[1 as libc::c_int as usize]
                * *xyz_0.offset(1 as libc::c_int as isize)
            + (*t.offset(2 as libc::c_int as isize))[2 as libc::c_int as usize]
                * *xyz_0.offset(2 as libc::c_int as isize);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn neu2azel(mut azel: *mut libc::c_double, mut neu: *const libc::c_double) {
    unsafe {
        let mut ne: libc::c_double = 0.;
        *azel.offset(0 as libc::c_int as isize) = atan2(
            *neu.offset(1 as libc::c_int as isize),
            *neu.offset(0 as libc::c_int as isize),
        );
        if *azel.offset(0 as libc::c_int as isize) < 0.0f64 {
            *azel.offset(0 as libc::c_int as isize) += 2.0f64 * 3.1415926535898f64;
        }
        ne = sqrt(
            *neu.offset(0 as libc::c_int as isize) * *neu.offset(0 as libc::c_int as isize)
                + *neu.offset(1 as libc::c_int as isize) * *neu.offset(1 as libc::c_int as isize),
        );
        *azel.offset(1 as libc::c_int as isize) = atan2(*neu.offset(2 as libc::c_int as isize), ne);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn satpos(
    mut eph: ephem_t,
    mut g: gpstime_t,
    mut pos: *mut libc::c_double,
    mut vel: *mut libc::c_double,
    mut clk: *mut libc::c_double,
) {
    unsafe {
        let mut tk: libc::c_double = 0.;
        let mut mk: libc::c_double = 0.;
        let mut ek: libc::c_double = 0.;
        let mut ekold: libc::c_double = 0.;
        let mut ekdot: libc::c_double = 0.;
        let mut cek: libc::c_double = 0.;
        let mut sek: libc::c_double = 0.;
        let mut pk: libc::c_double = 0.;
        let mut pkdot: libc::c_double = 0.;
        let mut c2pk: libc::c_double = 0.;
        let mut s2pk: libc::c_double = 0.;
        let mut uk: libc::c_double = 0.;
        let mut ukdot: libc::c_double = 0.;
        let mut cuk: libc::c_double = 0.;
        let mut suk: libc::c_double = 0.;
        let mut ok: libc::c_double = 0.;
        let mut sok: libc::c_double = 0.;
        let mut cok: libc::c_double = 0.;
        let mut ik: libc::c_double = 0.;
        let mut ikdot: libc::c_double = 0.;
        let mut sik: libc::c_double = 0.;
        let mut cik: libc::c_double = 0.;
        let mut rk: libc::c_double = 0.;
        let mut rkdot: libc::c_double = 0.;
        let mut xpk: libc::c_double = 0.;
        let mut ypk: libc::c_double = 0.;
        let mut xpkdot: libc::c_double = 0.;
        let mut ypkdot: libc::c_double = 0.;
        let mut relativistic: libc::c_double = 0.;
        let mut OneMinusecosE: libc::c_double = 0.;
        let mut tmp: libc::c_double = 0.;
        tk = g.sec - eph.toe.sec;
        if tk > 302400.0f64 {
            tk -= 604800.0f64;
        } else if tk < -302400.0f64 {
            tk += 604800.0f64;
        }
        mk = eph.m0 + eph.n * tk;
        ek = mk;
        ekold = ek + 1.0f64;
        OneMinusecosE = 0 as libc::c_int as libc::c_double;
        while fabs(ek - ekold) > 1.0E-14f64 {
            ekold = ek;
            OneMinusecosE = 1.0f64 - eph.ecc * cos(ekold);
            ek += (mk - ekold + eph.ecc * sin(ekold)) / OneMinusecosE;
        }
        sek = sin(ek);
        cek = cos(ek);
        ekdot = eph.n / OneMinusecosE;
        relativistic = -4.442807633E-10f64 * eph.ecc * eph.sqrta * sek;
        pk = atan2(eph.sq1e2 * sek, cek - eph.ecc) + eph.aop;
        pkdot = eph.sq1e2 * ekdot / OneMinusecosE;
        s2pk = sin(2.0f64 * pk);
        c2pk = cos(2.0f64 * pk);
        uk = pk + eph.cus * s2pk + eph.cuc * c2pk;
        suk = sin(uk);
        cuk = cos(uk);
        ukdot = pkdot * (1.0f64 + 2.0f64 * (eph.cus * c2pk - eph.cuc * s2pk));
        rk = eph.A * OneMinusecosE + eph.crc * c2pk + eph.crs * s2pk;
        rkdot = eph.A * eph.ecc * sek * ekdot + 2.0f64 * pkdot * (eph.crs * c2pk - eph.crc * s2pk);
        ik = eph.inc0 + eph.idot * tk + eph.cic * c2pk + eph.cis * s2pk;
        sik = sin(ik);
        cik = cos(ik);
        ikdot = eph.idot + 2.0f64 * pkdot * (eph.cis * c2pk - eph.cic * s2pk);
        xpk = rk * cuk;
        ypk = rk * suk;
        xpkdot = rkdot * cuk - ypk * ukdot;
        ypkdot = rkdot * suk + xpk * ukdot;
        ok = eph.omg0 + tk * eph.omgkdot - 7.2921151467e-5f64 * eph.toe.sec;
        sok = sin(ok);
        cok = cos(ok);
        *pos.offset(0 as libc::c_int as isize) = xpk * cok - ypk * cik * sok;
        *pos.offset(1 as libc::c_int as isize) = xpk * sok + ypk * cik * cok;
        *pos.offset(2 as libc::c_int as isize) = ypk * sik;
        tmp = ypkdot * cik - ypk * sik * ikdot;
        *vel.offset(0 as libc::c_int as isize) =
            -eph.omgkdot * *pos.offset(1 as libc::c_int as isize) + xpkdot * cok - tmp * sok;
        *vel.offset(1 as libc::c_int as isize) =
            eph.omgkdot * *pos.offset(0 as libc::c_int as isize) + xpkdot * sok + tmp * cok;
        *vel.offset(2 as libc::c_int as isize) = ypk * cik * ikdot + ypkdot * sik;
        tk = g.sec - eph.toc.sec;
        if tk > 302400.0f64 {
            tk -= 604800.0f64;
        } else if tk < -302400.0f64 {
            tk += 604800.0f64;
        }
        *clk.offset(0 as libc::c_int as isize) =
            eph.af0 + tk * (eph.af1 + tk * eph.af2) + relativistic - eph.tgd;
        *clk.offset(1 as libc::c_int as isize) = eph.af1 + 2.0f64 * tk * eph.af2;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eph2sbf(
    eph: ephem_t,
    ionoutc: ionoutc_t,
    mut sbf: *mut [libc::c_ulong; 10],
) {
    unsafe {
        let mut wn: libc::c_ulong = 0;
        let mut toe: libc::c_ulong = 0;
        let mut toc: libc::c_ulong = 0;
        let mut iode: libc::c_ulong = 0;
        let mut iodc: libc::c_ulong = 0;
        let mut deltan: libc::c_long = 0;
        let mut cuc: libc::c_long = 0;
        let mut cus: libc::c_long = 0;
        let mut cic: libc::c_long = 0;
        let mut cis: libc::c_long = 0;
        let mut crc: libc::c_long = 0;
        let mut crs: libc::c_long = 0;
        let mut ecc: libc::c_ulong = 0;
        let mut sqrta: libc::c_ulong = 0;
        let mut m0: libc::c_long = 0;
        let mut omg0: libc::c_long = 0;
        let mut inc0: libc::c_long = 0;
        let mut aop: libc::c_long = 0;
        let mut omgdot: libc::c_long = 0;
        let mut idot: libc::c_long = 0;
        let mut af0: libc::c_long = 0;
        let mut af1: libc::c_long = 0;
        let mut af2: libc::c_long = 0;
        let mut tgd: libc::c_long = 0;
        let mut svhlth: libc::c_int = 0;
        let mut codeL2: libc::c_int = 0;
        let mut ura: libc::c_ulong = 0 as libc::c_ulong;
        let mut dataId: libc::c_ulong = 1 as libc::c_ulong;
        let mut sbf4_page25_svId: libc::c_ulong = 63 as libc::c_ulong;
        let mut sbf5_page25_svId: libc::c_ulong = 51 as libc::c_ulong;
        let mut wna: libc::c_ulong = 0;
        let mut toa: libc::c_ulong = 0;
        let mut alpha0: libc::c_long = 0;
        let mut alpha1: libc::c_long = 0;
        let mut alpha2: libc::c_long = 0;
        let mut alpha3: libc::c_long = 0;
        let mut beta0: libc::c_long = 0;
        let mut beta1: libc::c_long = 0;
        let mut beta2: libc::c_long = 0;
        let mut beta3: libc::c_long = 0;
        let mut A0: libc::c_long = 0;
        let mut A1: libc::c_long = 0;
        let mut dtls: libc::c_long = 0;
        let mut tot: libc::c_ulong = 0;
        let mut wnt: libc::c_ulong = 0;
        let mut wnlsf: libc::c_ulong = 0;
        let mut dtlsf: libc::c_ulong = 0;
        let mut dn: libc::c_ulong = 0;
        let mut sbf4_page18_svId: libc::c_ulong = 56 as libc::c_ulong;
        wn = 0 as libc::c_ulong;
        toe = (eph.toe.sec / 16.0f64) as libc::c_ulong;
        toc = (eph.toc.sec / 16.0f64) as libc::c_ulong;
        iode = eph.iode as libc::c_ulong;
        iodc = eph.iodc as libc::c_ulong;
        deltan = (eph.deltan / 1.136_868_377_216_16e-13_f64 / 3.1415926535898f64) as libc::c_long;
        cuc = (eph.cuc / 1.862645149230957e-9f64) as libc::c_long;
        cus = (eph.cus / 1.862645149230957e-9f64) as libc::c_long;
        cic = (eph.cic / 1.862645149230957e-9f64) as libc::c_long;
        cis = (eph.cis / 1.862645149230957e-9f64) as libc::c_long;
        crc = (eph.crc / 0.03125f64) as libc::c_long;
        crs = (eph.crs / 0.03125f64) as libc::c_long;
        ecc = (eph.ecc / 1.164153218269348e-10f64) as libc::c_ulong;
        sqrta = (eph.sqrta / 1.907_348_632_812_5e-6_f64) as libc::c_ulong;
        m0 = (eph.m0 / 4.656612873077393e-10f64 / 3.1415926535898f64) as libc::c_long;
        omg0 = (eph.omg0 / 4.656612873077393e-10f64 / 3.1415926535898f64) as libc::c_long;
        inc0 = (eph.inc0 / 4.656612873077393e-10f64 / 3.1415926535898f64) as libc::c_long;
        aop = (eph.aop / 4.656612873077393e-10f64 / 3.1415926535898f64) as libc::c_long;
        omgdot = (eph.omgdot / 1.136_868_377_216_16e-13_f64 / 3.1415926535898f64) as libc::c_long;
        idot = (eph.idot / 1.136_868_377_216_16e-13_f64 / 3.1415926535898f64) as libc::c_long;
        af0 = (eph.af0 / 4.656612873077393e-10f64) as libc::c_long;
        af1 = (eph.af1 / 1.136_868_377_216_16e-13_f64) as libc::c_long;
        af2 = (eph.af2 / 2.775557561562891e-17f64) as libc::c_long;
        tgd = (eph.tgd / 4.656612873077393e-10f64) as libc::c_long;
        svhlth = eph.svhlth as libc::c_ulong as libc::c_int;
        codeL2 = eph.codeL2 as libc::c_ulong as libc::c_int;
        wna = (eph.toe.week % 256 as libc::c_int) as libc::c_ulong;
        toa = (eph.toe.sec / 4096.0f64) as libc::c_ulong;
        alpha0 = round(ionoutc.alpha0 / 9.313_225_746_154_785e-10_f64) as libc::c_long;
        alpha1 = round(ionoutc.alpha1 / 7.450_580_596_923_828e-9_f64) as libc::c_long;
        alpha2 = round(ionoutc.alpha2 / 5.960_464_477_539_063e-8_f64) as libc::c_long;
        alpha3 = round(ionoutc.alpha3 / 5.960_464_477_539_063e-8_f64) as libc::c_long;
        beta0 = round(ionoutc.beta0 / 2048.0f64) as libc::c_long;
        beta1 = round(ionoutc.beta1 / 16384.0f64) as libc::c_long;
        beta2 = round(ionoutc.beta2 / 65536.0f64) as libc::c_long;
        beta3 = round(ionoutc.beta3 / 65536.0f64) as libc::c_long;
        A0 = round(ionoutc.A0 / 9.313_225_746_154_785e-10_f64) as libc::c_long;
        A1 = round(ionoutc.A1 / 8.881_784_197_001_252e-16_f64) as libc::c_long;
        dtls = ionoutc.dtls as libc::c_long;
        tot = (ionoutc.tot / 4096 as libc::c_int) as libc::c_ulong;
        wnt = (ionoutc.wnt % 256 as libc::c_int) as libc::c_ulong;
        if ionoutc.leapen == 1 as libc::c_int {
            wnlsf = (ionoutc.wnlsf % 256 as libc::c_int) as libc::c_ulong;
            dn = ionoutc.dn as libc::c_ulong;
            dtlsf = ionoutc.dtlsf as libc::c_ulong;
        } else {
            wnlsf = (1929 as libc::c_int % 256 as libc::c_int) as libc::c_ulong;
            dn = 7 as libc::c_int as libc::c_ulong;
            dtlsf = 18 as libc::c_int as libc::c_ulong;
        }
        (*sbf.offset(0 as libc::c_int as isize))[0 as libc::c_int as usize] =
            (0x8b0000 as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(0 as libc::c_int as isize))[1 as libc::c_int as usize] =
            (0x1 as libc::c_ulong) << 8 as libc::c_int;
        (*sbf.offset(0 as libc::c_int as isize))[2 as libc::c_int as usize] =
            (wn & 0x3ff as libc::c_ulong) << 20 as libc::c_int
                | (codeL2 as libc::c_ulong & 0x3 as libc::c_ulong) << 18 as libc::c_int
                | (ura & 0xf as libc::c_ulong) << 14 as libc::c_int
                | (svhlth as libc::c_ulong & 0x3f as libc::c_ulong) << 8 as libc::c_int
                | (iodc >> 8 as libc::c_int & 0x3 as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(0 as libc::c_int as isize))[3 as libc::c_int as usize] = 0 as libc::c_ulong;
        (*sbf.offset(0 as libc::c_int as isize))[4 as libc::c_int as usize] = 0 as libc::c_ulong;
        (*sbf.offset(0 as libc::c_int as isize))[5 as libc::c_int as usize] = 0 as libc::c_ulong;
        (*sbf.offset(0 as libc::c_int as isize))[6 as libc::c_int as usize] =
            (tgd as libc::c_ulong & 0xff as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(0 as libc::c_int as isize))[7 as libc::c_int as usize] =
            (iodc & 0xff as libc::c_ulong) << 22 as libc::c_int
                | (toc & 0xffff as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(0 as libc::c_int as isize))[8 as libc::c_int as usize] =
            (af2 as libc::c_ulong & 0xff as libc::c_ulong) << 22 as libc::c_int
                | (af1 as libc::c_ulong & 0xffff as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(0 as libc::c_int as isize))[9 as libc::c_int as usize] =
            (af0 as libc::c_ulong & 0x3fffff as libc::c_ulong) << 8 as libc::c_int;
        (*sbf.offset(1 as libc::c_int as isize))[0 as libc::c_int as usize] =
            (0x8b0000 as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(1 as libc::c_int as isize))[1 as libc::c_int as usize] =
            (0x2 as libc::c_ulong) << 8 as libc::c_int;
        (*sbf.offset(1 as libc::c_int as isize))[2 as libc::c_int as usize] =
            (iode & 0xff as libc::c_ulong) << 22 as libc::c_int
                | (crs as libc::c_ulong & 0xffff as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(1 as libc::c_int as isize))[3 as libc::c_int as usize] =
            (deltan as libc::c_ulong & 0xffff as libc::c_ulong) << 14 as libc::c_int
                | ((m0 >> 24 as libc::c_int) as libc::c_ulong & 0xff as libc::c_ulong)
                    << 6 as libc::c_int;
        (*sbf.offset(1 as libc::c_int as isize))[4 as libc::c_int as usize] =
            (m0 as libc::c_ulong & 0xffffff as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(1 as libc::c_int as isize))[5 as libc::c_int as usize] =
            (cuc as libc::c_ulong & 0xffff as libc::c_ulong) << 14 as libc::c_int
                | (ecc >> 24 as libc::c_int & 0xff as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(1 as libc::c_int as isize))[6 as libc::c_int as usize] =
            (ecc & 0xffffff as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(1 as libc::c_int as isize))[7 as libc::c_int as usize] =
            (cus as libc::c_ulong & 0xffff as libc::c_ulong) << 14 as libc::c_int
                | (sqrta >> 24 as libc::c_int & 0xff as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(1 as libc::c_int as isize))[8 as libc::c_int as usize] =
            (sqrta & 0xffffff as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(1 as libc::c_int as isize))[9 as libc::c_int as usize] =
            (toe & 0xffff as libc::c_ulong) << 14 as libc::c_int;
        (*sbf.offset(2 as libc::c_int as isize))[0 as libc::c_int as usize] =
            (0x8b0000 as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(2 as libc::c_int as isize))[1 as libc::c_int as usize] =
            (0x3 as libc::c_ulong) << 8 as libc::c_int;
        (*sbf.offset(2 as libc::c_int as isize))[2 as libc::c_int as usize] =
            (cic as libc::c_ulong & 0xffff as libc::c_ulong) << 14 as libc::c_int
                | ((omg0 >> 24 as libc::c_int) as libc::c_ulong & 0xff as libc::c_ulong)
                    << 6 as libc::c_int;
        (*sbf.offset(2 as libc::c_int as isize))[3 as libc::c_int as usize] =
            (omg0 as libc::c_ulong & 0xffffff as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(2 as libc::c_int as isize))[4 as libc::c_int as usize] =
            (cis as libc::c_ulong & 0xffff as libc::c_ulong) << 14 as libc::c_int
                | ((inc0 >> 24 as libc::c_int) as libc::c_ulong & 0xff as libc::c_ulong)
                    << 6 as libc::c_int;
        (*sbf.offset(2 as libc::c_int as isize))[5 as libc::c_int as usize] =
            (inc0 as libc::c_ulong & 0xffffff as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(2 as libc::c_int as isize))[6 as libc::c_int as usize] =
            (crc as libc::c_ulong & 0xffff as libc::c_ulong) << 14 as libc::c_int
                | ((aop >> 24 as libc::c_int) as libc::c_ulong & 0xff as libc::c_ulong)
                    << 6 as libc::c_int;
        (*sbf.offset(2 as libc::c_int as isize))[7 as libc::c_int as usize] =
            (aop as libc::c_ulong & 0xffffff as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(2 as libc::c_int as isize))[8 as libc::c_int as usize] =
            (omgdot as libc::c_ulong & 0xffffff as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(2 as libc::c_int as isize))[9 as libc::c_int as usize] =
            (iode & 0xff as libc::c_ulong) << 22 as libc::c_int
                | (idot as libc::c_ulong & 0x3fff as libc::c_ulong) << 8 as libc::c_int;
        if ionoutc.vflg == 1 as libc::c_int {
            (*sbf.offset(3 as libc::c_int as isize))[0 as libc::c_int as usize] =
                (0x8b0000 as libc::c_ulong) << 6 as libc::c_int;
            (*sbf.offset(3 as libc::c_int as isize))[1 as libc::c_int as usize] =
                (0x4 as libc::c_ulong) << 8 as libc::c_int;
            (*sbf.offset(3 as libc::c_int as isize))[2 as libc::c_int as usize] = dataId
                << 28 as libc::c_int
                | sbf4_page18_svId << 22 as libc::c_int
                | (alpha0 as libc::c_ulong & 0xff as libc::c_ulong) << 14 as libc::c_int
                | (alpha1 as libc::c_ulong & 0xff as libc::c_ulong) << 6 as libc::c_int;
            (*sbf.offset(3 as libc::c_int as isize))[3 as libc::c_int as usize] =
                (alpha2 as libc::c_ulong & 0xff as libc::c_ulong) << 22 as libc::c_int
                    | (alpha3 as libc::c_ulong & 0xff as libc::c_ulong) << 14 as libc::c_int
                    | (beta0 as libc::c_ulong & 0xff as libc::c_ulong) << 6 as libc::c_int;
            (*sbf.offset(3 as libc::c_int as isize))[4 as libc::c_int as usize] =
                (beta1 as libc::c_ulong & 0xff as libc::c_ulong) << 22 as libc::c_int
                    | (beta2 as libc::c_ulong & 0xff as libc::c_ulong) << 14 as libc::c_int
                    | (beta3 as libc::c_ulong & 0xff as libc::c_ulong) << 6 as libc::c_int;
            (*sbf.offset(3 as libc::c_int as isize))[5 as libc::c_int as usize] =
                (A1 as libc::c_ulong & 0xffffff as libc::c_ulong) << 6 as libc::c_int;
            (*sbf.offset(3 as libc::c_int as isize))[6 as libc::c_int as usize] =
                ((A0 >> 8 as libc::c_int) as libc::c_ulong & 0xffffff as libc::c_ulong)
                    << 6 as libc::c_int;
            (*sbf.offset(3 as libc::c_int as isize))[7 as libc::c_int as usize] =
                (A0 as libc::c_ulong & 0xff as libc::c_ulong) << 22 as libc::c_int
                    | (tot & 0xff as libc::c_ulong) << 14 as libc::c_int
                    | (wnt & 0xff as libc::c_ulong) << 6 as libc::c_int;
            (*sbf.offset(3 as libc::c_int as isize))[8 as libc::c_int as usize] =
                (dtls as libc::c_ulong & 0xff as libc::c_ulong) << 22 as libc::c_int
                    | (wnlsf & 0xff as libc::c_ulong) << 14 as libc::c_int
                    | (dn & 0xff as libc::c_ulong) << 6 as libc::c_int;
            (*sbf.offset(3 as libc::c_int as isize))[9 as libc::c_int as usize] =
                (dtlsf & 0xff as libc::c_ulong) << 22 as libc::c_int;
        } else {
            (*sbf.offset(3 as libc::c_int as isize))[0 as libc::c_int as usize] =
                (0x8b0000 as libc::c_ulong) << 6 as libc::c_int;
            (*sbf.offset(3 as libc::c_int as isize))[1 as libc::c_int as usize] =
                (0x4 as libc::c_ulong) << 8 as libc::c_int;
            (*sbf.offset(3 as libc::c_int as isize))[2 as libc::c_int as usize] =
                dataId << 28 as libc::c_int | sbf4_page25_svId << 22 as libc::c_int;
            (*sbf.offset(3 as libc::c_int as isize))[3 as libc::c_int as usize] =
                0 as libc::c_ulong;
            (*sbf.offset(3 as libc::c_int as isize))[4 as libc::c_int as usize] =
                0 as libc::c_ulong;
            (*sbf.offset(3 as libc::c_int as isize))[5 as libc::c_int as usize] =
                0 as libc::c_ulong;
            (*sbf.offset(3 as libc::c_int as isize))[6 as libc::c_int as usize] =
                0 as libc::c_ulong;
            (*sbf.offset(3 as libc::c_int as isize))[7 as libc::c_int as usize] =
                0 as libc::c_ulong;
            (*sbf.offset(3 as libc::c_int as isize))[8 as libc::c_int as usize] =
                0 as libc::c_ulong;
            (*sbf.offset(3 as libc::c_int as isize))[9 as libc::c_int as usize] =
                0 as libc::c_ulong;
        }
        (*sbf.offset(4 as libc::c_int as isize))[0 as libc::c_int as usize] =
            (0x8b0000 as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(4 as libc::c_int as isize))[1 as libc::c_int as usize] =
            (0x5 as libc::c_ulong) << 8 as libc::c_int;
        (*sbf.offset(4 as libc::c_int as isize))[2 as libc::c_int as usize] = dataId
            << 28 as libc::c_int
            | sbf5_page25_svId << 22 as libc::c_int
            | (toa & 0xff as libc::c_ulong) << 14 as libc::c_int
            | (wna & 0xff as libc::c_ulong) << 6 as libc::c_int;
        (*sbf.offset(4 as libc::c_int as isize))[3 as libc::c_int as usize] = 0 as libc::c_ulong;
        (*sbf.offset(4 as libc::c_int as isize))[4 as libc::c_int as usize] = 0 as libc::c_ulong;
        (*sbf.offset(4 as libc::c_int as isize))[5 as libc::c_int as usize] = 0 as libc::c_ulong;
        (*sbf.offset(4 as libc::c_int as isize))[6 as libc::c_int as usize] = 0 as libc::c_ulong;
        (*sbf.offset(4 as libc::c_int as isize))[7 as libc::c_int as usize] = 0 as libc::c_ulong;
        (*sbf.offset(4 as libc::c_int as isize))[8 as libc::c_int as usize] = 0 as libc::c_ulong;
        (*sbf.offset(4 as libc::c_int as isize))[9 as libc::c_int as usize] = 0 as libc::c_ulong;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn countBits(mut v: libc::c_ulong) -> libc::c_ulong {
    let mut c: libc::c_ulong = 0;
    let S: [libc::c_int; 5] = [
        1 as libc::c_int,
        2 as libc::c_int,
        4 as libc::c_int,
        8 as libc::c_int,
        16 as libc::c_int,
    ];
    let B: [libc::c_ulong; 5] = [
        0x55555555 as libc::c_int as libc::c_ulong,
        0x33333333 as libc::c_int as libc::c_ulong,
        0xf0f0f0f as libc::c_int as libc::c_ulong,
        0xff00ff as libc::c_int as libc::c_ulong,
        0xffff as libc::c_int as libc::c_ulong,
    ];
    c = v;
    c = (c >> S[0 as libc::c_int as usize] & B[0 as libc::c_int as usize])
        .wrapping_add(c & B[0 as libc::c_int as usize]);
    c = (c >> S[1 as libc::c_int as usize] & B[1 as libc::c_int as usize])
        .wrapping_add(c & B[1 as libc::c_int as usize]);
    c = (c >> S[2 as libc::c_int as usize] & B[2 as libc::c_int as usize])
        .wrapping_add(c & B[2 as libc::c_int as usize]);
    c = (c >> S[3 as libc::c_int as usize] & B[3 as libc::c_int as usize])
        .wrapping_add(c & B[3 as libc::c_int as usize]);
    c = (c >> S[4 as libc::c_int as usize] & B[4 as libc::c_int as usize])
        .wrapping_add(c & B[4 as libc::c_int as usize]);
    c
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn computeChecksum(
    mut source: libc::c_ulong,
    mut nib: libc::c_int,
) -> libc::c_ulong {
    unsafe {
        let mut bmask: [libc::c_ulong; 6] = [
            0x3b1f3480 as libc::c_ulong,
            0x1d8f9a40 as libc::c_ulong,
            0x2ec7cd00 as libc::c_ulong,
            0x1763e680 as libc::c_ulong,
            0x2bb1f340 as libc::c_ulong,
            0xb7a89c0 as libc::c_ulong,
        ];
        let mut D: libc::c_ulong = 0;
        let mut d: libc::c_ulong = source & 0x3fffffc0 as libc::c_ulong;
        let mut D29: libc::c_ulong = source >> 31 as libc::c_int & 0x1 as libc::c_ulong;
        let mut D30: libc::c_ulong = source >> 30 as libc::c_int & 0x1 as libc::c_ulong;
        if nib != 0 {
            if D30
                .wrapping_add(countBits(bmask[4 as libc::c_int as usize] & d))
                .wrapping_rem(2 as libc::c_int as libc::c_ulong)
                != 0
            {
                d ^= (0x1 as libc::c_ulong) << 6 as libc::c_int;
            }
            if D29
                .wrapping_add(countBits(bmask[5 as libc::c_int as usize] & d))
                .wrapping_rem(2 as libc::c_int as libc::c_ulong)
                != 0
            {
                d ^= (0x1 as libc::c_ulong) << 7 as libc::c_int;
            }
        }
        D = d;
        if D30 != 0 {
            D ^= 0x3fffffc0 as libc::c_ulong;
        }
        D |= D29
            .wrapping_add(countBits(bmask[0 as libc::c_int as usize] & d))
            .wrapping_rem(2 as libc::c_int as libc::c_ulong)
            << 5 as libc::c_int;
        D |= D30
            .wrapping_add(countBits(bmask[1 as libc::c_int as usize] & d))
            .wrapping_rem(2 as libc::c_int as libc::c_ulong)
            << 4 as libc::c_int;
        D |= D29
            .wrapping_add(countBits(bmask[2 as libc::c_int as usize] & d))
            .wrapping_rem(2 as libc::c_int as libc::c_ulong)
            << 3 as libc::c_int;
        D |= D30
            .wrapping_add(countBits(bmask[3 as libc::c_int as usize] & d))
            .wrapping_rem(2 as libc::c_int as libc::c_ulong)
            << 2 as libc::c_int;
        D |= D30
            .wrapping_add(countBits(bmask[4 as libc::c_int as usize] & d))
            .wrapping_rem(2 as libc::c_int as libc::c_ulong)
            << 1 as libc::c_int;
        D |= D29
            .wrapping_add(countBits(bmask[5 as libc::c_int as usize] & d))
            .wrapping_rem(2 as libc::c_int as libc::c_ulong);
        D &= 0x3fffffff as libc::c_ulong;
        D
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn replaceExpDesignator(
    mut str: *mut libc::c_char,
    mut len: libc::c_int,
) -> libc::c_int {
    unsafe {
        let mut i: libc::c_int = 0;
        let mut n: libc::c_int = 0 as libc::c_int;
        i = 0 as libc::c_int;
        while i < len {
            if *str.offset(i as isize) as libc::c_int == 'D' as i32 {
                n += 1;
                *str.offset(i as isize) = 'E' as i32 as libc::c_char;
            }
            i += 1;
        }
        n
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn subGpsTime(mut g1: gpstime_t, mut g0: gpstime_t) -> libc::c_double {
    let mut dt: libc::c_double = 0.;
    dt = g1.sec - g0.sec;
    dt += (g1.week - g0.week) as libc::c_double * 604800.0f64;
    dt
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn incGpsTime(mut g0: gpstime_t, mut dt: libc::c_double) -> gpstime_t {
    unsafe {
        let mut g1: gpstime_t = gpstime_t { week: 0, sec: 0. };
        g1.week = g0.week;
        g1.sec = g0.sec + dt;
        g1.sec = round(g1.sec * 1000.0f64) / 1000.0f64;
        while g1.sec >= 604800.0f64 {
            g1.sec -= 604800.0f64;
            g1.week += 1;
            g1.week;
        }
        while g1.sec < 0.0f64 {
            g1.sec += 604800.0f64;
            g1.week -= 1;
            g1.week;
        }
        g1
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn readRinexNavAll(
    mut eph: *mut [ephem_t; 32],
    mut ionoutc: *mut ionoutc_t,
    mut fname: *const libc::c_char,
) -> libc::c_int {
    unsafe {
        let mut fp: *mut FILE = std::ptr::null_mut::<FILE>();
        let mut ieph: libc::c_int = 0;
        let mut sv: libc::c_int = 0;
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
        let mut dt: libc::c_double = 0.;
        let mut flags: libc::c_int = 0 as libc::c_int;
        fp = fopen(fname, b"rt\0" as *const u8 as *const libc::c_char);
        if fp.is_null() {
            return -(1 as libc::c_int);
        }
        ieph = 0 as libc::c_int;
        while ieph < 15 as libc::c_int {
            sv = 0 as libc::c_int;
            while sv < 32 as libc::c_int {
                (*eph.offset(ieph as isize))[sv as usize].vflg = 0 as libc::c_int;
                sv += 1;
            }
            ieph += 1;
        }
        while !(fgets(str.as_mut_ptr(), 100 as libc::c_int, fp)).is_null() {
            if strncmp(
                str.as_mut_ptr().offset(60 as libc::c_int as isize),
                b"END OF HEADER\0" as *const u8 as *const libc::c_char,
                13 as libc::c_int as libc::c_ulong,
            ) == 0 as libc::c_int
            {
                break;
            }
            if strncmp(
                str.as_mut_ptr().offset(60 as libc::c_int as isize),
                b"ION ALPHA\0" as *const u8 as *const libc::c_char,
                9 as libc::c_int as libc::c_ulong,
            ) == 0 as libc::c_int
            {
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(2 as libc::c_int as isize),
                    12 as libc::c_int as libc::c_ulong,
                );
                tmp[12 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12 as libc::c_int);
                (*ionoutc).alpha0 = atof(tmp.as_mut_ptr());
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(14 as libc::c_int as isize),
                    12 as libc::c_int as libc::c_ulong,
                );
                tmp[12 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12 as libc::c_int);
                (*ionoutc).alpha1 = atof(tmp.as_mut_ptr());
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(26 as libc::c_int as isize),
                    12 as libc::c_int as libc::c_ulong,
                );
                tmp[12 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12 as libc::c_int);
                (*ionoutc).alpha2 = atof(tmp.as_mut_ptr());
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(38 as libc::c_int as isize),
                    12 as libc::c_int as libc::c_ulong,
                );
                tmp[12 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12 as libc::c_int);
                (*ionoutc).alpha3 = atof(tmp.as_mut_ptr());
                flags |= 0x1 as libc::c_int;
            } else if strncmp(
                str.as_mut_ptr().offset(60 as libc::c_int as isize),
                b"ION BETA\0" as *const u8 as *const libc::c_char,
                8 as libc::c_int as libc::c_ulong,
            ) == 0 as libc::c_int
            {
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(2 as libc::c_int as isize),
                    12 as libc::c_int as libc::c_ulong,
                );
                tmp[12 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12 as libc::c_int);
                (*ionoutc).beta0 = atof(tmp.as_mut_ptr());
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(14 as libc::c_int as isize),
                    12 as libc::c_int as libc::c_ulong,
                );
                tmp[12 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12 as libc::c_int);
                (*ionoutc).beta1 = atof(tmp.as_mut_ptr());
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(26 as libc::c_int as isize),
                    12 as libc::c_int as libc::c_ulong,
                );
                tmp[12 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12 as libc::c_int);
                (*ionoutc).beta2 = atof(tmp.as_mut_ptr());
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(38 as libc::c_int as isize),
                    12 as libc::c_int as libc::c_ulong,
                );
                tmp[12 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 12 as libc::c_int);
                (*ionoutc).beta3 = atof(tmp.as_mut_ptr());
                flags |= (0x1 as libc::c_int) << 1 as libc::c_int;
            } else if strncmp(
                str.as_mut_ptr().offset(60 as libc::c_int as isize),
                b"DELTA-UTC\0" as *const u8 as *const libc::c_char,
                9 as libc::c_int as libc::c_ulong,
            ) == 0 as libc::c_int
            {
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(3 as libc::c_int as isize),
                    19 as libc::c_int as libc::c_ulong,
                );
                tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
                (*ionoutc).A0 = atof(tmp.as_mut_ptr());
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(22 as libc::c_int as isize),
                    19 as libc::c_int as libc::c_ulong,
                );
                tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
                replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
                (*ionoutc).A1 = atof(tmp.as_mut_ptr());
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(41 as libc::c_int as isize),
                    9 as libc::c_int as libc::c_ulong,
                );
                tmp[9 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
                (*ionoutc).tot = atoi(tmp.as_mut_ptr());
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr().offset(50 as libc::c_int as isize),
                    9 as libc::c_int as libc::c_ulong,
                );
                tmp[9 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
                (*ionoutc).wnt = atoi(tmp.as_mut_ptr());
                if (*ionoutc).tot % 4096 as libc::c_int == 0 as libc::c_int {
                    flags |= (0x1 as libc::c_int) << 2 as libc::c_int;
                }
            } else if strncmp(
                str.as_mut_ptr().offset(60 as libc::c_int as isize),
                b"LEAP SECONDS\0" as *const u8 as *const libc::c_char,
                12 as libc::c_int as libc::c_ulong,
            ) == 0 as libc::c_int
            {
                strncpy(
                    tmp.as_mut_ptr(),
                    str.as_mut_ptr(),
                    6 as libc::c_int as libc::c_ulong,
                );
                tmp[6 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
                (*ionoutc).dtls = atoi(tmp.as_mut_ptr());
                flags |= (0x1 as libc::c_int) << 3 as libc::c_int;
            }
        }
        (*ionoutc).vflg = 0 as libc::c_int;
        if flags == 0xf as libc::c_int {
            (*ionoutc).vflg = 1 as libc::c_int;
        }
        g0.week = -(1 as libc::c_int);
        ieph = 0 as libc::c_int;
        while !(fgets(str.as_mut_ptr(), 100 as libc::c_int, fp)).is_null() {
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr(),
                2 as libc::c_int as libc::c_ulong,
            );
            tmp[2 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            sv = atoi(tmp.as_mut_ptr()) - 1 as libc::c_int;
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(3 as libc::c_int as isize),
                2 as libc::c_int as libc::c_ulong,
            );
            tmp[2 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            t.y = atoi(tmp.as_mut_ptr()) + 2000 as libc::c_int;
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(6 as libc::c_int as isize),
                2 as libc::c_int as libc::c_ulong,
            );
            tmp[2 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            t.m = atoi(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(9 as libc::c_int as isize),
                2 as libc::c_int as libc::c_ulong,
            );
            tmp[2 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            t.d = atoi(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(12 as libc::c_int as isize),
                2 as libc::c_int as libc::c_ulong,
            );
            tmp[2 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            t.hh = atoi(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(15 as libc::c_int as isize),
                2 as libc::c_int as libc::c_ulong,
            );
            tmp[2 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            t.mm = atoi(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(18 as libc::c_int as isize),
                4 as libc::c_int as libc::c_ulong,
            );
            tmp[2 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            t.sec = atof(tmp.as_mut_ptr());
            date2gps(&mut t, &mut g);
            if g0.week == -(1 as libc::c_int) {
                g0 = g;
            }
            dt = subGpsTime(g, g0);
            if dt > 3600.0f64 {
                g0 = g;
                ieph += 1;
                if ieph >= 15 as libc::c_int {
                    break;
                }
            }
            (*eph.offset(ieph as isize))[sv as usize].t = t;
            (*eph.offset(ieph as isize))[sv as usize].toc = g;
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(22 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].af0 = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(41 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].af1 = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(60 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].af2 = atof(tmp.as_mut_ptr());
            if (fgets(str.as_mut_ptr(), 100 as libc::c_int, fp)).is_null() {
                break;
            }
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(3 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].iode = atof(tmp.as_mut_ptr()) as libc::c_int;
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(22 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].crs = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(41 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].deltan = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(60 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].m0 = atof(tmp.as_mut_ptr());
            if (fgets(str.as_mut_ptr(), 100 as libc::c_int, fp)).is_null() {
                break;
            }
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(3 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].cuc = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(22 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].ecc = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(41 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].cus = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(60 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].sqrta = atof(tmp.as_mut_ptr());
            if (fgets(str.as_mut_ptr(), 100 as libc::c_int, fp)).is_null() {
                break;
            }
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(3 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].toe.sec = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(22 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].cic = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(41 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].omg0 = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(60 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].cis = atof(tmp.as_mut_ptr());
            if (fgets(str.as_mut_ptr(), 100 as libc::c_int, fp)).is_null() {
                break;
            }
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(3 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].inc0 = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(22 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].crc = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(41 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].aop = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(60 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].omgdot = atof(tmp.as_mut_ptr());
            if (fgets(str.as_mut_ptr(), 100 as libc::c_int, fp)).is_null() {
                break;
            }
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(3 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].idot = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(22 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].codeL2 =
                atof(tmp.as_mut_ptr()) as libc::c_int;
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(41 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].toe.week =
                atof(tmp.as_mut_ptr()) as libc::c_int;
            if (fgets(str.as_mut_ptr(), 100 as libc::c_int, fp)).is_null() {
                break;
            }
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(22 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].svhlth =
                atof(tmp.as_mut_ptr()) as libc::c_int;
            if (*eph.offset(ieph as isize))[sv as usize].svhlth > 0 as libc::c_int
                && (*eph.offset(ieph as isize))[sv as usize].svhlth < 32 as libc::c_int
            {
                (*eph.offset(ieph as isize))[sv as usize].svhlth += 32 as libc::c_int;
            }
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(41 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].tgd = atof(tmp.as_mut_ptr());
            strncpy(
                tmp.as_mut_ptr(),
                str.as_mut_ptr().offset(60 as libc::c_int as isize),
                19 as libc::c_int as libc::c_ulong,
            );
            tmp[19 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            replaceExpDesignator(tmp.as_mut_ptr(), 19 as libc::c_int);
            (*eph.offset(ieph as isize))[sv as usize].iodc = atof(tmp.as_mut_ptr()) as libc::c_int;
            if (fgets(str.as_mut_ptr(), 100 as libc::c_int, fp)).is_null() {
                break;
            }
            (*eph.offset(ieph as isize))[sv as usize].vflg = 1 as libc::c_int;
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
        if g0.week >= 0 as libc::c_int {
            ieph += 1 as libc::c_int;
        }
        ieph
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ionosphericDelay(
    mut ionoutc: *const ionoutc_t,
    mut g: gpstime_t,
    mut llh: *mut libc::c_double,
    mut azel: *mut libc::c_double,
) -> libc::c_double {
    unsafe {
        let mut iono_delay: libc::c_double = 0.0f64;
        let mut E: libc::c_double = 0.;
        let mut phi_u: libc::c_double = 0.;
        let mut lam_u: libc::c_double = 0.;
        let mut F: libc::c_double = 0.;
        if (*ionoutc).enable == 0 as libc::c_int {
            return 0.0f64;
        }
        E = *azel.offset(1 as libc::c_int as isize) / 3.1415926535898f64;
        phi_u = *llh.offset(0 as libc::c_int as isize) / 3.1415926535898f64;
        lam_u = *llh.offset(1 as libc::c_int as isize) / 3.1415926535898f64;
        F = 1.0f64 + 16.0f64 * pow(0.53f64 - E, 3.0f64);
        if (*ionoutc).vflg == 0 as libc::c_int {
            iono_delay = F * 5.0e-9f64 * 2.99792458e8f64;
        } else {
            let mut t: libc::c_double = 0.;
            let mut psi: libc::c_double = 0.;
            let mut phi_i: libc::c_double = 0.;
            let mut lam_i: libc::c_double = 0.;
            let mut phi_m: libc::c_double = 0.;
            let mut phi_m2: libc::c_double = 0.;
            let mut phi_m3: libc::c_double = 0.;
            let mut AMP: libc::c_double = 0.;
            let mut PER: libc::c_double = 0.;
            let mut X: libc::c_double = 0.;
            let mut X2: libc::c_double = 0.;
            let mut X4: libc::c_double = 0.;
            psi = 0.0137f64 / (E + 0.11f64) - 0.022f64;
            phi_i = phi_u + psi * cos(*azel.offset(0 as libc::c_int as isize));
            if phi_i > 0.416f64 {
                phi_i = 0.416f64;
            } else if phi_i < -0.416f64 {
                phi_i = -0.416f64;
            }
            lam_i = lam_u
                + psi * sin(*azel.offset(0 as libc::c_int as isize))
                    / cos(phi_i * 3.1415926535898f64);
            phi_m = phi_i + 0.064f64 * cos((lam_i - 1.617f64) * 3.1415926535898f64);
            phi_m2 = phi_m * phi_m;
            phi_m3 = phi_m2 * phi_m;
            AMP = (*ionoutc).alpha0
                + (*ionoutc).alpha1 * phi_m
                + (*ionoutc).alpha2 * phi_m2
                + (*ionoutc).alpha3 * phi_m3;
            if AMP < 0.0f64 {
                AMP = 0.0f64;
            }
            PER = (*ionoutc).beta0
                + (*ionoutc).beta1 * phi_m
                + (*ionoutc).beta2 * phi_m2
                + (*ionoutc).beta3 * phi_m3;
            if PER < 72000.0f64 {
                PER = 72000.0f64;
            }
            t = 86400.0f64 / 2.0f64 * lam_i + g.sec;
            while t >= 86400.0f64 {
                t -= 86400.0f64;
            }
            while t < 0 as libc::c_int as libc::c_double {
                t += 86400.0f64;
            }
            X = 2.0f64 * 3.1415926535898f64 * (t - 50400.0f64) / PER;
            if fabs(X) < 1.57f64 {
                X2 = X * X;
                X4 = X2 * X2;
                iono_delay =
                    F * (5.0e-9f64 + AMP * (1.0f64 - X2 / 2.0f64 + X4 / 24.0f64)) * 2.99792458e8f64;
            } else {
                iono_delay = F * 5.0e-9f64 * 2.99792458e8f64;
            }
        }
        iono_delay
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn computeRange(
    mut rho: *mut range_t,
    mut eph: ephem_t,
    mut ionoutc: *mut ionoutc_t,
    mut g: gpstime_t,
    mut xyz_0: *mut libc::c_double,
) {
    unsafe {
        let mut pos: [libc::c_double; 3] = [0.; 3];
        let mut vel: [libc::c_double; 3] = [0.; 3];
        let mut clk: [libc::c_double; 2] = [0.; 2];
        let mut los: [libc::c_double; 3] = [0.; 3];
        let mut tau: libc::c_double = 0.;
        let mut range: libc::c_double = 0.;
        let mut rate: libc::c_double = 0.;
        let mut xrot: libc::c_double = 0.;
        let mut yrot: libc::c_double = 0.;
        let mut llh: [libc::c_double; 3] = [0.; 3];
        let mut neu: [libc::c_double; 3] = [0.; 3];
        let mut tmat: [[libc::c_double; 3]; 3] = [[0.; 3]; 3];
        satpos(eph, g, pos.as_mut_ptr(), vel.as_mut_ptr(), clk.as_mut_ptr());
        subVect(
            los.as_mut_ptr(),
            pos.as_mut_ptr(),
            xyz_0 as *const libc::c_double,
        );
        tau = normVect(los.as_mut_ptr()) / 2.99792458e8f64;
        pos[0 as libc::c_int as usize] -= vel[0 as libc::c_int as usize] * tau;
        pos[1 as libc::c_int as usize] -= vel[1 as libc::c_int as usize] * tau;
        pos[2 as libc::c_int as usize] -= vel[2 as libc::c_int as usize] * tau;
        xrot = pos[0 as libc::c_int as usize]
            + pos[1 as libc::c_int as usize] * 7.2921151467e-5f64 * tau;
        yrot = pos[1 as libc::c_int as usize]
            - pos[0 as libc::c_int as usize] * 7.2921151467e-5f64 * tau;
        pos[0 as libc::c_int as usize] = xrot;
        pos[1 as libc::c_int as usize] = yrot;
        subVect(
            los.as_mut_ptr(),
            pos.as_mut_ptr(),
            xyz_0 as *const libc::c_double,
        );
        range = normVect(los.as_mut_ptr());
        (*rho).d = range;
        (*rho).range = range - 2.99792458e8f64 * clk[0 as libc::c_int as usize];
        rate = dotProd(vel.as_mut_ptr(), los.as_mut_ptr()) / range;
        (*rho).rate = rate;
        (*rho).g = g;
        xyz2llh(xyz_0 as *const libc::c_double, llh.as_mut_ptr());
        ltcmat(llh.as_mut_ptr(), tmat.as_mut_ptr());
        ecef2neu(los.as_mut_ptr(), tmat.as_mut_ptr(), neu.as_mut_ptr());
        neu2azel(((*rho).azel).as_mut_ptr(), neu.as_mut_ptr());
        (*rho).iono_delay =
            ionosphericDelay(ionoutc, g, llh.as_mut_ptr(), ((*rho).azel).as_mut_ptr());
        (*rho).range += (*rho).iono_delay;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn computeCodePhase(
    mut chan: *mut channel_t,
    mut rho1: range_t,
    mut dt: libc::c_double,
) {
    unsafe {
        let mut ms: libc::c_double = 0.;
        let mut ims: libc::c_int = 0;
        let mut rhorate: libc::c_double = 0.;
        rhorate = (rho1.range - (*chan).rho0.range) / dt;
        (*chan).f_carr = -rhorate / 0.190293672798365f64;
        (*chan).f_code = 1.023e6f64 + (*chan).f_carr * (1.0f64 / 1540.0f64);
        ms = (subGpsTime((*chan).rho0.g, (*chan).g0) + 6.0f64
            - (*chan).rho0.range / 2.99792458e8f64)
            * 1000.0f64;
        ims = ms as libc::c_int;
        (*chan).code_phase = (ms - ims as libc::c_double) * 1023 as libc::c_int as libc::c_double;
        (*chan).iword = ims / 600 as libc::c_int;
        ims -= (*chan).iword * 600 as libc::c_int;
        (*chan).ibit = ims / 20 as libc::c_int;
        ims -= (*chan).ibit * 20 as libc::c_int;
        (*chan).icode = ims;
        (*chan).codeCA = (*chan).ca[(*chan).code_phase as libc::c_int as usize] * 2 as libc::c_int
            - 1 as libc::c_int;
        (*chan).dataBit = ((*chan).dwrd[(*chan).iword as usize]
            >> (29 as libc::c_int - (*chan).ibit)
            & 0x1 as libc::c_ulong) as libc::c_int
            * 2 as libc::c_int
            - 1 as libc::c_int;
        (*chan).rho0 = rho1;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn readUserMotion(
    mut xyz_0: *mut [libc::c_double; 3],
    mut filename: *const libc::c_char,
) -> libc::c_int {
    unsafe {
        let mut fp: *mut FILE = std::ptr::null_mut::<FILE>();
        let mut numd: libc::c_int = 0;
        let mut str: [libc::c_char; 100] = [0; 100];
        let mut t: libc::c_double = 0.;
        let mut x: libc::c_double = 0.;
        let mut y: libc::c_double = 0.;
        let mut z: libc::c_double = 0.;
        fp = fopen(filename, b"rt\0" as *const u8 as *const libc::c_char);
        if fp.is_null() {
            return -(1 as libc::c_int);
        }
        numd = 0 as libc::c_int;
        while numd < 3000 as libc::c_int {
            if (fgets(str.as_mut_ptr(), 100 as libc::c_int, fp)).is_null() {
                break;
            }
            if -(1 as libc::c_int)
                == sscanf(
                    str.as_mut_ptr(),
                    b"%lf,%lf,%lf,%lf\0" as *const u8 as *const libc::c_char,
                    &mut t as *mut libc::c_double,
                    &mut x as *mut libc::c_double,
                    &mut y as *mut libc::c_double,
                    &mut z as *mut libc::c_double,
                )
            {
                break;
            }
            (*xyz_0.offset(numd as isize))[0 as libc::c_int as usize] = x;
            (*xyz_0.offset(numd as isize))[1 as libc::c_int as usize] = y;
            (*xyz_0.offset(numd as isize))[2 as libc::c_int as usize] = z;
            numd += 1;
        }
        fclose(fp);
        numd
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn readUserMotionLLH(
    mut xyz_0: *mut [libc::c_double; 3],
    mut filename: *const libc::c_char,
) -> libc::c_int {
    unsafe {
        let mut fp: *mut FILE = std::ptr::null_mut::<FILE>();
        let mut numd: libc::c_int = 0;
        let mut t: libc::c_double = 0.;
        let mut llh: [libc::c_double; 3] = [0.; 3];
        let mut str: [libc::c_char; 100] = [0; 100];
        fp = fopen(filename, b"rt\0" as *const u8 as *const libc::c_char);
        if fp.is_null() {
            return -(1 as libc::c_int);
        }
        numd = 0 as libc::c_int;
        while numd < 3000 as libc::c_int {
            if (fgets(str.as_mut_ptr(), 100 as libc::c_int, fp)).is_null() {
                break;
            }
            if -(1 as libc::c_int)
                == sscanf(
                    str.as_mut_ptr(),
                    b"%lf,%lf,%lf,%lf\0" as *const u8 as *const libc::c_char,
                    &mut t as *mut libc::c_double,
                    &mut *llh.as_mut_ptr().offset(0 as libc::c_int as isize) as *mut libc::c_double,
                    &mut *llh.as_mut_ptr().offset(1 as libc::c_int as isize) as *mut libc::c_double,
                    &mut *llh.as_mut_ptr().offset(2 as libc::c_int as isize) as *mut libc::c_double,
                )
            {
                break;
            }
            if llh[0 as libc::c_int as usize] > 90.0f64
                || llh[0 as libc::c_int as usize] < -90.0f64
                || llh[1 as libc::c_int as usize] > 180.0f64
                || llh[1 as libc::c_int as usize] < -180.0f64
            {
                eprintln!(
                    "ERROR: Invalid file format (time[s], latitude[deg], longitude[deg], height [m].\n"
                );
                numd = 0 as libc::c_int;
                break;
            } else {
                llh[0 as libc::c_int as usize] /= 57.2957795131f64;
                llh[1 as libc::c_int as usize] /= 57.2957795131f64;
                llh2xyz(
                    llh.as_mut_ptr(),
                    (*xyz_0.offset(numd as isize)).as_mut_ptr(),
                );
                numd += 1;
            }
        }
        fclose(fp);
        numd
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn readNmeaGGA(
    mut xyz_0: *mut [libc::c_double; 3],
    mut filename: *const libc::c_char,
) -> libc::c_int {
    unsafe {
        let mut fp: *mut FILE = std::ptr::null_mut::<FILE>();
        let mut numd: libc::c_int = 0 as libc::c_int;
        let mut str: [libc::c_char; 100] = [0; 100];
        let mut token: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
        let mut llh: [libc::c_double; 3] = [0.; 3];
        let mut pos: [libc::c_double; 3] = [0.; 3];
        let mut tmp: [libc::c_char; 8] = [0; 8];
        fp = fopen(filename, b"rt\0" as *const u8 as *const libc::c_char);
        if fp.is_null() {
            return -(1 as libc::c_int);
        }
        while !(fgets(str.as_mut_ptr(), 100 as libc::c_int, fp)).is_null() {
            token = strtok(str.as_mut_ptr(), b",\0" as *const u8 as *const libc::c_char);
            if strncmp(
                token.offset(3 as libc::c_int as isize),
                b"GGA\0" as *const u8 as *const libc::c_char,
                3 as libc::c_int as libc::c_ulong,
            ) != 0 as libc::c_int
            {
                continue;
            }
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            strncpy(tmp.as_mut_ptr(), token, 2 as libc::c_int as libc::c_ulong);
            tmp[2 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            llh[0 as libc::c_int as usize] =
                atof(tmp.as_mut_ptr()) + atof(token.offset(2 as libc::c_int as isize)) / 60.0f64;
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            if *token.offset(0 as libc::c_int as isize) as libc::c_int == 'S' as i32 {
                llh[0 as libc::c_int as usize] *= -1.0f64;
            }
            llh[0 as libc::c_int as usize] /= 57.2957795131f64;
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            strncpy(tmp.as_mut_ptr(), token, 3 as libc::c_int as libc::c_ulong);
            tmp[3 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
            llh[1 as libc::c_int as usize] =
                atof(tmp.as_mut_ptr()) + atof(token.offset(3 as libc::c_int as isize)) / 60.0f64;
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            if *token.offset(0 as libc::c_int as isize) as libc::c_int == 'W' as i32 {
                llh[1 as libc::c_int as usize] *= -1.0f64;
            }
            llh[1 as libc::c_int as usize] /= 57.2957795131f64;
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            llh[2 as libc::c_int as usize] = atof(token);
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            llh[2 as libc::c_int as usize] += atof(token);
            llh2xyz(llh.as_mut_ptr(), pos.as_mut_ptr());
            (*xyz_0.offset(numd as isize))[0 as libc::c_int as usize] =
                pos[0 as libc::c_int as usize];
            (*xyz_0.offset(numd as isize))[1 as libc::c_int as usize] =
                pos[1 as libc::c_int as usize];
            (*xyz_0.offset(numd as isize))[2 as libc::c_int as usize] =
                pos[2 as libc::c_int as usize];
            numd += 1;
            if numd >= 3000 as libc::c_int {
                break;
            }
        }
        fclose(fp);
        numd
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn generateNavMsg(
    mut g: gpstime_t,
    mut chan: *mut channel_t,
    mut init: libc::c_int,
) -> libc::c_int {
    unsafe {
        let mut iwrd: libc::c_int = 0;
        let mut isbf: libc::c_int = 0;
        let mut g0: gpstime_t = gpstime_t { week: 0, sec: 0. };
        let mut wn: libc::c_ulong = 0;
        let mut tow: libc::c_ulong = 0;
        let mut sbfwrd: libc::c_uint = 0;
        let mut prevwrd: libc::c_ulong = 0;
        let mut nib: libc::c_int = 0;
        g0.week = g.week;
        g0.sec = ((g.sec + 0.5f64) as libc::c_ulong).wrapping_div(30 as libc::c_ulong)
            as libc::c_double
            * 30.0f64;
        (*chan).g0 = g0;
        wn = (g0.week % 1024 as libc::c_int) as libc::c_ulong;
        tow = (g0.sec as libc::c_ulong).wrapping_div(6 as libc::c_ulong);
        if init == 1 as libc::c_int {
            prevwrd = 0 as libc::c_ulong;
            iwrd = 0 as libc::c_int;
            while iwrd < 10 as libc::c_int {
                sbfwrd = (*chan).sbf[4 as libc::c_int as usize][iwrd as usize] as libc::c_uint;
                if iwrd == 1 as libc::c_int {
                    sbfwrd = (sbfwrd as libc::c_ulong
                        | (tow & 0x1ffff as libc::c_ulong) << 13 as libc::c_int)
                        as libc::c_uint;
                }
                sbfwrd = (sbfwrd as libc::c_ulong
                    | prevwrd << 30 as libc::c_int & 0xc0000000 as libc::c_ulong)
                    as libc::c_uint;
                nib = if iwrd == 1 as libc::c_int || iwrd == 9 as libc::c_int {
                    1 as libc::c_int
                } else {
                    0 as libc::c_int
                };
                (*chan).dwrd[iwrd as usize] = computeChecksum(sbfwrd as libc::c_ulong, nib);
                prevwrd = (*chan).dwrd[iwrd as usize];
                iwrd += 1;
            }
        } else {
            iwrd = 0 as libc::c_int;
            while iwrd < 10 as libc::c_int {
                (*chan).dwrd[iwrd as usize] =
                    (*chan).dwrd[(10 as libc::c_int * 5 as libc::c_int + iwrd) as usize];
                prevwrd = (*chan).dwrd[iwrd as usize];
                iwrd += 1;
            }
        }
        isbf = 0 as libc::c_int;
        while isbf < 5 as libc::c_int {
            tow = tow.wrapping_add(1);
            iwrd = 0 as libc::c_int;
            while iwrd < 10 as libc::c_int {
                sbfwrd = (*chan).sbf[isbf as usize][iwrd as usize] as libc::c_uint;
                if isbf == 0 as libc::c_int && iwrd == 2 as libc::c_int {
                    sbfwrd = (sbfwrd as libc::c_ulong
                        | (wn & 0x3ff as libc::c_ulong) << 20 as libc::c_int)
                        as libc::c_uint;
                }
                if iwrd == 1 as libc::c_int {
                    sbfwrd = (sbfwrd as libc::c_ulong
                        | (tow & 0x1ffff as libc::c_ulong) << 13 as libc::c_int)
                        as libc::c_uint;
                }
                sbfwrd = (sbfwrd as libc::c_ulong
                    | prevwrd << 30 as libc::c_int & 0xc0000000 as libc::c_ulong)
                    as libc::c_uint;
                nib = if iwrd == 1 as libc::c_int || iwrd == 9 as libc::c_int {
                    1 as libc::c_int
                } else {
                    0 as libc::c_int
                };
                (*chan).dwrd[((isbf + 1 as libc::c_int) * 10 as libc::c_int + iwrd) as usize] =
                    computeChecksum(sbfwrd as libc::c_ulong, nib);
                prevwrd =
                    (*chan).dwrd[((isbf + 1 as libc::c_int) * 10 as libc::c_int + iwrd) as usize];
                iwrd += 1;
            }
            isbf += 1;
        }
        1 as libc::c_int
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn checkSatVisibility(
    mut eph: ephem_t,
    mut g: gpstime_t,
    mut xyz_0: *mut libc::c_double,
    mut elvMask: libc::c_double,
    mut azel: *mut libc::c_double,
) -> libc::c_int {
    unsafe {
        let mut llh: [libc::c_double; 3] = [0.; 3];
        let mut neu: [libc::c_double; 3] = [0.; 3];
        let mut pos: [libc::c_double; 3] = [0.; 3];
        let mut vel: [libc::c_double; 3] = [0.; 3];
        let mut clk: [libc::c_double; 3] = [0.; 3];
        let mut los: [libc::c_double; 3] = [0.; 3];
        let mut tmat: [[libc::c_double; 3]; 3] = [[0.; 3]; 3];
        if eph.vflg != 1 as libc::c_int {
            return -(1 as libc::c_int);
        }
        xyz2llh(xyz_0, llh.as_mut_ptr());
        ltcmat(llh.as_mut_ptr(), tmat.as_mut_ptr());
        satpos(eph, g, pos.as_mut_ptr(), vel.as_mut_ptr(), clk.as_mut_ptr());
        subVect(los.as_mut_ptr(), pos.as_mut_ptr(), xyz_0);
        ecef2neu(los.as_mut_ptr(), tmat.as_mut_ptr(), neu.as_mut_ptr());
        neu2azel(azel, neu.as_mut_ptr());
        if *azel.offset(1 as libc::c_int as isize) * 57.2957795131f64 > elvMask {
            return 1 as libc::c_int;
        }
        0 as libc::c_int
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocateChannel(
    mut chan: *mut channel_t,
    mut eph: *mut ephem_t,
    mut ionoutc: ionoutc_t,
    mut grx: gpstime_t,
    mut xyz_0: *mut libc::c_double,
    mut _elvMask: libc::c_double,
) -> libc::c_int {
    unsafe {
        let mut nsat: libc::c_int = 0 as libc::c_int;
        let mut i: libc::c_int = 0;
        let mut sv: libc::c_int = 0;
        let mut azel: [libc::c_double; 2] = [0.; 2];
        let mut rho: range_t = range_t {
            g: gpstime_t { week: 0, sec: 0. },
            range: 0.,
            rate: 0.,
            d: 0.,
            azel: [0.; 2],
            iono_delay: 0.,
        };
        let mut ref_0: [libc::c_double; 3] = [0.0f64, 0., 0.];
        #[allow(unused_variables)]
        let mut r_ref: libc::c_double = 0.;
        #[allow(unused_variables)]
        let mut r_xyz: libc::c_double = 0.;
        let mut phase_ini: libc::c_double = 0.;
        sv = 0 as libc::c_int;
        while sv < 32 as libc::c_int {
            if checkSatVisibility(
                *eph.offset(sv as isize),
                grx,
                xyz_0,
                0.0f64,
                azel.as_mut_ptr(),
            ) == 1 as libc::c_int
            {
                nsat += 1;
                if allocatedSat[sv as usize] == -(1 as libc::c_int) {
                    i = 0 as libc::c_int;
                    while i < 16 as libc::c_int {
                        if (*chan.offset(i as isize)).prn == 0 as libc::c_int {
                            (*chan.offset(i as isize)).prn = sv + 1 as libc::c_int;
                            (*chan.offset(i as isize)).azel[0 as libc::c_int as usize] =
                                azel[0 as libc::c_int as usize];
                            (*chan.offset(i as isize)).azel[1 as libc::c_int as usize] =
                                azel[1 as libc::c_int as usize];
                            codegen(
                                ((*chan.offset(i as isize)).ca).as_mut_ptr(),
                                (*chan.offset(i as isize)).prn,
                            );
                            eph2sbf(
                                *eph.offset(sv as isize),
                                ionoutc,
                                ((*chan.offset(i as isize)).sbf).as_mut_ptr(),
                            );
                            generateNavMsg(grx, &mut *chan.offset(i as isize), 1 as libc::c_int);
                            computeRange(
                                &mut rho,
                                *eph.offset(sv as isize),
                                &mut ionoutc,
                                grx,
                                xyz_0,
                            );
                            (*chan.offset(i as isize)).rho0 = rho;
                            r_xyz = rho.range;
                            computeRange(
                                &mut rho,
                                *eph.offset(sv as isize),
                                &mut ionoutc,
                                grx,
                                ref_0.as_mut_ptr(),
                            );
                            r_ref = rho.range;
                            phase_ini = 0.0f64;
                            phase_ini -= floor(phase_ini);
                            (*chan.offset(i as isize)).carr_phase =
                                (512.0f64 * 65536.0f64 * phase_ini) as libc::c_uint;
                            break;
                        } else {
                            i += 1;
                        }
                    }
                    if i < 16 as libc::c_int {
                        allocatedSat[sv as usize] = i;
                    }
                }
            } else if allocatedSat[sv as usize] >= 0 as libc::c_int {
                (*chan.offset(allocatedSat[sv as usize] as isize)).prn = 0 as libc::c_int;
                allocatedSat[sv as usize] = -(1 as libc::c_int);
            }
            sv += 1;
        }
        nsat
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn usage() {
    eprintln!(
        "Usage: gps-sdr-sim [options]\nOptions:\n  -e <gps_nav>     RINEX navigation file for GPS ephemerides (required)\n  -u <user_motion> User motion file in ECEF x, y, z format (dynamic mode)\n  -x <user_motion> User motion file in lat, lon, height format (dynamic mode)\n  -g <nmea_gga>    NMEA GGA stream (dynamic mode)\n  -c <location>    ECEF X,Y,Z in meters (static mode) e.g. 3967283.154,1022538.181,4872414.484\n  -l <location>    Lat, lon, height (static mode) e.g. 35.681298,139.766247,10.0\n  -L <wnslf,dn,dtslf> User leap future event in GPS week number, day number, next leap second e.g. 2347,3,19\n  -t <date,time>   Scenario start time YYYY/MM/DD,hh:mm:ss\n  -T <date,time>   Overwrite TOC and TOE to scenario start time\n  -d <duration>    Duration [sec] (dynamic mode max: {}, static mode max: {})\n  -o <output>      I/Q sampling data file (default: gpssim.bin)\n  -s <frequency>   Sampling frequency [Hz] (default: 2600000)\n  -b <iq_bits>     I/Q data format [1/8/16] (default: 16)\n  -i               Disable ionospheric delay for spacecraft scenario\n  -p [fixed_gain]  Disable path loss and hold power level constant\n  -v               Show details about simulated channels\n",
        3000 as libc::c_int as libc::c_double / 10.0f64,
        86400 as libc::c_int,
    );
}
unsafe fn main_0(mut argc: libc::c_int, mut argv: *mut *mut libc::c_char) -> libc::c_int {
    unsafe {
        let mut tstart: clock_t = 0;
        let mut tend: clock_t = 0;
        let mut fp: *mut FILE = std::ptr::null_mut::<FILE>();
        let mut sv: libc::c_int = 0;
        let mut neph: libc::c_int = 0;
        let mut ieph: libc::c_int = 0;
        let mut eph: [[ephem_t; 32]; 15] = [[ephem_t {
            vflg: 0,
            t: datetime_t {
                y: 0,
                m: 0,
                d: 0,
                hh: 0,
                mm: 0,
                sec: 0.,
            },
            toc: gpstime_t { week: 0, sec: 0. },
            toe: gpstime_t { week: 0, sec: 0. },
            iodc: 0,
            iode: 0,
            deltan: 0.,
            cuc: 0.,
            cus: 0.,
            cic: 0.,
            cis: 0.,
            crc: 0.,
            crs: 0.,
            ecc: 0.,
            sqrta: 0.,
            m0: 0.,
            omg0: 0.,
            inc0: 0.,
            aop: 0.,
            omgdot: 0.,
            idot: 0.,
            af0: 0.,
            af1: 0.,
            af2: 0.,
            tgd: 0.,
            svhlth: 0,
            codeL2: 0,
            n: 0.,
            sq1e2: 0.,
            A: 0.,
            omgkdot: 0.,
        }; 32]; 15];
        let mut g0: gpstime_t = gpstime_t { week: 0, sec: 0. };
        let mut llh: [libc::c_double; 3] = [0.; 3];
        let mut i: libc::c_int = 0;
        let mut chan: [channel_t; 16] = [channel_t {
            prn: 0,
            ca: [0; 1023],
            f_carr: 0.,
            f_code: 0.,
            carr_phase: 0,
            carr_phasestep: 0,
            code_phase: 0.,
            g0: gpstime_t { week: 0, sec: 0. },
            sbf: [[0; 10]; 5],
            dwrd: [0; 60],
            iword: 0,
            ibit: 0,
            icode: 0,
            dataBit: 0,
            codeCA: 0,
            azel: [0.; 2],
            rho0: range_t {
                g: gpstime_t { week: 0, sec: 0. },
                range: 0.,
                rate: 0.,
                d: 0.,
                azel: [0.; 2],
                iono_delay: 0.,
            },
        }; 16];
        let mut elvmask: libc::c_double = 0.0f64;
        let mut ip: libc::c_int = 0;
        let mut qp: libc::c_int = 0;
        let mut iTable: libc::c_int = 0;
        let mut iq_buff: *mut libc::c_short = std::ptr::null_mut::<libc::c_short>();
        let mut iq8_buff: *mut libc::c_schar = std::ptr::null_mut::<libc::c_schar>();
        let mut grx: gpstime_t = gpstime_t { week: 0, sec: 0. };
        let mut delt: libc::c_double = 0.;
        let mut isamp: libc::c_int = 0;
        let mut iumd: libc::c_int = 0;
        let mut numd: libc::c_int = 0;
        let mut umfile: [libc::c_char; 100] = [0; 100];
        let mut staticLocationMode: libc::c_int = 0 as libc::c_int;
        let mut nmeaGGA: libc::c_int = 0 as libc::c_int;
        let mut umLLH: libc::c_int = 0 as libc::c_int;
        let mut navfile: [libc::c_char; 100] = [0; 100];
        let mut outfile: [libc::c_char; 100] = [0; 100];
        let mut samp_freq: libc::c_double = 0.;
        let mut iq_buff_size: libc::c_int = 0;
        let mut data_format: libc::c_int = 0;
        let mut result: libc::c_int = 0;
        let mut gain: [libc::c_int; 16] = [0; 16];
        let mut path_loss: libc::c_double = 0.;
        let mut ant_gain: libc::c_double = 0.;
        let mut fixed_gain: libc::c_int = 128 as libc::c_int;
        let mut ant_pat: [libc::c_double; 37] = [0.; 37];
        let mut ibs: libc::c_int = 0;
        let mut t0: datetime_t = datetime_t {
            y: 0,
            m: 0,
            d: 0,
            hh: 0,
            mm: 0,
            sec: 0.,
        };
        let mut tmin: datetime_t = datetime_t {
            y: 0,
            m: 0,
            d: 0,
            hh: 0,
            mm: 0,
            sec: 0.,
        };
        let mut tmax: datetime_t = datetime_t {
            y: 0,
            m: 0,
            d: 0,
            hh: 0,
            mm: 0,
            sec: 0.,
        };
        let mut gmin: gpstime_t = gpstime_t { week: 0, sec: 0. };
        let mut gmax: gpstime_t = gpstime_t { week: 0, sec: 0. };
        let mut dt: libc::c_double = 0.;
        let mut igrx: libc::c_int = 0;
        let mut duration: libc::c_double = 0.;
        let mut iduration: libc::c_int = 0;
        let mut verb: libc::c_int = 0;
        let mut timeoverwrite: libc::c_int = 0 as libc::c_int;
        let mut ionoutc: ionoutc_t = ionoutc_t {
            enable: 0,
            vflg: 0,
            alpha0: 0.,
            alpha1: 0.,
            alpha2: 0.,
            alpha3: 0.,
            beta0: 0.,
            beta1: 0.,
            beta2: 0.,
            beta3: 0.,
            A0: 0.,
            A1: 0.,
            dtls: 0,
            tot: 0,
            wnt: 0,
            dtlsf: 0,
            dn: 0,
            wnlsf: 0,
            leapen: 0,
        };
        let mut path_loss_enable: libc::c_int = 1 as libc::c_int;
        navfile[0 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
        umfile[0 as libc::c_int as usize] = 0 as libc::c_int as libc::c_char;
        strcpy(
            outfile.as_mut_ptr(),
            b"gpssim.bin\0" as *const u8 as *const libc::c_char,
        );
        samp_freq = 2.6e6f64;
        data_format = 16 as libc::c_int;
        g0.week = -(1 as libc::c_int);
        iduration = 3000 as libc::c_int;
        duration = iduration as libc::c_double / 10.0f64;
        verb = 0 as libc::c_int;
        ionoutc.enable = 1 as libc::c_int;
        ionoutc.leapen = 0 as libc::c_int;
        if argc < 3 as libc::c_int {
            usage();
            exit(1 as libc::c_int);
        }
        loop {
            result = getopt(
                argc,
                argv as *const *mut libc::c_char,
                b"e:u:x:g:c:l:o:s:b:L:T:t:d:ipv\0" as *const u8 as *const libc::c_char,
            );
            if result == -(1 as libc::c_int) {
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
                    nmeaGGA = 0 as libc::c_int;
                    umLLH = 0 as libc::c_int;
                    current_block_85 = 2750570471926810434;
                }
                120 => {
                    strcpy(umfile.as_mut_ptr(), optarg);
                    umLLH = 1 as libc::c_int;
                    current_block_85 = 2750570471926810434;
                }
                103 => {
                    strcpy(umfile.as_mut_ptr(), optarg);
                    nmeaGGA = 1 as libc::c_int;
                    current_block_85 = 2750570471926810434;
                }
                99 => {
                    staticLocationMode = 1 as libc::c_int;
                    sscanf(
                        optarg,
                        b"%lf,%lf,%lf\0" as *const u8 as *const libc::c_char,
                        &mut *(*xyz.as_mut_ptr().offset(0 as libc::c_int as isize))
                            .as_mut_ptr()
                            .offset(0 as libc::c_int as isize)
                            as *mut libc::c_double,
                        &mut *(*xyz.as_mut_ptr().offset(0 as libc::c_int as isize))
                            .as_mut_ptr()
                            .offset(1 as libc::c_int as isize)
                            as *mut libc::c_double,
                        &mut *(*xyz.as_mut_ptr().offset(0 as libc::c_int as isize))
                            .as_mut_ptr()
                            .offset(2 as libc::c_int as isize)
                            as *mut libc::c_double,
                    );
                    current_block_85 = 2750570471926810434;
                }
                108 => {
                    staticLocationMode = 1 as libc::c_int;
                    sscanf(
                        optarg,
                        b"%lf,%lf,%lf\0" as *const u8 as *const libc::c_char,
                        &mut *llh.as_mut_ptr().offset(0 as libc::c_int as isize)
                            as *mut libc::c_double,
                        &mut *llh.as_mut_ptr().offset(1 as libc::c_int as isize)
                            as *mut libc::c_double,
                        &mut *llh.as_mut_ptr().offset(2 as libc::c_int as isize)
                            as *mut libc::c_double,
                    );
                    llh[0 as libc::c_int as usize] /= 57.2957795131f64;
                    llh[1 as libc::c_int as usize] /= 57.2957795131f64;
                    llh2xyz(
                        llh.as_mut_ptr(),
                        (xyz[0 as libc::c_int as usize]).as_mut_ptr(),
                    );
                    current_block_85 = 2750570471926810434;
                }
                111 => {
                    strcpy(outfile.as_mut_ptr(), optarg);
                    current_block_85 = 2750570471926810434;
                }
                115 => {
                    samp_freq = atof(optarg);
                    if samp_freq < 1.0e6f64 {
                        eprintln!("ERROR: Invalid sampling frequency.\n");
                        exit(1 as libc::c_int);
                    }
                    current_block_85 = 2750570471926810434;
                }
                98 => {
                    data_format = atoi(optarg);
                    if data_format != 1 as libc::c_int
                        && data_format != 8 as libc::c_int
                        && data_format != 16 as libc::c_int
                    {
                        eprintln!("ERROR: Invalid I/Q data format.\n");
                        exit(1 as libc::c_int);
                    }
                    current_block_85 = 2750570471926810434;
                }
                76 => {
                    ionoutc.leapen = 1 as libc::c_int;
                    sscanf(
                        optarg,
                        b"%d,%d,%d\0" as *const u8 as *const libc::c_char,
                        &mut ionoutc.wnlsf as *mut libc::c_int,
                        &mut ionoutc.dn as *mut libc::c_int,
                        &mut ionoutc.dtlsf as *mut libc::c_int,
                    );
                    if ionoutc.dn < 1 as libc::c_int && ionoutc.dn > 7 as libc::c_int {
                        eprintln!("ERROR: Invalid GPS day number");
                        exit(1 as libc::c_int);
                    }
                    if ionoutc.wnlsf < 0 as libc::c_int {
                        eprintln!("ERROR: Invalid GPS week number");
                        exit(1 as libc::c_int);
                    }
                    if ionoutc.dtlsf < -(128 as libc::c_int) && ionoutc.dtlsf > 127 as libc::c_int {
                        eprintln!("ERROR: Invalid delta leap second");
                        exit(1 as libc::c_int);
                    }
                    current_block_85 = 2750570471926810434;
                }
                84 => {
                    timeoverwrite = 1 as libc::c_int;
                    if strncmp(
                        optarg,
                        b"now\0" as *const u8 as *const libc::c_char,
                        3 as libc::c_int as libc::c_ulong,
                    ) == 0 as libc::c_int
                    {
                        let mut timer: time_t = 0;
                        let mut gmt: *mut tm = std::ptr::null_mut::<tm>();
                        time(&mut timer);
                        gmt = gmtime(&mut timer);
                        t0.y = (*gmt).tm_year + 1900 as libc::c_int;
                        t0.m = (*gmt).tm_mon + 1 as libc::c_int;
                        t0.d = (*gmt).tm_mday;
                        t0.hh = (*gmt).tm_hour;
                        t0.mm = (*gmt).tm_min;
                        t0.sec = (*gmt).tm_sec as libc::c_double;
                        date2gps(&mut t0, &mut g0);
                        current_block_85 = 2750570471926810434;
                    } else {
                        current_block_85 = 4676144417340510455;
                    }
                }
                116 => {
                    current_block_85 = 4676144417340510455;
                }
                100 => {
                    duration = atof(optarg);
                    current_block_85 = 2750570471926810434;
                }
                105 => {
                    ionoutc.enable = 0 as libc::c_int;
                    current_block_85 = 2750570471926810434;
                }
                112 => {
                    if optind < argc
                        && *(*argv.offset(optind as isize)).offset(0 as libc::c_int as isize)
                            as libc::c_int
                            != '-' as i32
                    {
                        fixed_gain = atoi(*argv.offset(optind as isize));
                        if fixed_gain < 1 as libc::c_int || fixed_gain > 128 as libc::c_int {
                            eprintln!("ERROR: Fixed gain must be between 1 and 128.\n");
                            exit(1 as libc::c_int);
                        }
                        optind += 1;
                    }
                    path_loss_enable = 0 as libc::c_int;
                    current_block_85 = 2750570471926810434;
                }
                118 => {
                    verb = 1 as libc::c_int;
                    current_block_85 = 2750570471926810434;
                }
                58 | 63 => {
                    usage();
                    exit(1 as libc::c_int);
                }
                _ => {
                    current_block_85 = 2750570471926810434;
                }
            }
            if current_block_85 == 4676144417340510455 {
                sscanf(
                    optarg,
                    b"%d/%d/%d,%d:%d:%lf\0" as *const u8 as *const libc::c_char,
                    &mut t0.y as *mut libc::c_int,
                    &mut t0.m as *mut libc::c_int,
                    &mut t0.d as *mut libc::c_int,
                    &mut t0.hh as *mut libc::c_int,
                    &mut t0.mm as *mut libc::c_int,
                    &mut t0.sec as *mut libc::c_double,
                );
                if t0.y <= 1980 as libc::c_int
                    || t0.m < 1 as libc::c_int
                    || t0.m > 12 as libc::c_int
                    || t0.d < 1 as libc::c_int
                    || t0.d > 31 as libc::c_int
                    || t0.hh < 0 as libc::c_int
                    || t0.hh > 23 as libc::c_int
                    || t0.mm < 0 as libc::c_int
                    || t0.mm > 59 as libc::c_int
                    || t0.sec < 0.0f64
                    || t0.sec >= 60.0f64
                {
                    eprintln!("ERROR: Invalid date and time.\n");
                    exit(1 as libc::c_int);
                }
                t0.sec = floor(t0.sec);
                date2gps(&mut t0, &mut g0);
            }
        }
        if navfile[0 as libc::c_int as usize] as libc::c_int == 0 as libc::c_int {
            eprintln!("ERROR: GPS ephemeris file is not specified.\n");
            exit(1 as libc::c_int);
        }
        if umfile[0 as libc::c_int as usize] as libc::c_int == 0 as libc::c_int
            && staticLocationMode == 0
        {
            staticLocationMode = 1 as libc::c_int;
            llh[0 as libc::c_int as usize] = 35.681298f64 / 57.2957795131f64;
            llh[1 as libc::c_int as usize] = 139.766247f64 / 57.2957795131f64;
            llh[2 as libc::c_int as usize] = 10.0f64;
        }
        if duration < 0.0f64
            || duration > 3000 as libc::c_int as libc::c_double / 10.0f64 && staticLocationMode == 0
            || duration > 86400 as libc::c_int as libc::c_double && staticLocationMode != 0
        {
            eprintln!("ERROR: Invalid duration.\n\0");
            exit(1 as libc::c_int);
        }
        iduration = (duration * 10.0f64 + 0.5f64) as libc::c_int;
        samp_freq = floor(samp_freq / 10.0f64);
        iq_buff_size = samp_freq as libc::c_int;
        samp_freq *= 10.0f64;
        delt = 1.0f64 / samp_freq;
        if staticLocationMode == 0 {
            if nmeaGGA == 1 as libc::c_int {
                numd = readNmeaGGA(xyz.as_mut_ptr(), umfile.as_mut_ptr());
            } else if umLLH == 1 as libc::c_int {
                numd = readUserMotionLLH(xyz.as_mut_ptr(), umfile.as_mut_ptr());
            } else {
                numd = readUserMotion(xyz.as_mut_ptr(), umfile.as_mut_ptr());
            }
            if numd == -(1 as libc::c_int) {
                eprintln!("ERROR: Failed to open user motion / NMEA GGA file.\n\0");
                exit(1 as libc::c_int);
            } else if numd == 0 as libc::c_int {
                eprintln!("ERROR: Failed to read user motion / NMEA GGA data.\n\0");
                exit(1 as libc::c_int);
            }
            if numd > iduration {
                numd = iduration;
            }
            xyz2llh(
                (xyz[0 as libc::c_int as usize]).as_mut_ptr(),
                llh.as_mut_ptr(),
            );
        } else {
            eprintln!("Using static location mode.\n\0");
            numd = iduration;
            llh2xyz(
                llh.as_mut_ptr(),
                (xyz[0 as libc::c_int as usize]).as_mut_ptr(),
            );
        }

        eprintln!(
            "xyz = {}, {}, {}\n\0",
            xyz[0 as libc::c_int as usize][0 as libc::c_int as usize],
            xyz[0 as libc::c_int as usize][1 as libc::c_int as usize],
            xyz[0 as libc::c_int as usize][2 as libc::c_int as usize],
        );

        eprintln!(
            "llh = {}, {}, {}\n\0",
            llh[0 as libc::c_int as usize] * 57.2957795131f64,
            llh[1 as libc::c_int as usize] * 57.2957795131f64,
            llh[2 as libc::c_int as usize],
        );
        neph = readRinexNavAll(eph.as_mut_ptr(), &mut ionoutc, navfile.as_mut_ptr());
        if neph == 0 as libc::c_int {
            eprintln!("ERROR: No ephemeris available.\n\0",);
            exit(1 as libc::c_int);
        } else if neph == -(1 as libc::c_int) {
            eprintln!("ERROR: ephemeris file not found.\n\0");
            exit(1 as libc::c_int);
        }
        if verb == 1 as libc::c_int && ionoutc.vflg == 1 as libc::c_int {
            eprintln!(
                "  {:12.3e} {:12.3e} {:12.3e} {:12.3e}\n\0",
                ionoutc.alpha0, ionoutc.alpha1, ionoutc.alpha2, ionoutc.alpha3,
            );

            eprintln!(
                "  {:12.3e} {:12.3e} {:12.3e} {:12.3e}\n\0",
                ionoutc.beta0, ionoutc.beta1, ionoutc.beta2, ionoutc.beta3,
            );

            eprintln!(
                "   {:19.11e} {:19.11e}  {:9} {:9}\n\0",
                ionoutc.A0, ionoutc.A1, ionoutc.tot, ionoutc.wnt,
            );

            eprintln!("{:6}\n\0", ionoutc.dtls,);
        }
        sv = 0 as libc::c_int;
        while sv < 32 as libc::c_int {
            if eph[0 as libc::c_int as usize][sv as usize].vflg == 1 as libc::c_int {
                gmin = eph[0 as libc::c_int as usize][sv as usize].toc;
                tmin = eph[0 as libc::c_int as usize][sv as usize].t;
                break;
            } else {
                sv += 1;
            }
        }
        gmax.sec = 0 as libc::c_int as libc::c_double;
        gmax.week = 0 as libc::c_int;
        tmax.sec = 0 as libc::c_int as libc::c_double;
        tmax.mm = 0 as libc::c_int;
        tmax.hh = 0 as libc::c_int;
        tmax.d = 0 as libc::c_int;
        tmax.m = 0 as libc::c_int;
        tmax.y = 0 as libc::c_int;
        sv = 0 as libc::c_int;
        while sv < 32 as libc::c_int {
            if eph[(neph - 1 as libc::c_int) as usize][sv as usize].vflg == 1 as libc::c_int {
                gmax = eph[(neph - 1 as libc::c_int) as usize][sv as usize].toc;
                tmax = eph[(neph - 1 as libc::c_int) as usize][sv as usize].t;
                break;
            } else {
                sv += 1;
            }
        }
        if g0.week >= 0 as libc::c_int {
            if timeoverwrite == 1 as libc::c_int {
                let mut gtmp: gpstime_t = gpstime_t { week: 0, sec: 0. };
                let mut ttmp: datetime_t = datetime_t {
                    y: 0,
                    m: 0,
                    d: 0,
                    hh: 0,
                    mm: 0,
                    sec: 0.,
                };
                let mut dsec: libc::c_double = 0.;
                gtmp.week = g0.week;
                gtmp.sec =
                    (g0.sec as libc::c_int / 7200 as libc::c_int) as libc::c_double * 7200.0f64;
                dsec = subGpsTime(gtmp, gmin);
                ionoutc.wnt = gtmp.week;
                ionoutc.tot = gtmp.sec as libc::c_int;
                sv = 0 as libc::c_int;
                while sv < 32 as libc::c_int {
                    i = 0 as libc::c_int;
                    while i < neph {
                        if eph[i as usize][sv as usize].vflg == 1 as libc::c_int {
                            gtmp = incGpsTime(eph[i as usize][sv as usize].toc, dsec);
                            gps2date(&mut gtmp, &mut ttmp);
                            eph[i as usize][sv as usize].toc = gtmp;
                            eph[i as usize][sv as usize].t = ttmp;
                            gtmp = incGpsTime(eph[i as usize][sv as usize].toe, dsec);
                            eph[i as usize][sv as usize].toe = gtmp;
                        }
                        i += 1;
                    }
                    sv += 1;
                }
            } else if subGpsTime(g0, gmin) < 0.0f64 || subGpsTime(gmax, g0) < 0.0f64 {
                eprintln!("ERROR: Invalid start time.\n\0");
                eprintln!(
                    "tmin = {:4}/{:02}/{:02},{:02}:{:02}:{:0>2.0} ({}:{:.0})\n\0",
                    tmin.y, tmin.m, tmin.d, tmin.hh, tmin.mm, tmin.sec, gmin.week, gmin.sec,
                );
                eprintln!(
                    "tmax = {:4}/{:02}/{:02},{:02}:{:02}:{:0>2.0} ({}:{:.0})\n\0",
                    tmax.y, tmax.m, tmax.d, tmax.hh, tmax.mm, tmax.sec, gmax.week, gmax.sec,
                );
                exit(1 as libc::c_int);
            }
        } else {
            g0 = gmin;
            t0 = tmin;
        }

        eprintln!(
            "Start time = {:4}/{:02}/{:02},{:02}:{:02}:{:0>2.0} ({}:{:.0})\n\0",
            t0.y, t0.m, t0.d, t0.hh, t0.mm, t0.sec, g0.week, g0.sec,
        );

        eprintln!(
            "Duration = {:.1} [sec]\n\0",
            numd as libc::c_double / 10.0f64
        );
        ieph = -(1 as libc::c_int);
        i = 0 as libc::c_int;
        while i < neph {
            sv = 0 as libc::c_int;
            while sv < 32 as libc::c_int {
                if eph[i as usize][sv as usize].vflg == 1 as libc::c_int {
                    dt = subGpsTime(g0, eph[i as usize][sv as usize].toc);
                    if (-3600.0f64..3600.0f64).contains(&dt) {
                        ieph = i;
                        break;
                    }
                }
                sv += 1;
            }
            if ieph >= 0 as libc::c_int {
                break;
            }
            i += 1;
        }
        if ieph == -(1 as libc::c_int) {
            eprintln!("ERROR: No current set of ephemerides has been found.\n\0",);
            exit(1 as libc::c_int);
        }
        iq_buff = calloc(
            (2 as libc::c_int * iq_buff_size) as libc::c_ulong,
            2 as libc::c_int as libc::c_ulong,
        ) as *mut libc::c_short;
        if iq_buff.is_null() {
            eprintln!("ERROR: Failed to allocate 16-bit I/Q buffer.\n\0");
            exit(1 as libc::c_int);
        }
        if data_format == 8 as libc::c_int {
            iq8_buff = calloc(
                (2 as libc::c_int * iq_buff_size) as libc::c_ulong,
                1 as libc::c_int as libc::c_ulong,
            ) as *mut libc::c_schar;
            if iq8_buff.is_null() {
                eprintln!("ERROR: Failed to allocate 8-bit I/Q buffer.\n\0");
                exit(1 as libc::c_int);
            }
        } else if data_format == 1 as libc::c_int {
            iq8_buff = calloc(
                (iq_buff_size / 4 as libc::c_int) as libc::c_ulong,
                1 as libc::c_int as libc::c_ulong,
            ) as *mut libc::c_schar;
            if iq8_buff.is_null() {
                eprintln!("ERROR: Failed to allocate compressed 1-bit I/Q buffer.\n\0");
                exit(1 as libc::c_int);
            }
        }
        if strcmp(
            b"-\0" as *const u8 as *const libc::c_char,
            outfile.as_mut_ptr(),
        ) != 0
        {
            fp = fopen(
                outfile.as_mut_ptr(),
                b"wb\0" as *const u8 as *const libc::c_char,
            );
            if fp.is_null() {
                eprintln!("ERROR: Failed to open output file.\n\0");
                exit(1 as libc::c_int);
            }
        } else {
            fp = stdout;
        }
        i = 0 as libc::c_int;
        while i < 16 as libc::c_int {
            chan[i as usize].prn = 0 as libc::c_int;
            i += 1;
        }
        sv = 0 as libc::c_int;
        while sv < 32 as libc::c_int {
            allocatedSat[sv as usize] = -(1 as libc::c_int);
            sv += 1;
        }
        grx = incGpsTime(g0, 0.0f64);
        allocateChannel(
            chan.as_mut_ptr(),
            (eph[ieph as usize]).as_mut_ptr(),
            ionoutc,
            grx,
            (xyz[0 as libc::c_int as usize]).as_mut_ptr(),
            elvmask,
        );
        i = 0 as libc::c_int;
        while i < 16 as libc::c_int {
            if chan[i as usize].prn > 0 as libc::c_int {
                eprintln!(
                    "{:02} {:6.1} {:5.1} {:11.1} {:5.1}\n\0",
                    chan[i as usize].prn,
                    chan[i as usize].azel[0 as libc::c_int as usize] * 57.2957795131f64,
                    chan[i as usize].azel[1 as libc::c_int as usize] * 57.2957795131f64,
                    chan[i as usize].rho0.d,
                    chan[i as usize].rho0.iono_delay,
                );
            }
            i += 1;
        }
        i = 0 as libc::c_int;
        while i < 37 as libc::c_int {
            ant_pat[i as usize] = pow(10.0f64, -ant_pat_db[i as usize] / 20.0f64);
            i += 1;
        }
        tstart = clock();
        grx = incGpsTime(grx, 0.1f64);
        iumd = 1 as libc::c_int;
        while iumd < numd {
            i = 0 as libc::c_int;
            while i < 16 as libc::c_int {
                if chan[i as usize].prn > 0 as libc::c_int {
                    let mut rho: range_t = range_t {
                        g: gpstime_t { week: 0, sec: 0. },
                        range: 0.,
                        rate: 0.,
                        d: 0.,
                        azel: [0.; 2],
                        iono_delay: 0.,
                    };
                    sv = chan[i as usize].prn - 1 as libc::c_int;
                    if staticLocationMode == 0 {
                        computeRange(
                            &mut rho,
                            eph[ieph as usize][sv as usize],
                            &mut ionoutc,
                            grx,
                            (xyz[iumd as usize]).as_mut_ptr(),
                        );
                    } else {
                        computeRange(
                            &mut rho,
                            eph[ieph as usize][sv as usize],
                            &mut ionoutc,
                            grx,
                            (xyz[0 as libc::c_int as usize]).as_mut_ptr(),
                        );
                    }
                    chan[i as usize].azel[0 as libc::c_int as usize] =
                        rho.azel[0 as libc::c_int as usize];
                    chan[i as usize].azel[1 as libc::c_int as usize] =
                        rho.azel[1 as libc::c_int as usize];
                    computeCodePhase(&mut *chan.as_mut_ptr().offset(i as isize), rho, 0.1f64);
                    chan[i as usize].carr_phasestep =
                        round(512.0f64 * 65536.0f64 * chan[i as usize].f_carr * delt)
                            as libc::c_int;
                    path_loss = 20200000.0f64 / rho.d;
                    ibs = ((90.0f64 - rho.azel[1 as libc::c_int as usize] * 57.2957795131f64)
                        / 5.0f64) as libc::c_int;
                    ant_gain = ant_pat[ibs as usize];
                    if path_loss_enable == 1 as libc::c_int {
                        gain[i as usize] = (path_loss * ant_gain * 128.0f64) as libc::c_int;
                    } else {
                        gain[i as usize] = fixed_gain;
                    }
                }
                i += 1;
            }
            isamp = 0 as libc::c_int;
            while isamp < iq_buff_size {
                let mut i_acc: libc::c_int = 0 as libc::c_int;
                let mut q_acc: libc::c_int = 0 as libc::c_int;
                i = 0 as libc::c_int;
                while i < 16 as libc::c_int {
                    if chan[i as usize].prn > 0 as libc::c_int {
                        iTable = (chan[i as usize].carr_phase >> 16 as libc::c_int
                            & 0x1ff as libc::c_int as libc::c_uint)
                            as libc::c_int;
                        ip = chan[i as usize].dataBit
                            * chan[i as usize].codeCA
                            * cosTable512[iTable as usize]
                            * gain[i as usize];
                        qp = chan[i as usize].dataBit
                            * chan[i as usize].codeCA
                            * sinTable512[iTable as usize]
                            * gain[i as usize];
                        i_acc += ip;
                        q_acc += qp;
                        chan[i as usize].code_phase += chan[i as usize].f_code * delt;
                        if chan[i as usize].code_phase >= 1023 as libc::c_int as libc::c_double {
                            chan[i as usize].code_phase -= 1023 as libc::c_int as libc::c_double;
                            chan[i as usize].icode += 1;
                            chan[i as usize].icode;
                            if chan[i as usize].icode >= 20 as libc::c_int {
                                chan[i as usize].icode = 0 as libc::c_int;
                                chan[i as usize].ibit += 1;
                                chan[i as usize].ibit;
                                if chan[i as usize].ibit >= 30 as libc::c_int {
                                    chan[i as usize].ibit = 0 as libc::c_int;
                                    chan[i as usize].iword += 1;
                                    chan[i as usize].iword;
                                }
                                chan[i as usize].dataBit = (chan[i as usize].dwrd
                                    [chan[i as usize].iword as usize]
                                    >> (29 as libc::c_int - chan[i as usize].ibit)
                                    & 0x1 as libc::c_ulong)
                                    as libc::c_int
                                    * 2 as libc::c_int
                                    - 1 as libc::c_int;
                            }
                        }
                        chan[i as usize].codeCA = chan[i as usize].ca
                            [chan[i as usize].code_phase as libc::c_int as usize]
                            * 2 as libc::c_int
                            - 1 as libc::c_int;
                        chan[i as usize].carr_phase = (chan[i as usize].carr_phase)
                            .wrapping_add(chan[i as usize].carr_phasestep as libc::c_uint);
                    }
                    i += 1;
                }
                i_acc = (i_acc + 64 as libc::c_int) >> 7 as libc::c_int;
                q_acc = (q_acc + 64 as libc::c_int) >> 7 as libc::c_int;
                *iq_buff.offset((isamp * 2 as libc::c_int) as isize) = i_acc as libc::c_short;
                *iq_buff.offset((isamp * 2 as libc::c_int + 1 as libc::c_int) as isize) =
                    q_acc as libc::c_short;
                isamp += 1;
            }
            if data_format == 1 as libc::c_int {
                isamp = 0 as libc::c_int;
                while isamp < 2 as libc::c_int * iq_buff_size {
                    if isamp % 8 as libc::c_int == 0 as libc::c_int {
                        *iq8_buff.offset((isamp / 8 as libc::c_int) as isize) =
                            0 as libc::c_int as libc::c_schar;
                    }
                    let fresh1 = &mut (*iq8_buff.offset((isamp / 8 as libc::c_int) as isize));
                    *fresh1 = (*fresh1 as libc::c_int
                        | (if *iq_buff.offset(isamp as isize) as libc::c_int > 0 as libc::c_int {
                            0x1 as libc::c_int
                        } else {
                            0 as libc::c_int
                        }) << (7 as libc::c_int - isamp % 8 as libc::c_int))
                        as libc::c_schar;
                    isamp += 1;
                }
                fwrite(
                    iq8_buff as *const libc::c_void,
                    1 as libc::c_int as libc::c_ulong,
                    (iq_buff_size / 4 as libc::c_int) as libc::c_ulong,
                    fp,
                );
            } else if data_format == 8 as libc::c_int {
                isamp = 0 as libc::c_int;
                while isamp < 2 as libc::c_int * iq_buff_size {
                    *iq8_buff.offset(isamp as isize) =
                        (*iq_buff.offset(isamp as isize) as libc::c_int >> 4 as libc::c_int)
                            as libc::c_schar;
                    isamp += 1;
                }
                fwrite(
                    iq8_buff as *const libc::c_void,
                    1 as libc::c_int as libc::c_ulong,
                    (2 as libc::c_int * iq_buff_size) as libc::c_ulong,
                    fp,
                );
            } else {
                fwrite(
                    iq_buff as *const libc::c_void,
                    2 as libc::c_int as libc::c_ulong,
                    (2 as libc::c_int * iq_buff_size) as libc::c_ulong,
                    fp,
                );
            }
            igrx = (grx.sec * 10.0f64 + 0.5f64) as libc::c_int;
            if igrx % 300 as libc::c_int == 0 as libc::c_int {
                i = 0 as libc::c_int;
                while i < 16 as libc::c_int {
                    if chan[i as usize].prn > 0 as libc::c_int {
                        generateNavMsg(
                            grx,
                            &mut *chan.as_mut_ptr().offset(i as isize),
                            0 as libc::c_int,
                        );
                    }
                    i += 1;
                }
                sv = 0 as libc::c_int;
                while sv < 32 as libc::c_int {
                    if eph[(ieph + 1 as libc::c_int) as usize][sv as usize].vflg == 1 as libc::c_int
                    {
                        dt = subGpsTime(
                            eph[(ieph + 1 as libc::c_int) as usize][sv as usize].toc,
                            grx,
                        );
                        if dt < 3600.0f64 {
                            ieph += 1;
                            i = 0 as libc::c_int;
                            while i < 16 as libc::c_int {
                                if chan[i as usize].prn != 0 as libc::c_int {
                                    eph2sbf(
                                        eph[ieph as usize]
                                            [(chan[i as usize].prn - 1 as libc::c_int) as usize],
                                        ionoutc,
                                        (chan[i as usize].sbf).as_mut_ptr(),
                                    );
                                }
                                i += 1;
                            }
                        }
                        break;
                    } else {
                        sv += 1;
                    }
                }
                if staticLocationMode == 0 {
                    allocateChannel(
                        chan.as_mut_ptr(),
                        (eph[ieph as usize]).as_mut_ptr(),
                        ionoutc,
                        grx,
                        (xyz[iumd as usize]).as_mut_ptr(),
                        elvmask,
                    );
                } else {
                    allocateChannel(
                        chan.as_mut_ptr(),
                        (eph[ieph as usize]).as_mut_ptr(),
                        ionoutc,
                        grx,
                        (xyz[0 as libc::c_int as usize]).as_mut_ptr(),
                        elvmask,
                    );
                }
                if verb == 1 as libc::c_int {
                    eprintln!();
                    i = 0 as libc::c_int;
                    while i < 16 as libc::c_int {
                        if chan[i as usize].prn > 0 as libc::c_int {
                            eprintln!(
                                "{:02} {:6.1} {:5.1} {:11.1} {:5.1}\n\0",
                                chan[i as usize].prn,
                                chan[i as usize].azel[0 as libc::c_int as usize] * 57.2957795131f64,
                                chan[i as usize].azel[1 as libc::c_int as usize] * 57.2957795131f64,
                                chan[i as usize].rho0.d,
                                chan[i as usize].rho0.iono_delay,
                            );
                        }
                        i += 1;
                    }
                }
            }
            grx = incGpsTime(grx, 0.1f64);

            eprintln!("\rTime into run = {:4.1}\0", subGpsTime(grx, g0),);
            fflush(stdout);
            iumd += 1;
        }
        tend = clock();

        eprintln!("\nDone!\n\0");
        free(iq_buff as *mut libc::c_void);
        fclose(fp);

        eprintln!(
            "Process time = {:.1} [sec]\n\0",
            (tend - tstart) as libc::c_double
                / 1000000 as libc::c_int as __clock_t as libc::c_double,
        );
        0 as libc::c_int
    }
}
pub fn main() {
    let mut args: Vec<*mut libc::c_char> = Vec::new();
    for arg in ::std::env::args() {
        args.push(
            (::std::ffi::CString::new(arg))
                .expect("Failed to convert argument into CString.")
                .into_raw(),
        );
    }
    args.push(::core::ptr::null_mut());
    unsafe {
        ::std::process::exit(main_0(
            (args.len() - 1) as libc::c_int,
            args.as_mut_ptr() as *mut *mut libc::c_char,
        ) as i32)
    }
}
