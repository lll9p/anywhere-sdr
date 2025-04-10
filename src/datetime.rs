#[derive(Copy, Clone)]
#[repr(C)]
pub struct tm {
    pub tm_sec: i32,
    pub tm_min: i32,
    pub tm_hour: i32,
    pub tm_mday: i32,
    pub tm_mon: i32,
    pub tm_year: i32,
    pub tm_wday: i32,
    pub tm_yday: i32,
    pub tm_isdst: i32,
    pub tm_gmtoff: libc::c_long,
    pub tm_zone: *const libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct gpstime_t {
    pub week: i32,
    pub sec: f64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct datetime_t {
    pub y: i32,
    pub m: i32,
    pub d: i32,
    pub hh: i32,
    pub mm: i32,
    pub sec: f64,
}
