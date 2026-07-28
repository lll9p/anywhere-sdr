use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use gps::{
    Error, SignalGeneratorBuilder, as_bytes_i16, pack_bits1_into,
    pack_bits8_into,
};

fn output_path(prefix: &str) -> Result<PathBuf, Error> {
    let output_directory =
        PathBuf::from(env!("CARGO_WORKSPACE_DIR")).join("output");
    fs::create_dir_all(&output_directory)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| Error::msg(format!("system time error: {error}")))?;
    Ok(output_directory.join(format!(
        "{prefix}-{}-{}.bin",
        std::process::id(),
        timestamp.as_nanos()
    )))
}

fn builder(
    bits: usize, duration_seconds: f64, output_file: Option<PathBuf>,
) -> Result<SignalGeneratorBuilder, Error> {
    let navigation_file = PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("resources/brdc0010.22n");
    SignalGeneratorBuilder::default()
        .navigation_file(Some(navigation_file))?
        .location(Some(vec![35.681_298, 139.766_247, 10.0]))?
        .duration(Some(duration_seconds))
        .sample_rate(Some(0.1))
        .frequency(Some(1_000_000))?
        .data_format(Some(bits))
        .map(|builder| builder.output_file(output_file).verbose(Some(false)))
}

fn expected_tiny_output(bits: usize) -> Result<Vec<u8>, Error> {
    let mut generator = builder(bits, 0.000_001, None)?.build()?;
    generator.initialize()?;
    let mut samples = Vec::new();
    generator.run_streaming::<_, Error>(|block| {
        samples.extend_from_slice(block);
        Ok(())
    })?;

    match bits {
        1 => {
            let mut bytes = vec![0; samples.len().div_ceil(8)];
            pack_bits1_into(&samples, &mut bytes)?;
            Ok(bytes)
        }
        8 => {
            let mut bytes = vec![0; samples.len()];
            pack_bits8_into(&samples, &mut bytes)?;
            Ok(bytes)
        }
        16 => Ok(as_bytes_i16(&samples).to_vec()),
        _ => Err(Error::invalid_data_format()),
    }
}

#[test]
fn tiny_direct_outputs_are_complete_before_generator_drop() -> Result<(), Error>
{
    for bits in [1, 8, 16] {
        let path = output_path(&format!("gps-finalization-bits{bits}"))?;
        let expected = expected_tiny_output(bits)?;
        let mut generator =
            builder(bits, 0.000_001, Some(path.clone()))?.build()?;
        generator.initialize()?;
        generator.run_simulation()?;

        assert_eq!(fs::read(&path)?, expected);

        drop(generator);
        fs::remove_file(path)?;
    }
    Ok(())
}

#[test]
fn zero_sample_direct_run_finalizes_pending_output() -> Result<(), Error> {
    let path = output_path("gps-finalization-zero")?;
    let mut generator = builder(1, 0.0, Some(path.clone()))?.build()?;
    generator.initialize()?;
    let writer = generator
        .writer
        .as_mut()
        .ok_or(Error::IQWriterNotInitialized)?;
    writer.buffer_size = 1;
    writer.buffer = vec![1, -1];
    writer.write_samples()?;

    generator.run_simulation()?;

    assert_eq!(fs::read(&path)?, vec![0b1000_0000]);

    drop(generator);
    fs::remove_file(path)?;
    Ok(())
}

#[test]
fn missing_direct_writer_does_not_start_or_advance_the_run() -> Result<(), Error>
{
    let path = output_path("gps-finalization-missing-writer")?;
    let expected = expected_tiny_output(8)?;
    let mut generator = builder(8, 0.000_001, Some(path.clone()))?.build()?;
    generator.initialize()?;
    let writer = generator
        .writer
        .take()
        .ok_or(Error::IQWriterNotInitialized)?;

    assert!(matches!(
        generator.run_simulation(),
        Err(Error::IQWriterNotInitialized)
    ));

    generator.writer = Some(writer);
    generator.run_simulation()?;
    assert_eq!(fs::read(&path)?, expected);

    drop(generator);
    fs::remove_file(path)?;
    Ok(())
}

#[test]
fn primary_run_failure_still_flushes_and_blocks_direct_reuse()
-> Result<(), Error> {
    let path = output_path("gps-finalization-primary-error")?;
    let mut generator = builder(8, 0.000_001, Some(path.clone()))?.build()?;
    generator.initialize()?;
    let writer = generator
        .writer
        .as_mut()
        .ok_or(Error::IQWriterNotInitialized)?;
    writer.buffer_size = 1;
    writer.buffer = vec![16, -16];
    writer.write_samples()?;
    generator.positions.clear();

    let Err(error) = generator.run_simulation() else {
        return Err(Error::msg("run with no receiver position succeeded"));
    };
    assert!(matches!(error, Error::WrongPositions));
    assert_eq!(fs::read(&path)?, vec![1, 255]);
    assert!(matches!(
        generator.run_simulation(),
        Err(Error::FiniteRunOutputFailed)
    ));

    drop(generator);
    fs::remove_file(path)?;
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn primary_and_flush_failures_remain_typed_in_direct_run() -> Result<(), Error>
{
    let mut generator =
        builder(8, 0.000_001, Some(PathBuf::from("/dev/full")))?.build()?;
    generator.initialize()?;
    let writer = generator
        .writer
        .as_mut()
        .ok_or(Error::IQWriterNotInitialized)?;
    writer.buffer_size = 1;
    writer.buffer = vec![16, -16];
    writer.write_samples()?;
    generator.positions.clear();

    let Err(error) = generator.run_simulation() else {
        return Err(Error::msg("combined direct failures were discarded"));
    };
    let Error::OutputFinalization {
        primary,
        finalization,
    } = error
    else {
        return Err(Error::msg("expected typed direct finalization error"));
    };
    assert!(matches!(*primary, Error::WrongPositions));
    assert!(matches!(*finalization, Error::Io(_)));
    assert!(matches!(
        generator.run_simulation(),
        Err(Error::FiniteRunOutputFailed)
    ));
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn dev_full_write_failure_blocks_direct_reuse() -> Result<(), Error> {
    let mut generator =
        builder(16, 0.01, Some(PathBuf::from("/dev/full")))?.build()?;
    generator.initialize()?;

    assert!(matches!(generator.run_simulation(), Err(Error::Io(_))));
    assert!(matches!(
        generator.run_simulation(),
        Err(Error::FiniteRunOutputFailed)
    ));
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn dev_full_flush_failure_is_reported_and_requires_reinitialization()
-> Result<(), Error> {
    let mut generator =
        builder(8, 0.000_001, Some(PathBuf::from("/dev/full")))?.build()?;
    generator.initialize()?;

    assert!(matches!(generator.run_simulation(), Err(Error::Io(_))));
    assert!(matches!(
        generator.run_simulation(),
        Err(Error::FiniteRunOutputFailed)
    ));

    generator.initialize()?;
    assert!(matches!(generator.run_simulation(), Err(Error::Io(_))));
    Ok(())
}
