use constants::{
    SECONDS_IN_DAY, SECONDS_IN_HOUR, SECONDS_IN_MINUTE, SECONDS_IN_WEEK,
};
use geometry::Azel;
// Structure representing GPS time
#[derive(Debug, Clone, Default)]
// #[repr(C)]
pub struct GpsTime {
    // GPS week number (since January 1980)
    pub week: i32,
    // second inside the GPS \a week
    pub sec: f64,
}

impl GpsTime {
    pub fn diff_secs(&self, other: &Self) -> f64 {
        let mut dt = self.sec - other.sec;
        dt += f64::from(self.week - other.week) * SECONDS_IN_WEEK;
        dt
    }

    pub fn add_secs(&self, dt: f64) -> Self {
        let mut new_time: GpsTime = GpsTime { week: 0, sec: 0. };
        new_time.week = self.week;
        new_time.sec = self.sec + dt;
        new_time.sec = (new_time.sec * 1000.0).round() / 1000.0; // Avoid rounding error
        while new_time.sec >= SECONDS_IN_WEEK {
            new_time.sec -= SECONDS_IN_WEEK;
            new_time.week += 1;
        }
        while new_time.sec < 0.0 {
            new_time.sec += SECONDS_IN_WEEK;
            new_time.week -= 1;
        }
        new_time
    }
}
impl From<&DateTime> for GpsTime {
    ///  Convert a UTC date into a GPS date
    fn from(time: &DateTime) -> Self {
        const DOY: [i32; 12] =
            [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
        let ye = time.y - 1980;

        // Compute the number of leap days since Jan 5/Jan 6, 1980.
        let mut lpdays = ye / 4 + 1;
        if ye % 4 == 0 && time.m <= 2 {
            lpdays -= 1;
        }

        // Compute the number of days elapsed since Jan 5/Jan 6, 1980.
        let de = ye * 365 + DOY[(time.m - 1) as usize] + time.d + lpdays - 6;

        // Convert time to GPS weeks and seconds.
        let week = de / 7;
        let sec = f64::from(de % 7) * SECONDS_IN_DAY
            + f64::from(time.hh) * SECONDS_IN_HOUR
            + f64::from(time.mm) * SECONDS_IN_MINUTE
            + time.sec;
        Self { week, sec }
    }
}

//  Structure repreenting UTC time
#[derive(Debug, Clone, Default)]
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

impl From<&GpsTime> for DateTime {
    /// Convert Julian day number to calendar date
    fn from(time: &GpsTime) -> Self {
        let c = (f64::from(7 * time.week)
            + (time.sec / 86400.0).floor()
            + 2_444_245.0) as i32
            + 1537;
        let d = ((f64::from(c) - 122.1) / 365.25) as i32;
        let e = 365 * d + d / 4;
        let f = (f64::from(c - e) / 30.6001) as i32;
        let d = c - e - (30.6001 * f64::from(f)) as i32;
        let m = f - 1 - 12 * (f / 14);
        let y = d - 4715 - (7 + m) / 10;
        let hh = (time.sec / 3600.0) as i32 % 24;
        let mm = (time.sec / 60.0) as i32 % 60;
        let sec = time.sec - 60.0 * (time.sec / 60.0).floor();
        Self {
            y,
            m,
            d,
            hh,
            mm,
            sec,
        }
    }
}
impl From<jiff::Zoned> for DateTime {
    fn from(value: jiff::Zoned) -> Self {
        Self {
            y: i32::from(value.year()),
            m: i32::from(value.month()),
            d: i32::from(value.day()),
            hh: i32::from(value.hour()),
            mm: i32::from(value.minute()),
            sec: f64::from(value.second()),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TimeRange {
    pub time: GpsTime,
    // pseudorange
    pub range: f64,
    pub rate: f64,
    // geometric distance
    pub distance: f64,
    pub azel: Azel,
    pub iono_delay: f64,
}
