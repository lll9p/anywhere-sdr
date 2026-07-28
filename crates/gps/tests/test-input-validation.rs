use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use geometry::Ecef;
use gps::{Error, IqBlockSizing, RuntimeMotionControl, SignalGeneratorBuilder};

const MAX_COMPLEX_SAMPLES: usize = 16_777_216;

fn navigation_path() -> PathBuf {
    PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("resources")
        .join("brdc0010.22n")
}

fn motion_path() -> PathBuf {
    PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("resources")
        .join("circle.csv")
}

fn configured_builder() -> Result<SignalGeneratorBuilder, Error> {
    SignalGeneratorBuilder::default()
        .navigation_file(Some(navigation_path()))?
        .location(Some(vec![35.681_298, 139.766_247, 10.0]))?
        .data_format(Some(8))
}

fn expected_error<T>(result: Result<T, Error>) -> Result<Error, Error> {
    match result {
        Ok(_) => Err(Error::msg("expected builder configuration to fail")),
        Err(error) => Ok(error),
    }
}

#[test]
fn static_coordinate_vectors_require_exact_triplets() -> Result<(), Error> {
    for actual in [0, 1, 2, 4] {
        let error = expected_error(
            SignalGeneratorBuilder::default()
                .location_ecef(Some(vec![0.0; actual])),
        )?;
        assert!(matches!(
            error,
            Error::InvalidCoordinateCount { actual: count } if count == actual
        ));

        let error = expected_error(
            SignalGeneratorBuilder::default().location(Some(vec![0.0; actual])),
        )?;
        assert!(matches!(
            error,
            Error::InvalidCoordinateCount { actual: count } if count == actual
        ));
    }

    SignalGeneratorBuilder::default()
        .location_ecef(Some(vec![1.0, 2.0, 3.0]))?;
    SignalGeneratorBuilder::default()
        .location(Some(vec![35.0, 140.0, 10.0]))?;
    Ok(())
}

#[test]
fn exact_triplets_preserve_geometry_validation_errors() {
    let result =
        SignalGeneratorBuilder::default().location(Some(vec![91.0, 0.0, 0.0]));
    assert!(matches!(
        result,
        Err(Error::Geometry(geometry::Error::InvalidCoordinates { .. }))
    ));
}

#[test]
fn duration_domain_and_static_limit_are_validated() -> Result<(), Error> {
    for duration in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = expected_error(
            configured_builder()?.duration(Some(duration)).build(),
        )?;
        assert!(matches!(error, Error::InvalidDuration));
    }

    configured_builder()?.duration(Some(0.0)).build()?;
    configured_builder()?.duration(Some(86_400.0)).build()?;
    let result = configured_builder()?.duration(Some(86_401.0)).build();
    assert!(matches!(result, Err(Error::InvalidDuration)));
    Ok(())
}

#[test]
fn update_step_requires_a_positive_finite_value() -> Result<(), Error> {
    for step in [0.0, -0.1, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let result = configured_builder()?.sample_rate(Some(step)).build();
        assert!(matches!(result, Err(Error::InvalidUpdateStep { .. })));
    }

    let generator = configured_builder()?.sample_rate(Some(0.125)).build()?;
    assert!((generator.sample_rate - 0.125).abs() < f64::EPSILON);

    let mut generator = configured_builder()?
        .sample_rate(Some(f64::MIN_POSITIVE))
        .build()?;
    assert!(matches!(
        generator.initialize(),
        Err(Error::UnsupportedWorkload { .. })
    ));
    assert!(!generator.initialized);
    Ok(())
}

#[test]
fn huge_dynamic_duration_caps_before_interval_conversion() -> Result<(), Error>
{
    let generator = SignalGeneratorBuilder::default()
        .navigation_file(Some(navigation_path()))?
        .user_motion_file(Some(motion_path()))?
        .duration(Some(f64::MAX))
        .sample_rate(Some(0.1))
        .data_format(Some(8))?
        .build()?;

    assert_eq!(
        generator.simulation_step_count,
        generator.positions.len().checked_sub(1).ok_or_else(|| {
            Error::msg("dynamic fixture must contain at least one position")
        })?
    );
    Ok(())
}

#[test]
fn user_control_ignores_a_valid_configured_duration() -> Result<(), Error> {
    let origin = Ecef::new(-3_813_477.954, 3_554_276.552, 3_662_785.237);
    let control = RuntimeMotionControl::new(origin);
    let mut generator = SignalGeneratorBuilder::default()
        .navigation_file(Some(navigation_path()))?
        .runtime_motion_control(Some(control))?
        .duration(Some(f64::MAX))
        .frequency(Some(1_000_000))?
        .data_format(Some(8))?
        .build()?;

    generator.initialize()?;
    assert!(generator.initialized);
    Ok(())
}

#[test]
fn interleaved_iq_block_limit_is_inclusive() -> Result<(), Error> {
    let sizing = IqBlockSizing::new(MAX_COMPLEX_SAMPLES)?;
    assert_eq!(sizing.complex_samples(), MAX_COMPLEX_SAMPLES);
    assert_eq!(sizing.interleaved_i16_len(), 33_554_432);
    assert_eq!(sizing.interleaved_bytes(), 67_108_864);

    let result = IqBlockSizing::new(
        MAX_COMPLEX_SAMPLES
            .checked_add(1)
            .ok_or_else(|| Error::msg("test sample count overflow"))?,
    );
    assert!(matches!(result, Err(Error::UnsupportedWorkload { .. })));
    Ok(())
}

#[test]
fn oversized_initialization_is_rejected_before_state_or_file_mutation()
-> Result<(), Error> {
    static NEXT_PATH: AtomicU64 = AtomicU64::new(0);
    let output_path = std::env::temp_dir().join(format!(
        "anywhere-sdr-oversized-{}-{}.bin",
        std::process::id(),
        NEXT_PATH.fetch_add(1, Ordering::Relaxed)
    ));
    if output_path.exists() {
        std::fs::remove_file(&output_path)?;
    }

    let mut generator = configured_builder()?
        .duration(Some(1.0))
        .sample_rate(Some(16.777_217))
        .frequency(Some(1_000_000))?
        .output_file(Some(output_path.clone()))
        .build()?;
    generator.channels[0].prn = 7;
    generator.allocated_satellite[0] = 3;
    generator.antenna_pattern[0] = 0.25;

    let result = generator.initialize();
    assert!(matches!(result, Err(Error::UnsupportedWorkload { .. })));
    assert_eq!(generator.channels[0].prn, 7);
    assert_eq!(generator.allocated_satellite[0], 3);
    assert!((generator.antenna_pattern[0] - 0.25).abs() < f64::EPSILON);
    assert_eq!(generator.iq_buffer_size, 0);
    assert!(generator.writer.is_none());
    assert!(!generator.initialized);
    assert!(!output_path.exists());
    Ok(())
}

#[test]
fn exact_workload_boundary_initializes_without_allocating_an_output_buffer()
-> Result<(), Error> {
    let mut generator = configured_builder()?
        .duration(Some(0.0))
        .sample_rate(Some(16.777_216))
        .frequency(Some(1_000_000))?
        .output_file(None)
        .build()?;

    generator.initialize()?;
    assert_eq!(generator.iq_buffer_size, MAX_COMPLEX_SAMPLES);
    assert!(generator.writer.is_none());
    Ok(())
}
