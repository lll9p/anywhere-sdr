#![allow(
    clippy::missing_docs_in_private_items,
    reason = "private arithmetic is exposed through documented scale APIs"
)]

use constants::{
    SECONDS_IN_DAY, SECONDS_IN_HOUR, SECONDS_IN_MINUTE, SECONDS_IN_WEEK,
};

use super::{
    GpsTime,
    leap_seconds::{GPS_UTC_LEAP_SECONDS, LeapSecond},
};
use crate::Error;

const GPS_EPOCH_UNIX_DAY: i64 = 3_657;
const GPS_CALENDAR_SCALE: &str = "GPS calendar";
const UTC_SCALE: &str = "UTC";

#[derive(Clone, Debug, PartialEq)]
struct CalendarFields {
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
    second: f64,
}

impl CalendarFields {
    #[allow(clippy::too_many_arguments)]
    fn new(
        scale: &'static str, year: i32, month: i32, day: i32, hour: i32,
        minute: i32, second: f64, maximum_second: f64,
    ) -> Result<Self, Error> {
        if !(1..=12).contains(&month) {
            return Err(invalid_calendar(scale, "month must be in 1..=12"));
        }
        if !(1..=days_in_month(year, month)).contains(&day) {
            return Err(invalid_calendar(
                scale,
                "day is outside the selected month",
            ));
        }
        if !(0..=23).contains(&hour) {
            return Err(invalid_calendar(scale, "hour must be in 0..=23"));
        }
        if !(0..=59).contains(&minute) {
            return Err(invalid_calendar(scale, "minute must be in 0..=59"));
        }
        if !second.is_finite() || !(0.0..maximum_second).contains(&second) {
            return Err(invalid_calendar(
                scale,
                "second is non-finite or outside the permitted range",
            ));
        }
        Ok(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        })
    }

    fn from_jiff(value: jiff::civil::DateTime) -> Self {
        Self {
            year: i32::from(value.year()),
            month: i32::from(value.month()),
            day: i32::from(value.day()),
            hour: i32::from(value.hour()),
            minute: i32::from(value.minute()),
            second: f64::from(value.second())
                + f64::from(value.subsec_nanosecond()) / 1_000_000_000.0,
        }
    }

    fn elapsed_parts(&self, scale: &'static str) -> Result<(i64, f64), Error> {
        let days = days_since_gps_epoch(self.year, self.month, self.day);
        if days < 0 {
            return Err(Error::TimeBeforeGpsEpoch { scale });
        }
        let integral_second = self.second.floor() as i64;
        let fraction = self.second - integral_second as f64;
        let whole = days
            .checked_mul(SECONDS_IN_DAY as i64)
            .and_then(|value| {
                value.checked_add(i64::from(self.hour) * SECONDS_IN_HOUR as i64)
            })
            .and_then(|value| {
                value.checked_add(
                    i64::from(self.minute) * SECONDS_IN_MINUTE as i64,
                )
            })
            .and_then(|value| value.checked_add(integral_second))
            .ok_or_else(|| invalid_calendar(scale, "date exceeds GPS range"))?;
        Ok((whole, fraction))
    }
}

/// A civil calendar label expressed in the GPS system time scale.
///
/// GPS calendar labels have ordinary Gregorian dates but never apply UTC leap
/// seconds. RINEX navigation epochs use this scale.
#[derive(Clone, Debug, PartialEq)]
pub struct GpsCalendarDateTime(CalendarFields);

impl GpsCalendarDateTime {
    /// Creates a validated GPS-system calendar label.
    pub fn new(
        year: i32, month: i32, day: i32, hour: i32, minute: i32, second: f64,
    ) -> Result<Self, Error> {
        CalendarFields::new(
            GPS_CALENDAR_SCALE,
            year,
            month,
            day,
            hour,
            minute,
            second,
            60.0,
        )
        .map(Self)
    }

    /// Converts a validated generic civil value into a GPS calendar label.
    pub fn from_civil(value: jiff::civil::DateTime) -> Self {
        Self(CalendarFields::from_jiff(value))
    }

    /// Returns the calendar year.
    pub fn year(&self) -> i32 {
        self.0.year
    }

    /// Returns the calendar month.
    pub fn month(&self) -> i32 {
        self.0.month
    }

    /// Returns the day of the month.
    pub fn day(&self) -> i32 {
        self.0.day
    }

    /// Returns the hour.
    pub fn hour(&self) -> i32 {
        self.0.hour
    }

    /// Returns the minute.
    pub fn minute(&self) -> i32 {
        self.0.minute
    }

    /// Returns the second, including any fractional part.
    pub fn second(&self) -> f64 {
        self.0.second
    }
}

impl TryFrom<&rinex::ephemeris::GpsCalendarDateTime> for GpsCalendarDateTime {
    type Error = Error;

