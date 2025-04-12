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
// Structure representing GPS time
#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct gpstime_t {
    // GPS week number (since January 1980)
    pub week: i32,
    // second inside the GPS \a week
    pub sec: f64,
}
//  Structure repreenting UTC time
#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct datetime_t {
    // < Calendar year
    pub y: i32,
    // < Calendar month
    pub m: i32,
    // < Calendar day
    pub d: i32,
    // < Calendar hour
    pub hh: i32,
    // < Calendar minutes
    pub mm: i32,
    // < Calendar seconds
    pub sec: f64,
}
