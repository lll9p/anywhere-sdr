use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use gps::{Error, SignalGeneratorBuilder, read_navigation_data};

const RINEX_HEADER: &str = concat!(
    "     2              NAVIGATION DATA                         RINEX VERSION / TYPE\n",
    "CCRINEXN V1.6.0 UX  CDDIS               02-JAN-22 18:31     PGM / RUN BY / DATE \n",
    "IGS BROADCAST EPHEMERIS FILE                                COMMENT             \n",
    "    0.1211D-07 -0.7451D-08 -0.5960D-07  0.1192D-06          ION ALPHA           \n",
    "    0.1167D+06 -0.2458D+06 -0.6554D+05  0.1114D+07          ION BETA            \n",
    "    0.279396772385D-08 0.799360577730D-14   147456     2191 DELTA-UTC: A0,A1,T,W\n",
    "    18                                                      LEAP SECONDS        \n",
    "                                                            END OF HEADER       \n",
);

const RECORD_ROWS: &str = concat!(
    "    0.390000000000D+02-0.141125000000D+03 \
     0.398838041777D-08-0.624294238235D+00\n",
    "   -0.736303627491D-05 0.112181392033D-01 0.469572842121D-05 \
     0.515367499542D+04\n",
    "    0.518400000000D+06-0.316649675369D-07-0.103661124009D+01 \
     0.195577740669D-06\n",
    "    0.986418769490D+00 0.299750000000D+03 \
     0.884087601569D+00-0.813355308085D-08\n",
    "   -0.377872882780D-09 0.100000000000D+01 0.219000000000D+04 \
     0.000000000000D+00\n",
    "    0.200000000000D+01 0.000000000000D+00 0.512227416039D-08 \
     0.390000000000D+02\n",
    "    0.511218000000D+06 0.400000000000D+01 0.000000000000D+00 \
     0.000000000000D+00\n",
);

static TEST_DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn create() -> Result<Self, Error> {
        let sequence = TEST_DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "anywhere-sdr-ephemeris-lifecycle-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }

    fn write_navigation(
        &self, name: &str, navigation: &str,
    ) -> Result<PathBuf, Error> {
        let path = self.0.join(name);
        fs::write(&path, navigation)?;
        Ok(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!(
                "Failed to remove test directory {}: {error}",
                self.0.display()
            );
        }
    }
}

fn navigation(records: &[(usize, usize)]) -> String {
    let mut navigation = String::from(RINEX_HEADER);
    for &(prn, total_hours) in records {
        let day = 1 + total_hours / 24;
        let hour = total_hours % 24;
        let first_row = format!(
            "{prn:2} 22  1 {day:2} {hour:2}  0  0.0 \
             0.469126738608D-03-0.100044417195D-10 0.000000000000D+00\n"
        );
        navigation.push_str(&first_row);
        navigation.push_str(RECORD_ROWS);
    }
    navigation.push('\n');
    navigation
}

fn navigation_with_set_count(set_count: usize) -> String {
    let records = (0..set_count)
        .map(|index| (1, index * 2))
        .collect::<Vec<_>>();
    navigation(&records)
}

fn configured_builder(path: &Path) -> Result<SignalGeneratorBuilder, Error> {
    SignalGeneratorBuilder::default()
        .navigation_file(Some(path.to_path_buf()))?
        .location(Some(vec![35.681_298, 139.766_247, 10.0]))?
        .data_format(Some(8))
}

fn build_at(
    path: &Path, gps_calendar_time: &str,
) -> Result<gps::SignalGenerator, Error> {
    configured_builder(path)?
        .gps_calendar_time(Some(gps_calendar_time.into()))?
        .build()
}

fn expected_error<T>(result: Result<T, Error>) -> Result<Error, Error> {
    match result {
        Ok(_) => Err(Error::msg("expected operation to fail")),
        Err(error) => Ok(error),
    }
}

#[test]
fn reader_returns_exact_stored_set_counts_and_rejects_invalid_prns()
-> Result<(), Error> {
    let directory = TestDirectory::create()?;
    for expected_count in [1, 14, 15] {
        let path = directory.write_navigation(
            &format!("count-{expected_count}.nav"),
            &navigation_with_set_count(expected_count),
        )?;
        let (actual_count, ..) = read_navigation_data(&path)?;
        assert_eq!(actual_count, expected_count);
    }

    let invalid_path = directory
        .write_navigation("invalid-prn.nav", &navigation(&[(0, 0)]))?;
    for error in [
        expected_error(read_navigation_data(&invalid_path))?,
        expected_error(
            SignalGeneratorBuilder::default()
                .navigation_file(Some(invalid_path)),
        )?,
    ] {
        assert!(matches!(
            error,
            Error::Rinex(rinex::Error::Rule(message))
                if message.contains("GPS PRN 0")
        ));
    }
    Ok(())
}

