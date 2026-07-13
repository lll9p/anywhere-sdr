use std::path::PathBuf;

use geometry::Ecef;
use gps::{Error, GpsTime, MotionMode, SignalGeneratorBuilder};

fn build_generator(
    duration_seconds: f64, step_seconds: f64,
) -> Result<gps::SignalGenerator, Error> {
    let navigation_path = PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("resources")
        .join("brdc0010.22n");
    SignalGeneratorBuilder::default()
        .navigation_file(Some(navigation_path))?
        .location(Some(vec![35.681_298, 139.766_247, 10.0]))?
        .duration(Some(duration_seconds))
        .sample_rate(Some(step_seconds))
        .frequency(Some(1_000_000))?
        .data_format(Some(8))?
        .output_file(None)
        .verbose(Some(false))
        .build()
}

fn collect_block_samples(
    duration_seconds: f64, step_seconds: f64,
) -> Result<(Vec<usize>, GpsTime, GpsTime), Error> {
    let mut generator = build_generator(duration_seconds, step_seconds)?;
    generator.initialize()?;
    let start_time = generator.receiver_gps_time.clone();
    let mut block_samples = Vec::new();
    generator.run_streaming::<_, Error>(|block| {
        block_samples.push(block.len() / 2);
        Ok(())
    })?;
    Ok((
        block_samples,
        start_time,
        generator.receiver_gps_time.clone(),
    ))
}

fn assert_elapsed(
    start_time: &GpsTime, end_time: &GpsTime, expected_seconds: f64,
) {
    let expected_time = start_time.add_secs(expected_seconds);
    assert_eq!(end_time.week, expected_time.week);
    assert!(
        (end_time.sec - expected_time.sec).abs() < 1.0e-12,
        "expected {expected_time:?}, got {end_time:?}"
    );
}

#[test]
fn duration_is_exact_emitted_waveform_time() -> Result<(), Error> {
    let cases = [
        (0.0, Vec::new()),
        (0.1, vec![100_000]),
        (0.2, vec![100_000, 100_000]),
        (0.15, vec![100_000, 50_000]),
    ];

    for (duration_seconds, expected_blocks) in cases {
        let (blocks, start_time, end_time) =
            collect_block_samples(duration_seconds, 0.1)?;
        assert_eq!(blocks, expected_blocks);
        assert_eq!(
            blocks.iter().sum::<usize>(),
            (duration_seconds * 1_000_000.0) as usize
        );
        assert_elapsed(&start_time, &end_time, duration_seconds);
    }
    Ok(())
}

#[test]
fn custom_steps_share_the_sample_timeline() -> Result<(), Error> {
    let cases = [
        (0.05, 0.2, vec![50_000; 4]),
        (0.1, 0.2, vec![100_000; 2]),
        (0.2, 0.4, vec![200_000; 2]),
        (1.0 / 30.0, 0.1, vec![33_333, 33_334, 33_333]),
    ];

    for (step_seconds, duration_seconds, expected_blocks) in cases {
        let (blocks, start_time, end_time) =
            collect_block_samples(duration_seconds, step_seconds)?;
        assert_eq!(blocks, expected_blocks);
        assert_eq!(
            blocks.iter().sum::<usize>(),
            (duration_seconds * 1_000_000.0).round() as usize
        );
        assert_elapsed(&start_time, &end_time, duration_seconds);
    }
    Ok(())
}

struct StreamCapture {
    block_samples: Vec<usize>,
    samples: Vec<i16>,
    start_time: GpsTime,
    end_time: GpsTime,
}

fn build_dynamic_generator(
    step_seconds: f64, duration_seconds: f64, position_fractions: &[f64],
    start_before_deadline: bool,
) -> Result<gps::SignalGenerator, Error> {
    let mut generator = build_generator(duration_seconds, step_seconds)?;
    let initial = generator
        .positions
        .first()
        .copied()
        .ok_or_else(Error::wrong_positions)?;
    generator.positions = position_fractions
        .iter()
        .map(|fraction| {
            Ecef::new(
                initial.x + 20.0 * fraction,
                initial.y - 10.0 * fraction,
                initial.z + 4.0 * fraction,
            )
        })
        .collect();
    generator.mode = MotionMode::Dynamic;
    generator.sample_frequency = 10_000.0;
    generator.fixed_gain = Some(128);
    let frame_start = (generator.receiver_gps_time.sec / 30.0).floor() * 30.0;
    generator.receiver_gps_time.sec =
        frame_start + if start_before_deadline { 29.0 } else { 1.0 };
    Ok(generator)
}

fn capture_stream(
    mut generator: gps::SignalGenerator,
) -> Result<StreamCapture, Error> {
    generator.initialize()?;
    let start_time = generator.receiver_gps_time.clone();
    let mut block_samples = Vec::new();
    let mut samples = Vec::new();
    generator.run_streaming::<_, Error>(|block| {
        block_samples.push(block.len() / 2);
        samples.extend_from_slice(block);
        Ok(())
    })?;
    Ok(StreamCapture {
        block_samples,
        samples,
        start_time,
        end_time: generator.receiver_gps_time,
    })
}

