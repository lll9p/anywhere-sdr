use std::{
    fs,
    io::{BufWriter, Write},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use gps::{Error, SignalGeneratorBuilder, pack_bits1_into, pack_bits8_into};

fn unique_paths(prefix: &str) -> Result<(PathBuf, PathBuf), Error> {
    let workspace_dir = PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
    let out_dir = workspace_dir.join("output");
    fs::create_dir_all(&out_dir)?;
    let unique = format!(
        "{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| Error::msg(format!("system time error: {error}")))?
            .as_nanos()
    );
    Ok((
        out_dir.join(format!("{prefix}_run_{unique}.bin")),
        out_dir.join(format!("{prefix}_stream_{unique}.bin")),
    ))
}

fn builder(
    duration_seconds: f64, step_seconds: f64, bits: usize,
    output_file: Option<PathBuf>,
) -> Result<SignalGeneratorBuilder, Error> {
    let nav = PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("resources")
        .join("brdc0010.22n");
    SignalGeneratorBuilder::default()
        .navigation_file(Some(nav))?
        .location(Some(vec![35.681_298, 139.766_247, 10.0]))?
        .duration(Some(duration_seconds))
        .sample_rate(Some(step_seconds))
        .frequency(Some(1_000_000))?
        .data_format(Some(bits))
        .map(|builder| builder.output_file(output_file))
}

fn fixed_gain_128_bits8_builder(
    output_file: Option<PathBuf>,
) -> Result<SignalGeneratorBuilder, Error> {
    let builder =
        builder(1.0, 0.1, 8, output_file)?.frequency(Some(2_600_000))?;
    Ok(builder.path_loss(Some(128)).verbose(Some(false)))
}

#[test]
fn fractional_duration_file_and_stream_outputs_match() -> Result<(), Error> {
    let (run_path, stream_path) = unique_paths("rust_stream_equiv_bits8")?;
    let duration_seconds = 0.15;

    {
        let mut generator =
            builder(duration_seconds, 0.1, 8, Some(run_path.clone()))?
                .build()?;
        generator.initialize()?;
        generator.run_simulation()?;
    }

    {
        let mut generator = builder(duration_seconds, 0.1, 8, None)?.build()?;
        generator.initialize()?;

        let file = std::fs::File::create(&stream_path)?;
        let mut writer = BufWriter::new(file);
        let mut packed: Vec<u8> = Vec::new();
        let mut block_samples = Vec::new();

        generator.run_streaming::<_, Error>(|iq| {
            block_samples.push(iq.len() / 2);
            packed.resize(iq.len(), 0);
            pack_bits8_into(iq, &mut packed)?;
            writer.write_all(&packed)?;
            Ok(())
        })?;
        writer.flush()?;
        assert_eq!(block_samples, vec![100_000, 50_000]);
    }

    let run_bytes = fs::read(&run_path)?;
    let stream_bytes = fs::read(&stream_path)?;
    assert_eq!(run_bytes.len(), 300_000);
    assert_eq!(run_bytes, stream_bytes);

    fs::remove_file(&run_path)?;
    fs::remove_file(&stream_path)?;
    Ok(())
}

#[test]
fn fixed_gain_128_bits8_saturates_and_matches_direct_output()
-> Result<(), Error> {
    let (run_path, _) = unique_paths("rust_sc8_saturation_fixed128")?;
    let direct_bytes = {
        let mut generator =
            fixed_gain_128_bits8_builder(Some(run_path.clone()))?.build()?;
        generator.initialize()?;
        generator.run_simulation()?;
        fs::read(&run_path)?
    };

    let mut generator = fixed_gain_128_bits8_builder(None)?.build()?;
    generator.initialize()?;
    let mut helper_bytes = Vec::new();
    let mut independent_bytes = Vec::new();
    let mut negative_clip_count = 0usize;
    let mut positive_clip_count = 0usize;
    generator.run_streaming::<_, Error>(|iq| {
        let mut packed = vec![0u8; iq.len()];
        pack_bits8_into(iq, &mut packed)?;

        let block_start = independent_bytes.len();
        for &sample in iq {
            let expected = if sample <= -2049 {
                negative_clip_count += 1;
                0x80
            } else if sample >= 2048 {
                positive_clip_count += 1;
                0x7f
            } else {
                (sample.div_euclid(16) as i8) as u8
            };
            independent_bytes.push(expected);
        }
        assert_eq!(packed.as_slice(), &independent_bytes[block_start..]);
        helper_bytes.extend_from_slice(&packed);
        Ok(())
    })?;

    assert!(negative_clip_count > 0);
    assert!(positive_clip_count > 0);
    assert_eq!(helper_bytes, independent_bytes);
    assert_eq!(direct_bytes, helper_bytes);

    fs::remove_file(&run_path)?;
    Ok(())
}

fn capture_bits1(
    duration_seconds: f64, step_seconds: f64,
) -> Result<(Vec<usize>, Vec<i16>), Error> {
    let mut generator =
        builder(duration_seconds, step_seconds, 1, None)?.build()?;
    generator.initialize()?;
    let mut block_samples = Vec::new();
    let mut samples = Vec::new();
    generator.run_streaming::<_, Error>(|iq| {
        block_samples.push(iq.len() / 2);
        samples.extend_from_slice(iq);
        Ok(())
    })?;
    Ok((block_samples, samples))
}

fn pack_complete_bits1(iq: &[i16]) -> Result<Vec<u8>, Error> {
    let mut packed = vec![0u8; iq.len().div_ceil(8)];
    pack_bits1_into(iq, &mut packed)?;
    Ok(packed)
}

#[test]
fn bits1_fractional_blocks_match_continuous_one_shot_packing()
-> Result<(), Error> {
    let (run_path, _) = unique_paths("rust_stream_equiv_bits1_fractional")?;
    let mut generator =
        builder(0.1, 1.0 / 30.0, 1, Some(run_path.clone()))?.build()?;
    generator.initialize()?;
    generator.run_simulation()?;
    drop(generator);

    let (block_samples, samples) = capture_bits1(0.1, 1.0 / 30.0)?;
    let expected = pack_complete_bits1(&samples)?;
    let actual = fs::read(&run_path)?;
    assert_eq!(block_samples, vec![33_333, 33_334, 33_333]);
    assert_eq!(samples.len() / 2, 100_000);
    assert_eq!(actual.len(), 25_000);
    assert_eq!(actual, expected);

    fs::remove_file(&run_path)?;
    Ok(())
}

#[test]
fn bits1_partial_final_byte_is_zero_padded() -> Result<(), Error> {
    let (run_path, _) = unique_paths("rust_stream_equiv_bits1_partial")?;
    let mut generator =
        builder(0.100_001, 0.1, 1, Some(run_path.clone()))?.build()?;
    generator.initialize()?;
    generator.run_simulation()?;
    drop(generator);

    let (block_samples, samples) = capture_bits1(0.100_001, 0.1)?;
    let expected = pack_complete_bits1(&samples)?;
    let actual = fs::read(&run_path)?;
    assert_eq!(block_samples, vec![100_000, 1]);
    assert_eq!(samples.len() / 2, 100_001);
    assert_eq!(actual.len(), 25_001);
    assert_eq!(actual, expected);
    assert_eq!(actual.last().copied().unwrap_or_default() & 0b0011_1111, 0);

    fs::remove_file(&run_path)?;
    Ok(())
}
