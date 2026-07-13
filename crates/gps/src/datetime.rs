use constants::SECONDS_IN_WEEK;
use geometry::Azel;

/// Gregorian conversion and explicit time-scale types.
mod calendar;
/// Maintained IERS leap-second history.
mod leap_seconds;
pub use calendar::{GpsCalendarDateTime, UtcDateTime};
pub use leap_seconds::{GPS_UTC_LEAP_SECONDS, LeapSecond};

/// Represents continuous time in the GPS time system.
///
/// GPS time started at 00:00:00 UTC on January 6, 1980 and is not adjusted for
/// leap seconds. The full week number does not apply the legacy 1024-week
/// broadcast rollover.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GpsTime {
    /// Full GPS week number since January 6, 1980.
    pub week: i32,

    /// Seconds within the GPS week in the range `0.0..604800.0`.
    pub sec: f64,
}

impl GpsTime {
    /// Calculates the time difference in seconds between two GPS times.
    pub fn diff_secs(&self, other: &Self) -> f64 {
        let mut dt = self.sec - other.sec;
        dt += f64::from(self.week - other.week) * SECONDS_IN_WEEK;
        dt
    }

    /// Adds seconds without quantizing the continuous GPS timeline.
    pub fn add_secs(&self, dt: f64) -> Self {
        let seconds = self.sec + dt;
        let week_offset = seconds.div_euclid(SECONDS_IN_WEEK);
        Self {
            week: self.week + week_offset as i32,
            sec: seconds.rem_euclid(SECONDS_IN_WEEK),
        }
    }

    /// Floors seconds-of-week to an exact whole-second interval.
    pub(crate) fn floor_to_interval(&self, interval_seconds: u32) -> Self {
        let interval_seconds = f64::from(interval_seconds);
        Self {
            week: self.week,
            sec: (self.sec / interval_seconds).floor() * interval_seconds,
        }
    }
}

/// Represents a satellite range measurement at a specific GPS time.
#[derive(Clone, Debug, Default)]
pub struct TimeRange {
    /// GPS time of the measurement.
    pub time: GpsTime,

    /// Pseudorange measurement in meters, including signal delays.
    pub range: f64,

    /// Range rate in meters per second.
    pub rate: f64,

    /// Geometric distance in meters without signal delays.
    pub distance: f64,

    /// Satellite azimuth and elevation.
    pub azel: Azel,

    /// Ionospheric delay in meters.
    pub iono_delay: f64,
}

#[cfg(test)]
mod tests {
    use constants::SECONDS_IN_WEEK;

    use super::GpsTime;

    #[test]
    fn add_secs_preserves_sub_millisecond_composition() {
        let start = GpsTime {
            week: 2_000,
            sec: 10.0,
        };
        let mut time = start.clone();
        for _ in 0..10_000 {
            time = time.add_secs(0.0004);
        }
        assert!((time.diff_secs(&start) - 4.0).abs() < 1.0e-9);

        let thirds = start
            .add_secs(1.0 / 3.0)
            .add_secs(1.0 / 3.0)
            .add_secs(1.0 / 3.0);
        assert!((thirds.diff_secs(&start) - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn add_secs_normalizes_week_boundaries_without_quantization() {
        let start = GpsTime {
            week: 2_000,
            sec: SECONDS_IN_WEEK - 0.0004,
        };
        let end = start.add_secs(0.0008);
        assert_eq!(end.week, 2_001);
        assert!((end.sec - 0.0004).abs() < 1.0e-9);
    }
}
