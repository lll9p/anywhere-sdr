use std::{
    fmt::Write as _,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use geometry::Ecef;
use gps::{
    Error, GpsTime, MotionMode, SignalGenerator, SignalGeneratorBuilder,
    pack_bits8_into,
};

static NEXT_PATH: AtomicU64 = AtomicU64::new(0);
const TEST_FREQUENCY_HZ: f64 = 10_000.0;
const ORIGIN: Ecef = Ecef {
    x: -3_813_477.954,
    y: 3_554_276.552,
    z: 3_662_785.237,
};

fn unique_path(name: &str, extension: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "anywhere-sdr-{name}-{}-{}.{}",
        std::process::id(),
        NEXT_PATH.fetch_add(1, Ordering::Relaxed),
        extension
    ))
}

fn navigation_path() -> PathBuf {
    PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("resources")
        .join("brdc0010.22n")
}

fn position(fraction: f64) -> Ecef {
    Ecef::new(
        ORIGIN.x + 40.0 * fraction,
        ORIGIN.y - 20.0 * fraction,
        ORIGIN.z + 8.0 * fraction,
    )
}

fn write_motion_file(records: &[(f64, Ecef)]) -> Result<PathBuf, Error> {
    let path = unique_path("timestamped-motion", "csv");
    let mut contents = String::new();
    for (timestamp, position) in records {
        writeln!(
            &mut contents,
            "{timestamp},{},{},{}",
            position.x, position.y, position.z
        )
        .map_err(|error| {
            Error::msg(format!("failed to format motion: {error}"))
        })?;
    }
    fs::write(&path, contents)?;
    Ok(path)
}

fn timestamped_generator(
    motion_path: PathBuf, duration_seconds: Option<f64>, step_seconds: f64,
    output_path: Option<PathBuf>,
) -> Result<SignalGenerator, Error> {
    let mut generator = SignalGeneratorBuilder::default()
        .navigation_file(Some(navigation_path()))?
        .user_motion_file(Some(motion_path))?
        .duration(duration_seconds)
        .sample_rate(Some(step_seconds))
        .frequency(Some(1_000_000))?
        .data_format(Some(8))?
        .path_loss(Some(128))
        .verbose(Some(false))
        .output_file(output_path)
        .build()?;
    generator.sample_frequency = TEST_FREQUENCY_HZ;
    Ok(generator)
}

fn reference_generator(
    positions: Vec<Ecef>, duration_seconds: f64, step_seconds: f64,
) -> Result<SignalGenerator, Error> {
    let mut generator = SignalGeneratorBuilder::default()
        .navigation_file(Some(navigation_path()))?
        .location_ecef(Some(vec![ORIGIN.x, ORIGIN.y, ORIGIN.z]))?
        .duration(Some(duration_seconds))
        .sample_rate(Some(step_seconds))
        .frequency(Some(1_000_000))?
        .data_format(Some(8))?
        .path_loss(Some(128))
        .verbose(Some(false))
        .build()?;
    generator.positions = positions;
    generator.mode = MotionMode::Dynamic;
    generator.sample_frequency = TEST_FREQUENCY_HZ;
    Ok(generator)
}

struct Capture {
    block_samples: Vec<usize>,
    samples: Vec<i16>,
    start_time: GpsTime,
    end_time: GpsTime,
}

fn capture(mut generator: SignalGenerator) -> Result<Capture, Error> {
    generator.initialize()?;
    let start_time = generator.receiver_gps_time.clone();
    let mut block_samples = Vec::new();
    let mut samples = Vec::new();
    generator.run_streaming::<_, Error>(|block| {
        block_samples.push(block.len() / 2);
        samples.extend_from_slice(block);
        Ok(())
    })?;
    Ok(Capture {
        block_samples,
        samples,
        start_time,
        end_time: generator.receiver_gps_time,
    })
}