    fn try_from(
        value: &rinex::ephemeris::GpsCalendarDateTime,
    ) -> Result<Self, Self::Error> {
        Self::new(
            value.year,
            value.month,
            value.day,
            value.hour,
            value.minute,
            value.second,
        )
    }
}

impl Default for GpsCalendarDateTime {
    fn default() -> Self {
        Self(CalendarFields {
            year: 1980,
            month: 1,
            day: 6,
            hour: 0,
            minute: 0,
            second: 0.0,
        })
    }
}

/// A validated civil instant in the UTC time scale.
///
/// The value may represent `23:59:60` only on a leap second date present in
/// the crate's maintained GPS-UTC table.
#[derive(Clone, Debug, PartialEq)]
pub struct UtcDateTime(CalendarFields);

impl UtcDateTime {
    /// Creates a validated UTC civil instant.
    pub fn new(
        year: i32, month: i32, day: i32, hour: i32, minute: i32, second: f64,
    ) -> Result<Self, Error> {
        let fields = CalendarFields::new(
            UTC_SCALE, year, month, day, hour, minute, second, 61.0,
        )?;
        if second >= 60.0 && leap_second_for_insertion(&fields).is_none() {
            return Err(invalid_calendar(
                UTC_SCALE,
                "second 60 is not valid at this UTC instant",
            ));
        }
        Ok(Self(fields))
    }

    /// Converts a precise timestamp to its UTC civil representation.
    pub fn from_timestamp(value: &jiff::Timestamp) -> Result<Self, Error> {
        let zoned = value.in_tz("UTC")?;
        Ok(Self(CalendarFields::from_jiff(zoned.datetime())))
    }

    /// Returns the calendar year.
    pub fn year(&self) -> i32 {
        self.0.year
    }

    /// Returns the calendar month.
    pub fn month(&self) -> i32 {
        self.0.month
    }

    /// Returns the day of the month.
    pub fn day(&self) -> i32 {
        self.0.day
    }

    /// Returns the hour.
    pub fn hour(&self) -> i32 {
        self.0.hour
    }

    /// Returns the minute.
    pub fn minute(&self) -> i32 {
        self.0.minute
    }

    /// Returns the second, including a fractional or leap-second part.
    pub fn second(&self) -> f64 {
        self.0.second
    }
}

impl Default for UtcDateTime {
    fn default() -> Self {
        Self(CalendarFields {
            year: 1980,
            month: 1,
            day: 6,
            hour: 0,
            minute: 0,
            second: 0.0,
        })
    }
}

impl GpsTime {
    /// Converts a GPS-system calendar label to continuous full-week GPS time.
    pub fn from_gps_calendar(
        time: &GpsCalendarDateTime,
    ) -> Result<Self, Error> {
        let (whole, fraction) = time.0.elapsed_parts(GPS_CALENDAR_SCALE)?;
        Self::from_elapsed_parts(whole, fraction)
    }

    /// Converts a UTC civil instant to continuous full-week GPS time.
    pub fn from_utc(time: &UtcDateTime) -> Result<Self, Error> {
        let (whole, fraction) = time.0.elapsed_parts(UTC_SCALE)?;
        let offset = if time.second() >= 60.0 {
            leap_second_for_insertion(&time.0)
                .map_or(0, |leap| leap.gps_utc_offset - 1)
        } else {
            gps_utc_offset_at_day(days_since_gps_epoch(
                time.year(),
                time.month(),
                time.day(),
            ))
        };
        Self::from_elapsed_parts(whole + offset, fraction)
    }

    /// Converts continuous GPS time to a GPS-system calendar label.
    pub fn to_gps_calendar(&self) -> Result<GpsCalendarDateTime, Error> {
        let (whole, fraction) = self.elapsed_parts()?;
        let fields = fields_from_elapsed(whole, fraction, GPS_CALENDAR_SCALE)?;
        Ok(GpsCalendarDateTime(fields))
    }

    /// Converts continuous GPS time to a UTC civil instant.
    pub fn to_utc(&self) -> Result<UtcDateTime, Error> {
        let (whole, fraction) = self.elapsed_parts()?;
        if let Some(leap) = GPS_UTC_LEAP_SECONDS
            .iter()
            .copied()
            .find(|leap| whole == gps_effective_seconds(*leap) - 1)
        {
            return UtcDateTime::new(
                leap.insertion_year,
                leap.insertion_month,
                leap.insertion_day,
                23,
                59,
                60.0 + fraction,
            );
        }
        let offset = GPS_UTC_LEAP_SECONDS
            .iter()
            .copied()
            .take_while(|leap| gps_effective_seconds(*leap) <= whole)
            .last()
            .map_or(0, |leap| leap.gps_utc_offset);
        let fields = fields_from_elapsed(whole - offset, fraction, UTC_SCALE)?;
        Ok(UtcDateTime(fields))
    }

