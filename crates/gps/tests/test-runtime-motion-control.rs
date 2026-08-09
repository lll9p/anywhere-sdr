use std::path::PathBuf;

use geometry::{Ecef, Location, Neu};
use gps::{Error, MotionCommand, RuntimeMotionControl, SignalGeneratorBuilder};

fn assert_ecef_matches(actual: &Ecef, expected: &Ecef) {
    assert!((actual.x - expected.x).abs() <= f64::EPSILON);
    assert!((actual.y - expected.y).abs() <= f64::EPSILON);
    assert!((actual.z - expected.z).abs() <= f64::EPSILON);
}

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn builder_preserves_geometry_error_for_invalid_degree_location() {
    let result =
        SignalGeneratorBuilder::default().location(Some(vec![91.0, 0.0, 0.0]));
    assert!(matches!(
        result,
        Err(Error::Geometry(geometry::Error::InvalidCoordinates { .. }))
    ));
}

#[test]
fn generator_initialization_propagates_invalid_ecef() -> Result<(), Error> {
    let navigation_path = PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("resources")
        .join("brdc0010.22n");
    let builder = SignalGeneratorBuilder::default()
        .navigation_file(Some(navigation_path))?
        .location_ecef(Some(vec![f64::NAN, 0.0, 0.0]))?
        .data_format(Some(8))?;
    let mut generator = builder.build()?;
    assert!(matches!(
        generator.initialize(),
        Err(Error::Geometry(geometry::Error::InvalidEcef { .. }))
    ));
    Ok(())
}

#[test]
fn runtime_motion_rejects_non_finite_ecef_at_submit() {
    let control = RuntimeMotionControl::new(Ecef::default());
    assert!(matches!(
        control.submit(MotionCommand::SetPositionEcef(Ecef::new(
            f64::NAN,
            0.0,
            0.0,
        ))),
        Err(Error::NonFiniteMotionCommandValue {
            command: "SetPositionEcef",
            field: "x",
            value,
        }) if value.is_nan()
    ));
}

#[test]
fn runtime_motion_defers_finite_invalid_ecef_to_the_next_step()
-> Result<(), Error> {
    let navigation_path = PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("resources")
        .join("brdc0010.22n");
    let origin = Ecef::new(-3_813_477.954, 3_554_276.552, 3_662_785.237);
    let control = RuntimeMotionControl::new(origin);
    let builder = SignalGeneratorBuilder::default()
        .navigation_file(Some(navigation_path))?
        .runtime_motion_control(Some(control.clone()))?
        .frequency(Some(1_000_000))?
        .data_format(Some(8))?
        .output_file(None);
    let mut generator = builder.build()?;
    generator.initialize()?;

    let mut blocks = 0usize;
    let result = generator.run_streaming_user_control::<_, Error>(|_iq| {
        blocks += 1;
        control.submit(MotionCommand::SetPositionEcef(Ecef::default()))?;
        Ok(())
    });

    assert_eq!(blocks, 1);
    assert!(matches!(result, Err(Error::Geometry(_))));
    Ok(())
}

#[test]
fn runtime_streaming_produces_blocks_and_setposition_applies_next_step()
-> Result<(), Error> {
    let workspace_dir = PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
    let nav = workspace_dir.join("resources").join("brdc0010.22n");

    let origin = Ecef::new(-3_813_477.954, 3_554_276.552, 3_662_785.237);
    let control = RuntimeMotionControl::new(origin);

    let builder = SignalGeneratorBuilder::default()
        .navigation_file(Some(nav))?
        .runtime_motion_control(Some(control.clone()))?
        .frequency(Some(1_000_000))?
        .data_format(Some(8))?
        .output_file(None);
    let mut generator = builder.build()?;
    generator.initialize()?;

    let new_pos = Ecef::new(origin.x + 1000.0, origin.y, origin.z);
    let mut positions: Vec<Ecef> = Vec::new();
    let mut blocks: usize = 0;

    let result = generator.run_streaming_user_control::<_, Error>(|_iq| {
        blocks += 1;
        positions.push(control.snapshot().position_ecef);

        if blocks == 2 {
            control.submit(MotionCommand::SetPositionEcef(new_pos))?;
        }

        if blocks >= 3 {
            return Err(Error::msg("stop"));
        }
        Ok(())
    });

    match result {
        Err(e) if e.to_string().contains("stop") => {}
        Ok(()) => {
            return Err(Error::msg("streaming unexpectedly returned Ok(())"));
        }
        Err(e) => return Err(e),
    }

    assert_eq!(positions.len(), 3);
    assert_ecef_matches(&positions[0], &origin);
    assert_ecef_matches(&positions[1], &origin);
    assert_ecef_matches(&positions[2], &new_pos);
    Ok(())
}

#[test]
fn runtime_motion_uses_deadline_split_durations() -> Result<(), Error> {
    let navigation_path = PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("resources")
        .join("brdc0010.22n");
    let origin = Ecef::new(-3_813_477.954, 3_554_276.552, 3_662_785.237);
    let control = RuntimeMotionControl::new(origin);
    control.submit(MotionCommand::SetAccelerationNeu(Neu {
        north: 1.0,
        east: 0.0,
        up: 0.0,
    }))?;
    let builder = SignalGeneratorBuilder::default()
        .navigation_file(Some(navigation_path))?
        .runtime_motion_control(Some(control.clone()))?
        .sample_rate(Some(0.2))
        .frequency(Some(1_000_000))?
        .data_format(Some(8))?
        .output_file(None);
    let mut generator = builder.build()?;
    let frame_start = (generator.receiver_gps_time.sec / 30.0).floor() * 30.0;
    generator.receiver_gps_time.sec = frame_start + 29.9;
    generator.initialize()?;

    let mut block_samples = Vec::new();
    let mut snapshots = Vec::new();
    let result = generator.run_streaming_user_control::<_, Error>(|iq| {
        block_samples.push(iq.len() / 2);
        snapshots.push(control.snapshot());
        if snapshots.len() == 2 {
            Err(Error::msg("deadline split captured"))
        } else {
            Ok(())
        }
    });
    assert!(matches!(result, Err(ref error) if error
        .to_string()
        .contains("deadline split captured")));
    assert_eq!(block_samples, vec![100_000, 100_000]);
    assert_eq!(snapshots.len(), 2);
    assert_close(snapshots[0].speed_mps, 0.1, 1.0e-12);
    assert_close(snapshots[1].speed_mps, 0.2, 1.0e-12);

    let first_delta = snapshots[0].position_ecef - &origin;
    let first_ltcmat = Location::try_from(&origin)?.ltcmat();
    let first_displacement = Neu::from_ecef(&first_delta, first_ltcmat);
    assert_close(first_displacement.north, 0.005, 1.0e-7);

    let second_start = snapshots[0].position_ecef;
    let second_delta = snapshots[1].position_ecef - &second_start;
    let second_ltcmat = Location::try_from(&second_start)?.ltcmat();
    let second_displacement = Neu::from_ecef(&second_delta, second_ltcmat);
    assert_close(second_displacement.north, 0.015, 1.0e-7);
    Ok(())
}