fn set_start_before_frame_deadline(generator: &mut SignalGenerator) {
    let frame_start = (generator.receiver_gps_time.sec / 30.0).floor() * 30.0;
    generator.receiver_gps_time.sec = frame_start + 29.0;
}

fn assert_elapsed(capture: &Capture, expected_seconds: f64) {
    let expected = capture.start_time.add_secs(expected_seconds);
    assert_eq!(capture.end_time.week, expected.week);
    assert!((capture.end_time.sec - expected.sec).abs() < 1.0e-12);
}

#[test]
fn irregular_timestamps_drive_implicit_and_clipped_playback()
-> Result<(), Error> {
    let motion_path = write_motion_file(&[
        (10.0, position(0.0)),
        (10.25, position(0.25)),
        (10.75, position(1.0)),
    ])?;

    let implicit_generator =
        timestamped_generator(motion_path.clone(), None, 0.25, None)?;
    assert_eq!(implicit_generator.duration_seconds, Some(0.75));
    assert_eq!(implicit_generator.simulation_step_count, 3);
    assert_eq!(
        implicit_generator.motion_elapsed_seconds.as_deref(),
        Some([0.0, 0.25, 0.75].as_slice())
    );
    let implicit = capture(implicit_generator)?;
    let implicit_reference = capture(reference_generator(
        vec![
            position(0.0),
            position(0.25),
            position(0.625),
            position(1.0),
        ],
        0.75,
        0.25,
    )?)?;
    assert_eq!(implicit.block_samples, vec![2_500, 2_500, 2_500]);
    assert_eq!(implicit.block_samples, implicit_reference.block_samples);
    assert_eq!(implicit.samples, implicit_reference.samples);
    assert_elapsed(&implicit, 0.75);

    let capped_generator =
        timestamped_generator(motion_path.clone(), Some(10.0), 0.25, None)?;
    assert_eq!(capped_generator.duration_seconds, Some(0.75));
    assert_eq!(capped_generator.simulation_step_count, 3);
    let capped = capture(capped_generator)?;
    assert_eq!(capped.block_samples, implicit.block_samples);
    assert_eq!(capped.samples, implicit.samples);
    assert_elapsed(&capped, 0.75);

    let clipped_generator =
        timestamped_generator(motion_path.clone(), Some(0.5), 0.25, None)?;
    assert_eq!(clipped_generator.duration_seconds, Some(0.5));
    assert_eq!(clipped_generator.simulation_step_count, 2);
    let clipped = capture(clipped_generator)?;
    let clipped_reference = capture(reference_generator(
        vec![position(0.0), position(0.25), position(0.625)],
        0.5,
        0.25,
    )?)?;
    assert_eq!(clipped.block_samples, vec![2_500, 2_500]);
    assert_eq!(clipped.samples, clipped_reference.samples);
    assert_elapsed(&clipped, 0.5);

    fs::remove_file(motion_path)?;
    Ok(())
}

#[test]
fn single_motion_record_is_a_zero_duration_dynamic_epoch() -> Result<(), Error>
{
    let motion_path = write_motion_file(&[(7.5, position(0.0))])?;
    let generator =
        timestamped_generator(motion_path.clone(), None, 0.25, None)?;
    assert!(matches!(generator.mode, MotionMode::Dynamic));
    assert_eq!(generator.duration_seconds, Some(0.0));
    assert_eq!(generator.simulation_step_count, 0);
    assert_eq!(
        generator.motion_elapsed_seconds.as_deref(),
        Some([0.0].as_slice())
    );

    let capture = capture(generator)?;
    assert!(capture.block_samples.is_empty());
    assert!(capture.samples.is_empty());
    assert_elapsed(&capture, 0.0);
    fs::remove_file(motion_path)?;
    Ok(())
}

