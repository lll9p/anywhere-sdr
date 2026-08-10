use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use super::{read_user_motion, read_user_motion_llh};
use crate::Error;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
const EPSILON: f64 = 1.0e-12;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= EPSILON,
        "expected {expected}, got {actual}"
    );
}

fn fixture(contents: &str) -> Result<PathBuf, std::io::Error> {
    let path = std::env::temp_dir().join(format!(
        "anywhere-sdr-user-motion-{}-{}.csv",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&path, contents)?;
    Ok(path)
}

fn assert_invalid_timestamp(
    contents: &str, expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = fixture(contents)?;
    let result = read_user_motion(&path);
    fs::remove_file(path)?;
    assert!(
        matches!(result, Err(Error::InvalidUserMotionFormat(ref message)) if message.contains(expected)),
        "unexpected result: {result:?}"
    );
    Ok(())
}

#[test]
fn ecef_timestamps_are_normalized_and_preserved()
-> Result<(), Box<dyn std::error::Error>> {
    let path = fixture(
        "12.25,-3813477.954,3554276.552,3662785.237\n12.375,-3813477.599,\
         3554276.226,3662785.918\n13.0,-3813477.240,3554275.906,3662786.598\n",
    )?;
    let samples = read_user_motion(&path)?;
    fs::remove_file(path)?;

    assert_eq!(samples.len(), 3);
    for (sample, expected) in samples.iter().zip([0.0, 0.125, 0.75]) {
        assert_close(sample.elapsed_seconds, expected);
    }
    assert_close(samples[0].position_ecef.x, -3_813_477.954);
    assert_close(samples[2].position_ecef.z, 3_662_786.598);
    Ok(())
}

#[test]
fn llh_timestamps_share_the_normalization_contract()
-> Result<(), Box<dyn std::error::Error>> {
    let path = fixture(
        "5.5,35.274015965,137.014864092,99.999\n5.75,35.274023485,137.\
         014864053,99.999\n6.5,35.274030991,137.014863935,99.999\n",
    )?;
    let samples = read_user_motion_llh(&path)?;
    fs::remove_file(path)?;

    assert_eq!(samples.len(), 3);
    assert_close(samples[0].elapsed_seconds, 0.0);
    assert_close(samples[1].elapsed_seconds, 0.25);
    assert_close(samples[2].elapsed_seconds, 1.0);
    assert!(samples.iter().all(|sample| {
        sample.position_ecef.x.is_finite()
            && sample.position_ecef.y.is_finite()
            && sample.position_ecef.z.is_finite()
    }));
    Ok(())
}

#[test]
fn empty_and_single_record_files_preserve_the_endpoint_contract()
-> Result<(), Box<dyn std::error::Error>> {
    let empty_path = fixture("")?;
    let empty_result = read_user_motion(&empty_path);
    fs::remove_file(empty_path)?;
    assert!(matches!(
        empty_result,
        Err(Error::InvalidUserMotionFormat(ref message))
            if message.contains("No valid motion records")
    ));

    let single_path = fixture("-7.5,-3813477.954,3554276.552,3662785.237\n")?;
    let samples = read_user_motion(&single_path)?;
    fs::remove_file(single_path)?;
    assert_eq!(samples.len(), 1);
    assert_close(samples[0].elapsed_seconds, 0.0);
    Ok(())
}

#[test]
fn llh_rejects_every_invalid_timestamp_class()
-> Result<(), Box<dyn std::error::Error>> {
    let position = "35.274015965,137.014864092,99.999";
    for (contents, expected) in [
        (format!("bad,{position}\n"), "invalid timestamp"),
        (format!("NaN,{position}\n"), "timestamp must be finite"),
        (
            format!("1.0,{position}\n1.0,{position}\n"),
            "must be greater than the previous timestamp",
        ),
        (
            format!("2.0,{position}\n1.0,{position}\n"),
            "must be greater than the previous timestamp",
        ),
        (
            format!("-1e308,{position}\n1e308,{position}\n"),
            "normalized elapsed timestamp is not finite",
        ),
    ] {
        let path = fixture(&contents)?;
        let result = read_user_motion_llh(&path);
        fs::remove_file(path)?;
        assert!(
            matches!(result, Err(Error::InvalidUserMotionFormat(ref message)) if message.contains(expected)),
            "unexpected result: {result:?}"
        );
    }
    Ok(())
}

#[test]
fn malformed_and_non_finite_timestamps_are_rejected()
-> Result<(), Box<dyn std::error::Error>> {
    for (timestamp, expected) in [
        ("not-a-time", "invalid timestamp"),
        ("NaN", "timestamp must be finite"),
        ("inf", "timestamp must be finite"),
        ("-inf", "timestamp must be finite"),
    ] {
        assert_invalid_timestamp(
            &format!("{timestamp},-3813477.954,3554276.552,3662785.237\n"),
            expected,
        )?;
    }
    Ok(())
}

#[test]
fn duplicate_and_decreasing_timestamps_are_rejected()
-> Result<(), Box<dyn std::error::Error>> {
    let position = "-3813477.954,3554276.552,3662785.237";
    assert_invalid_timestamp(
        &format!("1.0,{position}\n1.0,{position}\n"),
        "must be greater than the previous timestamp",
    )?;
    assert_invalid_timestamp(
        &format!("2.0,{position}\n1.5,{position}\n"),
        "must be greater than the previous timestamp",
    )?;
    Ok(())
}

#[test]
fn non_finite_timestamp_normalization_is_rejected()
-> Result<(), Box<dyn std::error::Error>> {
    let position = "-3813477.954,3554276.552,3662785.237";
    assert_invalid_timestamp(
        &format!("-1e308,{position}\n1e308,{position}\n"),
        "normalized elapsed timestamp is not finite",
    )?;
    Ok(())
}