    fn from_elapsed_parts(whole: i64, fraction: f64) -> Result<Self, Error> {
        if whole < 0 {
            return Err(Error::TimeBeforeGpsEpoch { scale: "GPS" });
        }
        let week = whole.div_euclid(SECONDS_IN_WEEK as i64);
        let week = i32::try_from(week).map_err(|_| {
            Error::InvalidGpsTime("week exceeds the supported i32 range".into())
        })?;
        let sec = whole.rem_euclid(SECONDS_IN_WEEK as i64) as f64 + fraction;
        Ok(Self { week, sec })
    }

    fn elapsed_parts(&self) -> Result<(i64, f64), Error> {
        if self.week < 0 {
            return Err(Error::TimeBeforeGpsEpoch { scale: "GPS" });
        }
        if !self.sec.is_finite() || !(0.0..SECONDS_IN_WEEK).contains(&self.sec)
        {
            return Err(Error::InvalidGpsTime(
                "seconds-of-week must be finite and normalized".into(),
            ));
        }
        let integral_second = self.sec.floor() as i64;
        let fraction = self.sec - integral_second as f64;
        let whole = i64::from(self.week)
            .checked_mul(SECONDS_IN_WEEK as i64)
            .and_then(|value| value.checked_add(integral_second))
            .ok_or_else(|| {
                Error::InvalidGpsTime("elapsed time overflow".into())
            })?;
        Ok((whole, fraction))
    }
}

fn invalid_calendar(scale: &'static str, reason: &str) -> Error {
    Error::InvalidCalendarDate {
        scale,
        reason: reason.into(),
    }
}

fn gps_effective_seconds(leap: LeapSecond) -> i64 {
    days_since_gps_epoch(
        leap.effective_year,
        leap.effective_month,
        leap.effective_day,
    ) * SECONDS_IN_DAY as i64
        + leap.gps_utc_offset
}

fn leap_second_for_insertion(fields: &CalendarFields) -> Option<LeapSecond> {
    if fields.hour != 23 || fields.minute != 59 {
        return None;
    }
    GPS_UTC_LEAP_SECONDS.iter().copied().find(|leap| {
        (fields.year, fields.month, fields.day)
            == (
                leap.insertion_year,
                leap.insertion_month,
                leap.insertion_day,
            )
    })
}

fn gps_utc_offset_at_day(day: i64) -> i64 {
    GPS_UTC_LEAP_SECONDS
        .iter()
        .copied()
        .take_while(|leap| {
            days_since_gps_epoch(
                leap.effective_year,
                leap.effective_month,
                leap.effective_day,
            ) <= day
        })
        .last()
        .map_or(0, |leap| leap.gps_utc_offset)
}

fn fields_from_elapsed(
    whole: i64, fraction: f64, scale: &'static str,
) -> Result<CalendarFields, Error> {
    if whole < 0 {
        return Err(Error::TimeBeforeGpsEpoch { scale });
    }
    let days = whole.div_euclid(SECONDS_IN_DAY as i64);
    let seconds_of_day = whole.rem_euclid(SECONDS_IN_DAY as i64);
    let (year, month, day) = civil_from_days(GPS_EPOCH_UNIX_DAY + days)?;
    let hour = seconds_of_day / SECONDS_IN_HOUR as i64;
    let minute = seconds_of_day.rem_euclid(SECONDS_IN_HOUR as i64)
        / SECONDS_IN_MINUTE as i64;
    let second =
        seconds_of_day.rem_euclid(SECONDS_IN_MINUTE as i64) as f64 + fraction;
    CalendarFields::new(
        scale,
        year,
        month,
        day,
        i32::try_from(hour)
            .map_err(|_| invalid_calendar(scale, "hour overflow"))?,
        i32::try_from(minute)
            .map_err(|_| invalid_calendar(scale, "minute overflow"))?,
        second,
        60.0,
    )
}

fn days_since_gps_epoch(year: i32, month: i32, day: i32) -> i64 {
    days_from_civil(year, month, day) - GPS_EPOCH_UNIX_DAY
}

fn days_from_civil(year: i32, month: i32, day: i32) -> i64 {
    let mut year = i64::from(year);
    let month = i64::from(month);
    let day = i64::from(day);
    year -= i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let shifted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era =
        year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> Result<(i32, i32, i32), Error> {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524
        - day_of_era / 146_096)
        / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year =
        day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    Ok((
        i32::try_from(year).map_err(|_| {
            Error::InvalidGpsTime("calendar year exceeds i32 range".into())
        })?,
        i32::try_from(month).map_err(|_| {
            Error::InvalidGpsTime("calendar month exceeds i32 range".into())
        })?,
        i32::try_from(day).map_err(|_| {
            Error::InvalidGpsTime("calendar day exceeds i32 range".into())
        })?,
    ))
}

fn days_in_month(year: i32, month: i32) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_gregorian_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_gregorian_leap_year(year: i32) -> bool {
    year.rem_euclid(4) == 0
        && (year.rem_euclid(100) != 0 || year.rem_euclid(400) == 0)
}
