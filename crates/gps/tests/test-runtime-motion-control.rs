use std::path::PathBuf;

use geometry::Ecef;
use gps::{Error, MotionCommand, RuntimeMotionControl, SignalGeneratorBuilder};

fn assert_ecef_matches(actual: &Ecef, expected: &Ecef) {
    assert!((actual.x - expected.x).abs() <= f64::EPSILON);
    assert!((actual.y - expected.y).abs() <= f64::EPSILON);
    assert!((actual.z - expected.z).abs() <= f64::EPSILON);
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
            control.submit(MotionCommand::SetPositionEcef(new_pos));
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
