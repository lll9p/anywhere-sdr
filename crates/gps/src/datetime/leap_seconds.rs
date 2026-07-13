/// One leap second in the maintained GPS-UTC history.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LeapSecond {
    /// UTC year whose last minute contains `23:59:60`.
    pub insertion_year: i32,
    /// UTC month whose last minute contains `23:59:60`.
    pub insertion_month: i32,
    /// UTC day whose last minute contains `23:59:60`.
    pub insertion_day: i32,
    /// UTC year at which the new offset is effective from midnight.
    pub effective_year: i32,
    /// UTC month at which the new offset is effective from midnight.
    pub effective_month: i32,
    /// UTC day at which the new offset is effective from midnight.
    pub effective_day: i32,
    /// GPS minus UTC in seconds after the transition.
    pub gps_utc_offset: i64,
}

impl LeapSecond {
    /// Constructs one compile-time leap history entry.
    const fn new(
        insertion: (i32, i32, i32), effective: (i32, i32, i32),
        gps_utc_offset: i64,
    ) -> Self {
        Self {
            insertion_year: insertion.0,
            insertion_month: insertion.1,
            insertion_day: insertion.2,
            effective_year: effective.0,
            effective_month: effective.1,
            effective_day: effective.2,
            gps_utc_offset,
        }
    }
}

/// GPS-UTC leap-second history since the GPS epoch.
///
/// Dates and offsets follow the IERS Bulletin C effective-date history in
/// <https://hpiers.obspm.fr/iers/bul/bulc/ntp/leap-seconds.list>, whose current
/// publication expires 2027-06-28. No leap second has been introduced after
/// 2017-01-01, so the current GPS-UTC offset is 18 seconds. Extend this table
/// when IERS announces another event.
pub const GPS_UTC_LEAP_SECONDS: [LeapSecond; 18] = [
    LeapSecond::new((1981, 6, 30), (1981, 7, 1), 1),
    LeapSecond::new((1982, 6, 30), (1982, 7, 1), 2),
    LeapSecond::new((1983, 6, 30), (1983, 7, 1), 3),
    LeapSecond::new((1985, 6, 30), (1985, 7, 1), 4),
    LeapSecond::new((1987, 12, 31), (1988, 1, 1), 5),
    LeapSecond::new((1989, 12, 31), (1990, 1, 1), 6),
    LeapSecond::new((1990, 12, 31), (1991, 1, 1), 7),
    LeapSecond::new((1992, 6, 30), (1992, 7, 1), 8),
    LeapSecond::new((1993, 6, 30), (1993, 7, 1), 9),
    LeapSecond::new((1994, 6, 30), (1994, 7, 1), 10),
    LeapSecond::new((1995, 12, 31), (1996, 1, 1), 11),
    LeapSecond::new((1997, 6, 30), (1997, 7, 1), 12),
    LeapSecond::new((1998, 12, 31), (1999, 1, 1), 13),
    LeapSecond::new((2005, 12, 31), (2006, 1, 1), 14),
    LeapSecond::new((2008, 12, 31), (2009, 1, 1), 15),
    LeapSecond::new((2012, 6, 30), (2012, 7, 1), 16),
    LeapSecond::new((2015, 6, 30), (2015, 7, 1), 17),
    LeapSecond::new((2016, 12, 31), (2017, 1, 1), 18),
];
