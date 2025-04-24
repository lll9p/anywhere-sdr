use constants::{
    SECONDS_IN_DAY, SECONDS_IN_HOUR, SECONDS_IN_MINUTE, SECONDS_IN_WEEK,
};
use geometry::Azel;

/// Represents time in the GPS time system.
///
/// GPS time is a continuous time scale that started at 00:00:00 UTC on January
/// 6, 1980. It is not adjusted for leap seconds, unlike UTC. GPS time is
/// expressed as a week number and seconds within the week.
///
/// The GPS week number rolls over every 1024 weeks (approximately 19.7 years),
/// with the first rollover occurring on August 21, 1999, and the second on
/// April 6, 2019.
#[derive(Clone, Default)]
pub struct GpsTime {
    /// GPS week number (since January 6, 1980)
    pub week: i32,

    /// Seconds within the GPS week (0.0 to 604799.999...)
    pub sec: f64,
}

impl GpsTime {
    /// Calculates the time difference in seconds between two GPS times.
    ///
    /// This method computes the total time difference by accounting for both
    /// the week number difference and the seconds difference.
    ///
    /// # Arguments
    /// * `other` - The GPS time to subtract from this time
    ///
    /// # Returns
    /// The time difference in seconds (positive if self is later than other)
    pub fn diff_secs(&self, other: &Self) -> f64 {
        let mut dt = self.sec - other.sec;
        dt += f64::from(self.week - other.week) * SECONDS_IN_WEEK;
        dt
    }

    /// Adds a specified number of seconds to this GPS time.
    ///
    /// This method creates a new GPS time that is the specified number of
    /// seconds later than the current time. It handles week rollovers
    /// automatically.
    ///
    /// # Arguments
    /// * `dt` - The number of seconds to add (can be negative)
    ///
    /// # Returns
    /// A new GPS time that is `dt` seconds later than this time
    pub fn add_secs(&self, dt: f64) -> Self {
        let mut new_time: GpsTime = GpsTime { week: 0, sec: 0. };
        new_time.week = self.week;
        new_time.sec = self.sec + dt;
        new_time.sec = (new_time.sec * 1000.0).round() / 1000.0; // Avoid rounding error

        // Handle week rollovers
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
    /// Converts a UTC date and time to GPS time.
    ///
    /// This implementation converts a calendar date in UTC to the corresponding
    /// GPS time (week number and seconds). It accounts for leap years and the
    /// offset between UTC and GPS time origins.
    ///
    /// # Algorithm
    /// 1. Calculate days since GPS epoch (January 6, 1980)
    /// 2. Account for leap years
    /// 3. Convert to weeks and seconds
    ///
    /// # Arguments
    /// * `time` - UTC date and time to convert
    ///
    /// # Returns
    /// The equivalent GPS time
    fn from(time: &DateTime) -> Self {
        // Day of year for the 1st of each month (0-based)
        const DOY: [i32; 12] =
            [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];

        // Years since 1980
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

/// Represents a date and time in the Gregorian calendar (UTC).
///
/// This structure stores a calendar date and time with components for year,
/// month, day, hour, minute, and second. It is used for representing UTC
/// (Coordinated Universal Time) dates in the simulation.
#[derive(Clone, Default)]
pub struct DateTime {
    /// Calendar year (e.g., 2023)
    pub y: i32,

    /// Calendar month (1-12)
    pub m: i32,

    /// Calendar day (1-31)
    pub d: i32,

    /// Hour (0-23)
    pub hh: i32,

    /// Minute (0-59)
    pub mm: i32,

    /// Second and fraction of second (0.0-59.999...)
    pub sec: f64,
}

impl From<&GpsTime> for DateTime {
    /// Converts GPS time to UTC date and time.
    ///
    /// This implementation converts GPS time (week number and seconds) to a
    /// calendar date in UTC. It uses an algorithm based on Julian day numbers
    /// to perform the conversion.
    ///
    /// # Algorithm
    /// 1. Convert GPS time to Julian day number
    /// 2. Apply the Julian day to Gregorian date conversion algorithm
    /// 3. Extract hours, minutes, and seconds from the seconds-of-week
    ///
    /// # Arguments
    /// * `time` - GPS time to convert
    ///
    /// # Returns
    /// The equivalent UTC date and time
    fn from(time: &GpsTime) -> Self {
        // Convert GPS time to Julian day number
        let c = (f64::from(7 * time.week)
            + (time.sec / 86400.0).floor()
            + 2_444_245.0) as i32
            + 1537;

        // Convert Julian day number to calendar date using the algorithm
        let d = ((f64::from(c) - 122.1) / 365.25) as i32;
        let e = 365 * d + d / 4;
        let f = (f64::from(c - e) / 30.6001) as i32;
        let d = c - e - (30.6001 * f64::from(f)) as i32;
        let m = f - 1 - 12 * (f / 14);
        let y = d - 4715 - (7 + m) / 10;

        // Extract time components from seconds-of-week
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
    /// Converts a `jiff::Zoned` timestamp to `DateTime`.
    ///
    /// This implementation allows for easy conversion from the jiff crate's
    /// timestamp type to our internal `DateTime` representation.
    ///
    /// # Arguments
    /// * `value` - A `jiff::Zoned` timestamp to convert
    ///
    /// # Returns
    /// The equivalent `DateTime`
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

/// Represents a satellite range measurement at a specific time.
///
/// This structure combines GPS time, pseudorange, range rate, geometric
/// distance, azimuth/elevation, and ionospheric delay for a satellite
/// measurement. It is used for tracking satellite positions and calculating
/// signal parameters.
#[derive(Clone, Default)]
pub struct TimeRange {
    /// GPS time of the measurement
    pub time: GpsTime,

    /// Pseudorange measurement in meters (includes signal delays)
    pub range: f64,

    /// Range rate (Doppler) in meters per second
    pub rate: f64,

    /// Geometric distance to satellite in meters (true distance without
    /// delays)
    pub distance: f64,

    /// Azimuth and elevation angles to the satellite
    pub azel: Azel,

    /// Ionospheric delay in meters
    pub iono_delay: f64,
}
