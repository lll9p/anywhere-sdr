use std::{error::Error as StdError, fs, path::PathBuf, time::Duration};

use gps::{
    Error, GPS_UTC_LEAP_SECONDS, GpsCalendarDateTime, GpsTime,
    SignalGeneratorBuilder, UtcDateTime, read_navigation_data,
};
use jiff::{Timestamp, civil};
use rinex::Rinex;

type CalendarDate = (i32, i32, i32);
type LeapVector = (CalendarDate, CalendarDate, i64);

const AUTHORITATIVE_LEAPS: [LeapVector; 18] = [
    ((1981, 6, 30), (1981, 7, 1), 1),
    ((1982, 6, 30), (1982, 7, 1), 2),
    ((1983, 6, 30), (1983, 7, 1), 3),
    ((1985, 6, 30), (1985, 7, 1), 4),
    ((1987, 12, 31), (1988, 1, 1), 5),
    ((1989, 12, 31), (1990, 1, 1), 6),
    ((1990, 12, 31), (1991, 1, 1), 7),
    ((1992, 6, 30), (1992, 7, 1), 8),
    ((1993, 6, 30), (1993, 7, 1), 9),
    ((1994, 6, 30), (1994, 7, 1), 10),
    ((1995, 12, 31), (1996, 1, 1), 11),
    ((1997, 6, 30), (1997, 7, 1), 12),
    ((1998, 12, 31), (1999, 1, 1), 13),
    ((2005, 12, 31), (2006, 1, 1), 14),
    ((2008, 12, 31), (2009, 1, 1), 15),
    ((2012, 6, 30), (2012, 7, 1), 16),
    ((2015, 6, 30), (2015, 7, 1), 17),
    ((2016, 12, 31), (2017, 1, 1), 18),
];

fn gps_total_seconds(time: &GpsTime) -> f64 {
    f64::from(time.week) * 604_800.0 + time.sec
}

fn jiff_timestamp_elapsed(timestamp: Timestamp) -> Result<f64, jiff::Error> {
    let epoch: Timestamp = "1980-01-06T00:00:00Z".parse()?;
    let duration = timestamp.duration_since(epoch);
    Ok(duration.as_secs() as f64
        + f64::from(duration.subsec_nanos()) / 1_000_000_000.0)
}

fn jiff_civil_elapsed(value: civil::DateTime) -> Result<f64, jiff::Error> {
    let epoch: civil::DateTime = "1980-01-06T00:00:00".parse()?;
    let duration = value.duration_since(epoch);
    Ok(duration.as_secs() as f64
        + f64::from(duration.subsec_nanos()) / 1_000_000_000.0)
}

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1.0e-9);
}

fn assert_same_gps(actual: &GpsTime, expected_week: i32, expected_sec: f64) {
    assert_eq!(actual.week, expected_week);
    assert_close(actual.sec, expected_sec);
}

fn assert_same_utc(actual: &UtcDateTime, expected: &UtcDateTime) {
    assert_eq!(actual.year(), expected.year());
    assert_eq!(actual.month(), expected.month());
    assert_eq!(actual.day(), expected.day());
    assert_eq!(actual.hour(), expected.hour());
    assert_eq!(actual.minute(), expected.minute());
    assert_close(actual.second(), expected.second());
}

fn assert_same_gps_calendar(
    actual: &GpsCalendarDateTime, expected: &GpsCalendarDateTime,
) {
    assert_eq!(actual.year(), expected.year());
    assert_eq!(actual.month(), expected.month());
    assert_eq!(actual.day(), expected.day());
    assert_eq!(actual.hour(), expected.hour());
    assert_eq!(actual.minute(), expected.minute());
    assert_close(actual.second(), expected.second());
}

#[test]
fn leap_table_matches_iers_effective_midnights() {
    assert_eq!(GPS_UTC_LEAP_SECONDS.len(), AUTHORITATIVE_LEAPS.len());
    for (actual, expected) in
        GPS_UTC_LEAP_SECONDS.iter().zip(AUTHORITATIVE_LEAPS)
    {
        assert_eq!(
            (
                actual.insertion_year,
                actual.insertion_month,
                actual.insertion_day
            ),
            expected.0
        );
        assert_eq!(
            (
                actual.effective_year,
                actual.effective_month,
                actual.effective_day
            ),
            expected.1
        );
        assert_eq!(actual.gps_utc_offset, expected.2);
    }
}