#[test]
fn reader_and_builder_reject_ephemeris_set_overflow() -> Result<(), Error> {
    let directory = TestDirectory::create()?;
    for set_count in [16, 100] {
        let path = directory.write_navigation(
            &format!("overflow-{set_count}.nav"),
            &navigation_with_set_count(set_count),
        )?;
        let reader_error = expected_error(read_navigation_data(&path))?;
        assert!(matches!(reader_error, Error::TooManyEphemerisSets {
            max_supported: 15
        }));

        let builder_error = expected_error(
            SignalGeneratorBuilder::default()
                .navigation_file(Some(path.clone())),
        )?;
        assert!(matches!(builder_error, Error::TooManyEphemerisSets {
            max_supported: 15
        }));
    }
    Ok(())
}

#[test]
fn builder_preserves_navigation_loading_error_types() -> Result<(), Error> {
    let directory = TestDirectory::create()?;
    let missing_error = expected_error(
        SignalGeneratorBuilder::default()
            .navigation_file(Some(directory.0.join("missing.nav"))),
    )?;
    assert!(matches!(
        missing_error,
        Error::Rinex(rinex::error::Error::ReadRinex(_))
    ));

    let malformed_path =
        directory.write_navigation("malformed.nav", "not RINEX data")?;
    let malformed_error = expected_error(
        SignalGeneratorBuilder::default().navigation_file(Some(malformed_path)),
    )?;
    assert!(matches!(malformed_error, Error::Rinex(_)));
    Ok(())
}

#[test]
fn builder_selects_first_and_last_stored_sets() -> Result<(), Error> {
    let directory = TestDirectory::create()?;
    let path = directory
        .write_navigation("fifteen-sets.nav", &navigation_with_set_count(15))?;

    let first = build_at(&path, "2022-01-01T00:00:00")?;
    assert_eq!(first.valid_ephemerides_index, 0);

    let last = build_at(&path, "2022-01-02T04:00:00")?;
    assert_eq!(last.valid_ephemerides_index, 14);
    Ok(())
}

#[test]
fn selection_uses_exact_one_hour_boundaries_and_any_valid_prn()
-> Result<(), Error> {
    let directory = TestDirectory::create()?;
    let boundary_path = directory
        .write_navigation("boundary.nav", &navigation(&[(1, 0), (2, 2)]))?;
    let boundary = build_at(&boundary_path, "2022-01-01T01:00:00")?;
    assert_eq!(boundary.valid_ephemerides_index, 1);
    assert!(!boundary.ephemerides[1][0].vflg);
    assert!(boundary.ephemerides[1][1].vflg);

    let gap_path = directory
        .write_navigation("gap.nav", &navigation(&[(1, 0), (2, 4)]))?;
    let gap_error = expected_error(build_at(&gap_path, "2022-01-01T02:00:00"))?;
    assert!(matches!(gap_error, Error::NoCurrentEphemerides));
    Ok(())
}

#[test]
fn time_override_without_a_current_set_returns_an_error() -> Result<(), Error> {
    let directory = TestDirectory::create()?;
    let path = directory
        .write_navigation("override-gap.nav", &navigation_with_set_count(1))?;
    let error = expected_error(
        configured_builder(&path)?
            .gps_calendar_time(Some("2022-01-01T01:30:00".into()))?
            .time_override(Some(true))
            .build(),
    )?;
    assert!(matches!(error, Error::NoCurrentEphemerides));
    Ok(())
}

#[test]
fn runtime_rollover_uses_a_set_without_prn_one() -> Result<(), Error> {
    let directory = TestDirectory::create()?;
    let path = directory
        .write_navigation("rollover.nav", &navigation(&[(1, 0), (2, 2)]))?;
    let mut generator = configured_builder(&path)?
        .gps_calendar_time(Some("2022-01-01T00:59:59.9".into()))?
        .duration(Some(0.1))
        .sample_rate(Some(0.1))
        .frequency(Some(1_000_000))?
        .path_loss(Some(128))
        .verbose(Some(false))
        .build()?;
    generator.elevation_mask_degrees = -90.0;
    generator.initialize()?;
    assert!(generator.channels.iter().any(|channel| channel.prn == 1));

    generator.run_streaming::<_, Error>(|_| Ok(()))?;

    assert_eq!(generator.valid_ephemerides_index, 1);
    assert!(generator.channels.iter().all(|channel| channel.prn != 1));
    assert!(generator.channels.iter().any(|channel| channel.prn == 2));
    Ok(())
}