#[test]
fn interrupted_timestamped_run_resumes_at_the_elapsed_cursor()
-> Result<(), Error> {
    let motion_path = write_motion_file(&[
        (10.0, position(0.0)),
        (10.25, position(0.25)),
        (10.75, position(1.0)),
    ])?;
    let complete = capture(timestamped_generator(
        motion_path.clone(),
        None,
        0.25,
        None,
    )?)?;

    let mut interrupted =
        timestamped_generator(motion_path.clone(), None, 0.25, None)?;
    interrupted.initialize()?;
    let start_time = interrupted.receiver_gps_time.clone();
    let mut resumed_block_samples = Vec::new();
    let mut resumed_samples = Vec::new();
    let result = interrupted.run_streaming::<_, Error>(|block| {
        resumed_block_samples.push(block.len() / 2);
        resumed_samples.extend_from_slice(block);
        Err(Error::msg("intentional timestamped stop"))
    });
    assert!(matches!(result, Err(ref error) if error
        .to_string()
        .contains("intentional timestamped stop")));

    interrupted.run_streaming::<_, Error>(|block| {
        resumed_block_samples.push(block.len() / 2);
        resumed_samples.extend_from_slice(block);
        Ok(())
    })?;
    assert_eq!(resumed_block_samples, complete.block_samples);
    assert_eq!(resumed_samples, complete.samples);
    let first_end = interrupted.receiver_gps_time.clone();
    assert_eq!(first_end, start_time.add_secs(0.75));

    let mut restarted_blocks = Vec::new();
    interrupted.run_streaming::<_, Error>(|block| {
        restarted_blocks.push(block.len() / 2);
        Ok(())
    })?;
    assert_eq!(restarted_blocks, complete.block_samples);
    assert_eq!(interrupted.receiver_gps_time, start_time.add_secs(1.5));

    fs::remove_file(motion_path)?;
    Ok(())
}

#[test]
fn frame_split_uses_timestamp_interpolation_at_each_endpoint()
-> Result<(), Error> {
    let motion_path =
        write_motion_file(&[(20.0, position(0.0)), (22.0, position(1.0))])?;
    let mut timestamped =
        timestamped_generator(motion_path.clone(), None, 2.0, None)?;
    let mut reference = reference_generator(
        vec![position(0.0), position(0.5), position(1.0)],
        2.0,
        1.0,
    )?;
    set_start_before_frame_deadline(&mut timestamped);
    set_start_before_frame_deadline(&mut reference);

    let timestamped = capture(timestamped)?;
    let reference = capture(reference)?;
    assert_eq!(timestamped.block_samples, vec![10_000, 10_000]);
    assert_eq!(timestamped.block_samples, reference.block_samples);
    assert_eq!(timestamped.samples, reference.samples);
    assert_elapsed(&timestamped, 2.0);

    fs::remove_file(motion_path)?;
    Ok(())
}

#[test]
fn timestamped_direct_output_matches_streaming_output() -> Result<(), Error> {
    let motion_path = write_motion_file(&[
        (3.5, position(0.0)),
        (3.75, position(0.25)),
        (4.25, position(1.0)),
    ])?;
    let output_path = unique_path("timestamped-motion-output", "bin");

    let mut direct = timestamped_generator(
        motion_path.clone(),
        None,
        0.25,
        Some(output_path.clone()),
    )?;
    direct.initialize()?;
    direct.run_simulation()?;
    let direct_bytes = fs::read(&output_path)?;

    let mut streaming =
        timestamped_generator(motion_path.clone(), None, 0.25, None)?;
    streaming.initialize()?;
    let mut stream_bytes = Vec::new();
    streaming.run_streaming::<_, Error>(|samples| {
        let mut packed = vec![0_u8; samples.len()];
        pack_bits8_into(samples, &mut packed)?;
        stream_bytes.extend_from_slice(&packed);
        Ok(())
    })?;

    assert_eq!(direct_bytes.len(), 15_000);
    assert_eq!(direct_bytes, stream_bytes);
    fs::remove_file(output_path)?;
    fs::remove_file(motion_path)?;
    Ok(())
}