#[test]
fn every_leap_boundary_matches_independent_jiff_timeline()
-> Result<(), Box<dyn StdError>> {
    for (insertion, effective, offset) in AUTHORITATIVE_LEAPS {
        let effective_text = format!(
            "{:04}-{:02}-{:02}T00:00:00Z",
            effective.0, effective.1, effective.2
        );
        let effective_timestamp: Timestamp = effective_text.parse()?;
        let before_timestamp =
            effective_timestamp.checked_sub(Duration::from_secs(1))?;
        let before_utc = UtcDateTime::from_timestamp(&before_timestamp)?;
        let after_utc = UtcDateTime::from_timestamp(&effective_timestamp)?;
        let before_gps = GpsTime::from_utc(&before_utc)?;
        let after_gps = GpsTime::from_utc(&after_utc)?;

        assert_close(
            gps_total_seconds(&before_gps),
            jiff_timestamp_elapsed(before_timestamp)? + (offset - 1) as f64,
        );
        assert_close(
            gps_total_seconds(&after_gps),
            jiff_timestamp_elapsed(effective_timestamp)? + offset as f64,
        );
        assert_close(after_gps.diff_secs(&before_gps), 2.0);
        assert_same_utc(&before_gps.to_utc()?, &before_utc);
        assert_same_utc(&after_gps.to_utc()?, &after_utc);

        let leap_utc = UtcDateTime::new(
            insertion.0,
            insertion.1,
            insertion.2,
            23,
            59,
            60.0,
        )?;
        let leap_gps = GpsTime::from_utc(&leap_utc)?;
        assert_close(leap_gps.diff_secs(&before_gps), 1.0);
        assert_close(after_gps.diff_secs(&leap_gps), 1.0);
        assert_same_utc(&leap_gps.to_utc()?, &leap_utc);
    }
    Ok(())
}

#[test]
fn gps_calendar_vectors_use_full_gregorian_century_rules()
-> Result<(), Box<dyn StdError>> {
    let vectors = [
        ("1980-01-06T00:00:00", 0, 0.0),
        ("1999-08-22T00:00:00", 1_024, 0.0),
        ("2019-04-07T00:00:00", 2_048, 0.0),
        ("2099-12-31T23:59:59", 6_260, 431_999.0),
        ("2100-02-28T23:59:59", 6_269, 86_399.0),
        ("2100-03-01T00:00:00", 6_269, 86_400.0),
        ("2400-03-01T00:00:00", 21_922, 259_200.0),
    ];

    for (text, expected_week, expected_sec) in vectors {
        let civil: civil::DateTime = text.parse()?;
        let calendar = GpsCalendarDateTime::from_civil(civil);
        let gps = GpsTime::from_gps_calendar(&calendar)?;
        assert_same_gps(&gps, expected_week, expected_sec);
        assert_close(gps_total_seconds(&gps), jiff_civil_elapsed(civil)?);
        assert_same_gps_calendar(&gps.to_gps_calendar()?, &calendar);
    }
    Ok(())
}

#[test]
fn utc_and_gps_calendar_labels_have_explicit_distinct_semantics()
-> Result<(), Box<dyn StdError>> {
    let epoch_timestamp: Timestamp = "1980-01-06T00:00:00Z".parse()?;
    let epoch_utc = UtcDateTime::from_timestamp(&epoch_timestamp)?;
    let epoch_gps = GpsTime::from_utc(&epoch_utc)?;
    assert_same_gps(&epoch_gps, 0, 0.0);
    assert_same_utc(&epoch_gps.to_utc()?, &epoch_utc);

    let timestamp: Timestamp = "2017-01-01T00:00:00Z".parse()?;
    let utc = UtcDateTime::from_timestamp(&timestamp)?;
    let utc_gps = GpsTime::from_utc(&utc)?;
    assert_same_gps(&utc_gps, 1_930, 18.0);

    let civil: civil::DateTime = "2017-01-01T00:00:00".parse()?;
    let gps_calendar = GpsCalendarDateTime::from_civil(civil);
    let labelled_gps = GpsTime::from_gps_calendar(&gps_calendar)?;
    assert_same_gps(&labelled_gps, 1_930, 0.0);
    assert_close(utc_gps.diff_secs(&labelled_gps), 18.0);

    for text in ["2026-07-14T00:00:00Z", "2026-12-31T23:59:59.5Z"] {
        let timestamp: Timestamp = text.parse()?;
        let utc = UtcDateTime::from_timestamp(&timestamp)?;
        let gps = GpsTime::from_utc(&utc)?;
        assert_close(
            gps_total_seconds(&gps),
            jiff_timestamp_elapsed(timestamp)? + 18.0,
        );
        assert_same_utc(&gps.to_utc()?, &utc);
    }
    Ok(())
}