#[test]
fn dynamic_deadline_partition_matches_an_explicit_midpoint() -> Result<(), Error>
{
    let split =
        capture_stream(build_dynamic_generator(2.0, 2.0, &[0.0, 1.0], true)?)?;
    let midpoint = capture_stream(build_dynamic_generator(
        1.0,
        2.0,
        &[0.0, 0.5, 1.0],
        true,
    )?)?;

    assert_eq!(split.block_samples, vec![10_000, 10_000]);
    assert_eq!(midpoint.block_samples, split.block_samples);
    assert_eq!(split.samples.len() / 2, 20_000);
    assert_eq!(midpoint.samples, split.samples);
    assert_elapsed(&split.start_time, &split.end_time, 2.0);
    assert_eq!(midpoint.end_time, split.end_time);
    Ok(())
}

#[test]
fn partial_dynamic_step_uses_its_true_interpolated_endpoint()
-> Result<(), Error> {
    let clipped =
        capture_stream(build_dynamic_generator(2.0, 1.0, &[0.0, 1.0], false)?)?;
    let midpoint =
        capture_stream(build_dynamic_generator(1.0, 1.0, &[0.0, 0.5], false)?)?;

    assert_eq!(clipped.block_samples, vec![10_000]);
    assert_eq!(midpoint.block_samples, clipped.block_samples);
    assert_eq!(midpoint.samples, clipped.samples);
    assert_elapsed(&clipped.start_time, &clipped.end_time, 1.0);
    assert_eq!(midpoint.end_time, clipped.end_time);
    Ok(())
}

#[test]
fn completed_finite_runs_restart_from_the_current_gps_time() -> Result<(), Error>
{
    let mut generator = build_generator(0.2, 0.1)?;
    generator.initialize()?;
    let start_time = generator.receiver_gps_time.clone();

    let mut first_blocks = Vec::new();
    generator.run_streaming::<_, Error>(|block| {
        first_blocks.push(block.len() / 2);
        Ok(())
    })?;
    let first_end = generator.receiver_gps_time.clone();

    let mut second_blocks = Vec::new();
    generator.run_streaming::<_, Error>(|block| {
        second_blocks.push(block.len() / 2);
        Ok(())
    })?;

    assert_eq!(first_blocks, vec![100_000, 100_000]);
    assert_eq!(second_blocks, first_blocks);
    assert_elapsed(&start_time, &first_end, 0.2);
    assert_elapsed(&start_time, &generator.receiver_gps_time, 0.4);
    Ok(())
}

#[test]
fn interrupted_finite_run_resumes_without_replaying_consumed_blocks()
-> Result<(), Error> {
    let mut generator = build_generator(0.3, 0.1)?;
    generator.initialize()?;
    let start_time = generator.receiver_gps_time.clone();

    let result = generator.run_streaming::<_, Error>(|_| {
        Err(Error::msg("intentional callback stop"))
    });
    assert!(matches!(result, Err(ref error) if error
        .to_string()
        .contains("intentional callback stop")));

    let mut remaining_blocks = Vec::new();
    generator.run_streaming::<_, Error>(|block| {
        remaining_blocks.push(block.len() / 2);
        Ok(())
    })?;
    assert_eq!(remaining_blocks, vec![100_000, 100_000]);
    assert_elapsed(&start_time, &generator.receiver_gps_time, 0.3);
    Ok(())
}

#[test]
fn final_block_callback_failure_requires_reinitialization() -> Result<(), Error>
{
    let mut generator = build_generator(0.2, 0.1)?;
    generator.initialize()?;
    let start_time = generator.receiver_gps_time.clone();
    let mut interrupted_blocks = 0usize;

    let result = generator.run_streaming::<_, Error>(|_| {
        interrupted_blocks += 1;
        if interrupted_blocks == 2 {
            Err(Error::msg("reject final block"))
        } else {
            Ok(())
        }
    });
    assert!(matches!(result, Err(ref error) if error
        .to_string()
        .contains("reject final block")));
    assert_eq!(interrupted_blocks, 2);
    assert_elapsed(&start_time, &generator.receiver_gps_time, 0.2);

    let mut replayed_blocks = 0usize;
    let retry = generator.run_streaming::<_, Error>(|_| {
        replayed_blocks += 1;
        Ok(())
    });
    assert!(matches!(retry, Err(Error::FiniteRunInterruptedAtEnd)));
    assert_eq!(replayed_blocks, 0);
    Ok(())
}

#[test]
fn zero_duration_runs_complete_and_restart_without_callbacks()
-> Result<(), Error> {
    let mut generator = build_generator(0.0, 0.1)?;
    generator.initialize()?;
    let start_time = generator.receiver_gps_time.clone();
    let mut callbacks = 0usize;

    generator.run_streaming::<_, Error>(|_| {
        callbacks += 1;
        Ok(())
    })?;
    generator.run_streaming::<_, Error>(|_| {
        callbacks += 1;
        Ok(())
    })?;

    assert_eq!(callbacks, 0);
    assert_eq!(generator.receiver_gps_time, start_time);
    Ok(())
}
