use std::path::PathBuf;

use gps::{Error, SignalGeneratorBuilder};

#[test]
fn leap_length_insufficient_is_error() -> Result<(), Error> {
    let nav = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("resources")
        .join("brdc0010.22n");

    let result = SignalGeneratorBuilder::default()
        .navigation_file(Some(nav))?
        .leap(Some(vec![1]))
        .build();

    assert!(matches!(result, Err(Error::InvalidLeapSecondParameters)));
    Ok(())
}