#[test]
fn utc_rollover_vectors_reach_exact_full_gps_weeks()
-> Result<(), Box<dyn StdError>> {
    let vectors = [
        ("1999-08-21T23:59:47Z", 1_024),
        ("2019-04-06T23:59:42Z", 2_048),
    ];

    for (text, expected_week) in vectors {
        let timestamp: Timestamp = text.parse()?;
        let utc = UtcDateTime::from_timestamp(&timestamp)?;
        let gps = GpsTime::from_utc(&utc)?;
        assert_same_gps(&gps, expected_week, 0.0);
        assert_same_utc(&gps.to_utc()?, &utc);
    }
    Ok(())
}

#[test]
fn invalid_and_pre_epoch_calendar_inputs_return_typed_errors()
-> Result<(), Error> {
    assert!(matches!(
        GpsCalendarDateTime::new(2100, 2, 29, 0, 0, 0.0),
        Err(Error::InvalidCalendarDate {
            scale: "GPS calendar",
            ..
        })
    ));
    assert!(matches!(
        UtcDateTime::new(2026, 13, 1, 0, 0, 0.0),
        Err(Error::InvalidCalendarDate { scale: "UTC", .. })
    ));
    assert!(matches!(
        UtcDateTime::new(2026, 1, 1, 0, 0, f64::NAN),
        Err(Error::InvalidCalendarDate { scale: "UTC", .. })
    ));
    assert!(matches!(
        GpsCalendarDateTime::new(2026, 1, 1, 0, 0, f64::INFINITY),
        Err(Error::InvalidCalendarDate {
            scale: "GPS calendar",
            ..
        })
    ));

    for second in [f64::NAN, -0.1, 604_800.0] {
        assert!(matches!(
            GpsTime {
                week: 0,
                sec: second
            }
            .to_utc(),
            Err(Error::InvalidGpsTime(_))
        ));
    }

    let pre_epoch_utc = UtcDateTime::new(1980, 1, 5, 23, 59, 59.0)?;
    assert!(matches!(
        GpsTime::from_utc(&pre_epoch_utc),
        Err(Error::TimeBeforeGpsEpoch { scale: "UTC" })
    ));
    let pre_epoch_gps = GpsCalendarDateTime::new(1979, 12, 31, 0, 0, 0.0)?;
    assert!(matches!(
        GpsTime::from_gps_calendar(&pre_epoch_gps),
        Err(Error::TimeBeforeGpsEpoch {
            scale: "GPS calendar"
        })
    ));
    Ok(())
}

fn parsed_rinex_epoch_gps_time(
    navigation: &str, epoch_fields: &str,
) -> Result<GpsTime, Box<dyn StdError>> {
    let navigation =
        navigation.replacen(" 1 22  1  1  0  0  0.0", epoch_fields, 1);
    let rinex = Rinex::read_string(&navigation)?;
    let first = rinex.ephemerides.first().ok_or(Error::NoEphemeris)?;
    let calendar = GpsCalendarDateTime::try_from(&first.time_of_clock)?;
    Ok(GpsTime::from_gps_calendar(&calendar)?)
}

#[test]
fn rinex_2_epoch_pivot_preserves_exact_gps_calendar_time()
-> Result<(), Box<dyn StdError>> {
    let navigation_path = PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("resources/brdc0010.22n");
    let navigation = fs::read_to_string(navigation_path)?;
    let vectors = [
        (" 1 79  1  1  0  0  0.0", 5_165, 0.0),
        (" 1 80  1  6  0  0  0.0", 0, 0.0),
        (" 1 99  8 21 23 59 59.0", 1_023, 604_799.0),
        (" 1 99  8 22  0  0  0.0", 1_024, 0.0),
        (" 1 00  1  2  0  0  0.0", 1_043, 0.0),
    ];

    for (epoch_fields, expected_week, expected_sec) in vectors {
        let gps = parsed_rinex_epoch_gps_time(&navigation, epoch_fields)?;
        assert_same_gps(&gps, expected_week, expected_sec);
    }
    Ok(())
}

#[test]
fn rinex_epochs_remain_gps_calendar_labels() -> Result<(), Box<dyn StdError>> {
    let navigation_path = PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("resources/brdc0010.22n");
    let (_, _, ephemerides) = read_navigation_data(&navigation_path)?;
    let first = ephemerides
        .iter()
        .flat_map(|set| set.iter())
        .find(|ephemeris| ephemeris.vflg)
        .ok_or(Error::NoEphemeris)?;
    let expected_calendar = GpsCalendarDateTime::new(2022, 1, 1, 0, 0, 0.0)?;
    let expected_gps = GpsTime::from_gps_calendar(&expected_calendar)?;

    assert_same_gps_calendar(&first.time_of_clock, &expected_calendar);
    assert_same_gps(&first.toc, expected_gps.week, expected_gps.sec);
    Ok(())
}

