// Structure representing GPS time
#[derive(Copy, Clone, Default)]
// #[repr(C)]
pub struct GpsTime {
    // GPS week number (since January 1980)
    pub week: i32,
    // second inside the GPS \a week
    pub sec: f64,
}
//  Structure repreenting UTC time
#[derive(Copy, Clone, Default)]
// #[repr(C)]
pub struct DateTime {
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
