use std::{
    fs,
    io::{BufWriter, Write},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use gps::{Error, SignalGeneratorBuilder, pack_bits8_into};

#[test]
fn streaming_output_matches_run_simulation_bits8() -> Result<(), Error> {
    let workspace_dir = PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
    let nav = workspace_dir.join("resources").join("brdc0010.22n");

    let out_dir = workspace_dir.join("output");
    fs::create_dir_all(&out_dir)?;

    let unique = format!(
        "{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let run_path = out_dir.join(format!("rust_stream_equiv_run_{unique}.bin"));
    let stream_path =
        out_dir.join(format!("rust_stream_equiv_stream_{unique}.bin"));

    let location_tokyo = vec![35.681_298, 139.766_247, 10.0];

    // Path A: existing file-output golden path.
    {
        let builder = SignalGeneratorBuilder::default()
            .navigation_file(Some(nav.clone()))?
            .location(Some(location_tokyo.clone()))?
            .duration(Some(0.2))
            .frequency(Some(1_000_000))?
            .data_format(Some(8))?
            .output_file(Some(run_path.clone()));
        let mut generator = builder.build()?;
        generator.initialize()?;
        generator.run_simulation()?;
    }

    // Path B: streaming + pack with the same Bits8 semantics.
    {
        let builder = SignalGeneratorBuilder::default()
            .navigation_file(Some(nav))?
            .location(Some(location_tokyo))?
            .duration(Some(0.2))
            .frequency(Some(1_000_000))?
            .data_format(Some(8))?
            .output_file(None);
        let mut generator = builder.build()?;
        generator.initialize()?;

        let file = std::fs::File::create(&stream_path)?;
        let mut writer = BufWriter::new(file);
        let mut packed: Vec<u8> = Vec::new();

        generator.run_streaming::<_, Error>(|iq| {
            packed.resize(iq.len(), 0);
            pack_bits8_into(iq, &mut packed)?;
            writer.write_all(&packed)?;
            Ok(())
        })?;
        writer.flush()?;
    }

    let run_bytes = fs::read(&run_path)?;
    let stream_bytes = fs::read(&stream_path)?;
    assert_eq!(
        run_bytes, stream_bytes,
        "Streaming output differs from run_simulation: {run_path:?} vs \
         {stream_path:?}"
    );

    fs::remove_file(&run_path)?;
    fs::remove_file(&stream_path)?;
    Ok(())
}