fn configured_builder(
    navigation_path: PathBuf,
) -> Result<SignalGeneratorBuilder, Error> {
    SignalGeneratorBuilder::default()
        .navigation_file(Some(navigation_path))?
        .location(Some(vec![35.0, 139.0, 10.0]))?
        .data_format(Some(8))
}

fn overridden_builder_utc_time(value: &str) -> Result<GpsTime, Error> {
    let navigation_path = PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("resources/brdc0010.22n");
    let generator = configured_builder(navigation_path)?
        .utc_time(Some(value.into()))?
        .time_override(Some(true))
        .build()?;
    Ok(generator.receiver_gps_time)
}

#[test]
fn builder_utc_and_legacy_gps_calendar_inputs_are_explicit()
-> Result<(), Box<dyn StdError>> {
    let navigation_path = PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("resources/brdc0010.22n");
    let utc_generator = configured_builder(navigation_path.clone())?
        .utc_time(Some("2022-01-01T00:00:00Z".into()))?
        .build()?;
    let gps_calendar_generator = configured_builder(navigation_path.clone())?
        .gps_calendar_time(Some("2022-01-01T00:00:00".into()))?
        .build()?;
    let offset_generator = configured_builder(navigation_path)?
        .utc_time(Some("2022-01-01T01:00:00+01:00".into()))?
        .build()?;
    #[allow(deprecated)]
    let compatibility_generator = configured_builder(
        PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
            .join("resources/brdc0010.22n"),
    )?
    .time(Some("2022-01-01T00:00:00Z".into()))?
    .build()?;

    assert_close(
        utc_generator
            .receiver_gps_time
            .diff_secs(&gps_calendar_generator.receiver_gps_time),
        18.0,
    );
    assert_close(
        utc_generator
            .receiver_gps_time
            .diff_secs(&offset_generator.receiver_gps_time),
        0.0,
    );
    assert_close(
        utc_generator
            .receiver_gps_time
            .diff_secs(&compatibility_generator.receiver_gps_time),
        0.0,
    );
    Ok(())
}

#[test]
fn builder_preserves_fractional_utc_leap_labels()
-> Result<(), Box<dyn StdError>> {
    let before = overridden_builder_utc_time("2016-12-31T23:59:59.25Z")?;
    let leap = overridden_builder_utc_time("2016-12-31T23:59:60.25Z")?;
    let after = overridden_builder_utc_time("2017-01-01T00:00:00.25Z")?;

    let expected_before =
        GpsTime::from_utc(&UtcDateTime::new(2016, 12, 31, 23, 59, 59.25)?)?;
    let expected_leap =
        GpsTime::from_utc(&UtcDateTime::new(2016, 12, 31, 23, 59, 60.25)?)?;
    let expected_after =
        GpsTime::from_utc(&UtcDateTime::new(2017, 1, 1, 0, 0, 0.25)?)?;

    assert_same_gps(&before, expected_before.week, expected_before.sec);
    assert_same_gps(&leap, expected_leap.week, expected_leap.sec);
    assert_same_gps(&after, expected_after.week, expected_after.sec);
    assert_close(leap.diff_secs(&before), 1.0);
    assert_close(after.diff_secs(&leap), 1.0);
    Ok(())
}

#[test]
fn builder_rejects_invalid_or_mixed_scale_labels() {
    for value in [
        "2016-12-30T23:59:60.25Z",
        "2016-12-31T23:59:60.25+00:00",
        "2026-13-01T00:00:00Z",
        "2026-01-01T00:00:00",
        "not-a-timestamp",
    ] {
        assert!(matches!(
            SignalGeneratorBuilder::default().utc_time(Some(value.into())),
            Err(Error::InvalidCalendarDate { scale: "UTC", .. })
        ));
    }

    assert!(
        SignalGeneratorBuilder::default()
            .utc_time(Some("now".into()))
            .is_ok()
    );
    assert!(matches!(
        SignalGeneratorBuilder::default().gps_calendar_time(Some("now".into())),
        Err(Error::InvalidCalendarDate {
            scale: "GPS calendar",
            ..
        })
    ));
    for value in [
        "2017-01-01T00:00:00Z",
        "2017-01-01T00:00:00+00:00",
        "2100-02-29T00:00:00",
        "not-a-calendar-label",
    ] {
        assert!(matches!(
            SignalGeneratorBuilder::default()
                .gps_calendar_time(Some(value.into())),
            Err(Error::InvalidCalendarDate {
                scale: "GPS calendar",
                ..
            })
        ));
    }
}
