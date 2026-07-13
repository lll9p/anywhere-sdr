//! Transmission backends for real-time output.

use std::path::PathBuf;

use gps::{DataFormat, IQWriter};

use crate::Error;

/// `HackRF` transmission backend.
mod hackrf;

pub use hackrf::{HackrfTxConfig, HackrfTxSink};

/// A transmission backend that consumes generator-produced interleaved I/Q
/// blocks.
pub trait TxSink {
    /// Backend identifier used for logging and error context.
    fn backend(&self) -> &'static str;

    /// Writes one interleaved I/Q block produced by the generator.
    fn write_block_i16(
        &mut self, interleaved_iq_i16: &[i16],
    ) -> Result<(), Error>;

    /// Finalizes the backend and releases resources.
    fn finish(&mut self) -> Result<(), Error>;
}

/// A sink that fans out each block to multiple underlying sinks.
pub struct TxTee {
    /// Sinks to dispatch each block to.
    sinks: Vec<Box<dyn TxSink>>,
}

impl TxTee {
    /// Creates a new tee sink.
    pub fn new(sinks: Vec<Box<dyn TxSink>>) -> Self {
        Self { sinks }
    }

    /// Returns `true` if no sinks are configured.
    pub fn is_empty(&self) -> bool {
        self.sinks.is_empty()
    }
}

impl TxSink for TxTee {
    fn backend(&self) -> &'static str {
        "tee"
    }

    fn write_block_i16(
        &mut self, interleaved_iq_i16: &[i16],
    ) -> Result<(), Error> {
        for sink in &mut self.sinks {
            if let Err(err) = sink.write_block_i16(interleaved_iq_i16) {
                for other in &mut self.sinks {
                    if let Err(finish_err) = other.finish() {
                        tracing::warn!(
                            backend = other.backend(),
                            error = %finish_err,
                            "tx backend finish failed after upstream error"
                        );
                    }
                }
                return Err(err);
            }
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<(), Error> {
        let mut first_error: Option<Error> = None;
        for sink in &mut self.sinks {
            if let Err(err) = sink.finish() {
                if first_error.is_none() {
                    first_error = Some(err);
                } else {
                    tracing::warn!(
                        backend = sink.backend(),
                        error = %err,
                        "tx backend finish failed"
                    );
                }
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

/// A sink that writes I/Q samples to a file via `gps::IQWriter`.
pub struct FileTxSink {
    /// Output file path.
    path: PathBuf,
    /// Writer configured to match the legacy file output path.
    writer: IQWriter,
}

/// A sink that discards blocks but touches a few samples to keep generation
/// from being optimized away in CPU-only benchmark runs.
pub struct NullTxSink {
    /// Number of generator-produced blocks observed.
    blocks: u64,
    /// Best-effort accumulator used to keep the benchmark loop from being
    /// optimized away.
    checksum: i64,
}

impl NullTxSink {
    /// Creates a new null sink.
    pub fn new() -> Self {
        Self {
            blocks: 0,
            checksum: 0,
        }
    }
}

impl TxSink for NullTxSink {
    fn backend(&self) -> &'static str {
        "null"
    }

    fn write_block_i16(
        &mut self, interleaved_iq_i16: &[i16],
    ) -> Result<(), Error> {
        self.blocks = self.blocks.wrapping_add(1);

        let mut sample: i64 = 0;
        if !interleaved_iq_i16.is_empty() {
            sample ^= i64::from(interleaved_iq_i16[0]);
            sample ^=
                i64::from(interleaved_iq_i16[interleaved_iq_i16.len() / 2]);
            sample ^=
                i64::from(interleaved_iq_i16[interleaved_iq_i16.len() - 1]);
        }
        self.checksum = self.checksum.wrapping_add(sample);
        std::hint::black_box(self.checksum);

        Ok(())
    }

    fn finish(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

impl FileTxSink {
    /// Creates a file sink writing blocks with the given format and block size.
    pub fn new(
        path: PathBuf, format: DataFormat, buffer_size_samples: usize,
    ) -> Result<Self, Error> {
        let writer = IQWriter::new(&path, format, buffer_size_samples)
            .map_err(|err| {
                Error::tx_backend_with_source(
                    "file",
                    format!(
                        "path={} format={:?} buffer_size_samples={}",
                        path.display(),
                        format,
                        buffer_size_samples
                    ),
                    err,
                )
            })?;
        Ok(Self { path, writer })
    }
}

impl TxSink for FileTxSink {
    fn backend(&self) -> &'static str {
        "file"
    }

    fn write_block_i16(
        &mut self, interleaved_iq_i16: &[i16],
    ) -> Result<(), Error> {
        if !interleaved_iq_i16.len().is_multiple_of(2) {
            return Err(Error::tx_backend_msg(
                self.backend(),
                format!(
                    "IQ block length must be even: got {} i16 (path={})",
                    interleaved_iq_i16.len(),
                    self.path.display(),
                ),
            ));
        }

        self.writer.buffer_size = interleaved_iq_i16.len() / 2;
        self.writer.buffer.resize(interleaved_iq_i16.len(), 0);
        self.writer.buffer.copy_from_slice(interleaved_iq_i16);
        self.writer.write_samples().map_err(|err| {
            Error::tx_backend_with_source(
                self.backend(),
                format!("path={}", self.path.display()),
                err,
            )
        })
    }

    fn finish(&mut self) -> Result<(), Error> {
        self.writer.finish_packing().map_err(|err| {
            Error::tx_backend_with_source(
                self.backend(),
                format!("path={}", self.path.display()),
                err,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
        time::{SystemTime, UNIX_EPOCH},
    };

    use gps::{MotionMode, SignalGeneratorBuilder};

    use super::*;

    fn unique_output_path(prefix: &str) -> Result<PathBuf, Error> {
        let workspace_dir = PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
        let output_dir = workspace_dir.join("output");
        std::fs::create_dir_all(&output_dir)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| Error::msg(format!("system time error: {err}")))?;
        Ok(output_dir.join(format!("{prefix}_{}.bin", now.as_nanos())))
    }

    struct MockSink {
        backend: &'static str,
        writes: Arc<AtomicUsize>,
        finished: Arc<AtomicBool>,
        fail_on_write: bool,
    }

    impl TxSink for MockSink {
        fn backend(&self) -> &'static str {
            self.backend
        }

        fn write_block_i16(
            &mut self, _interleaved_iq_i16: &[i16],
        ) -> Result<(), Error> {
            self.writes.fetch_add(1, Ordering::SeqCst);
            if self.fail_on_write {
                Err(Error::tx_backend_msg(self.backend, "intentional failure"))
            } else {
                Ok(())
            }
        }

        fn finish(&mut self) -> Result<(), Error> {
            self.finished.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn tee_dispatches_to_all_sinks() -> Result<(), Error> {
        let writes_a = Arc::new(AtomicUsize::new(0));
        let writes_b = Arc::new(AtomicUsize::new(0));

        let mut tee = TxTee::new(vec![
            Box::new(MockSink {
                backend: "a",
                writes: writes_a.clone(),
                finished: Arc::new(AtomicBool::new(false)),
                fail_on_write: false,
            }),
            Box::new(MockSink {
                backend: "b",
                writes: writes_b.clone(),
                finished: Arc::new(AtomicBool::new(false)),
                fail_on_write: false,
            }),
        ]);

        let block: [i16; 4] = [1, -1, 2, -2];
        tee.write_block_i16(&block)?;
        tee.write_block_i16(&block)?;

        assert_eq!(writes_a.load(Ordering::SeqCst), 2);
        assert_eq!(writes_b.load(Ordering::SeqCst), 2);
        Ok(())
    }

    #[test]
    fn tee_finishes_other_sinks_on_error() -> Result<(), Error> {
        let finished_ok = Arc::new(AtomicBool::new(false));
        let writes_ok = Arc::new(AtomicUsize::new(0));

        let mut tee = TxTee::new(vec![
            Box::new(MockSink {
                backend: "ok",
                writes: writes_ok,
                finished: finished_ok.clone(),
                fail_on_write: false,
            }),
            Box::new(MockSink {
                backend: "fail",
                writes: Arc::new(AtomicUsize::new(0)),
                finished: Arc::new(AtomicBool::new(false)),
                fail_on_write: true,
            }),
        ]);

        let block: [i16; 2] = [1, -1];
        let err = match tee.write_block_i16(&block) {
            Ok(()) => {
                return Err(Error::msg(
                    "expected tee write to fail for the failing sink",
                ));
            }
            Err(err) => err,
        };
        assert!(finished_ok.load(Ordering::SeqCst));

        let error_string = err.to_string();
        assert!(error_string.contains("fail"));
        Ok(())
    }

    #[test]
    fn tee_file_sink_output_matches_golden_run_simulation() -> Result<(), Error>
    {
        let resources_dir =
            PathBuf::from(env!("CARGO_WORKSPACE_DIR")).join("resources");
        let nav = resources_dir.join("brdc0010.22n");

        let golden_path = unique_output_path("gpssim_golden")?;
        let tee_path = unique_output_path("gpssim_tee")?;

        let duration_seconds = 0.15;
        let sample_frequency_hz = 1_000_000;
        let bits = 8;

        // Golden path: generator writes the file directly.
        let builder_a = SignalGeneratorBuilder::default()
            .navigation_file(Some(nav.clone()))?
            .location(Some(vec![35.681_298, 139.766_247, 100.0]))?
            .duration(Some(duration_seconds))
            .frequency(Some(sample_frequency_hz))?
            .data_format(Some(bits))?
            .output_file(Some(golden_path.clone()))
            .verbose(Some(false));
        let mut gen_a = builder_a.build()?;
        gen_a.initialize()?;
        gen_a.run_simulation()?;

        // Streaming path: generator streams blocks to a tee that includes a
        // file sink and a mock sink.
        let builder_b = SignalGeneratorBuilder::default()
            .navigation_file(Some(nav))?
            .location(Some(vec![35.681_298, 139.766_247, 100.0]))?
            .duration(Some(duration_seconds))
            .frequency(Some(sample_frequency_hz))?
            .data_format(Some(bits))?
            .output_file(None)
            .verbose(Some(false));
        let mut gen_b = builder_b.build()?;
        gen_b.initialize()?;

        let expected_blocks = match gen_b.mode {
            MotionMode::Static | MotionMode::Dynamic => {
                gen_b.simulation_step_count
            }
            MotionMode::UserControl => {
                return Err(Error::msg(
                    "unexpected user-control mode in tee golden-path test",
                ));
            }
        };

        let writes_mock = Arc::new(AtomicUsize::new(0));
        let mut tee = TxTee::new(vec![
            Box::new(FileTxSink::new(
                tee_path.clone(),
                DataFormat::Bits8,
                gen_b.iq_buffer_size,
            )?),
            Box::new(MockSink {
                backend: "mock",
                writes: writes_mock.clone(),
                finished: Arc::new(AtomicBool::new(false)),
                fail_on_write: false,
            }),
        ]);

        gen_b.run_streaming::<_, Error>(|block| tee.write_block_i16(block))?;
        tee.finish()?;
        drop(tee);

        let golden_bytes = std::fs::read(&golden_path)?;
        let tee_bytes = std::fs::read(&tee_path)?;
        assert_eq!(tee_bytes, golden_bytes);
        assert_eq!(writes_mock.load(Ordering::SeqCst), expected_blocks);

        std::fs::remove_file(&golden_path)?;
        std::fs::remove_file(&tee_path)?;
        Ok(())
    }

    #[test]
    fn bits1_file_sink_matches_run_simulation_with_final_padding()
    -> Result<(), Error> {
        let resources_dir =
            PathBuf::from(env!("CARGO_WORKSPACE_DIR")).join("resources");
        let nav = resources_dir.join("brdc0010.22n");
        let golden_path = unique_output_path("gpssim_bits1_golden")?;
        let stream_path = unique_output_path("gpssim_bits1_stream")?;

        let mut golden = SignalGeneratorBuilder::default()
            .navigation_file(Some(nav.clone()))?
            .location(Some(vec![35.681_298, 139.766_247, 100.0]))?
            .duration(Some(0.100_001))
            .sample_rate(Some(0.1))
            .frequency(Some(1_000_000))?
            .data_format(Some(1))?
            .output_file(Some(golden_path.clone()))
            .verbose(Some(false))
            .build()?;
        golden.initialize()?;
        golden.run_simulation()?;
        drop(golden);

        let mut streaming = SignalGeneratorBuilder::default()
            .navigation_file(Some(nav))?
            .location(Some(vec![35.681_298, 139.766_247, 100.0]))?
            .duration(Some(0.100_001))
            .sample_rate(Some(0.1))
            .frequency(Some(1_000_000))?
            .data_format(Some(1))?
            .output_file(None)
            .verbose(Some(false))
            .build()?;
        streaming.initialize()?;
        let mut sink = FileTxSink::new(
            stream_path.clone(),
            DataFormat::Bits1,
            streaming.iq_buffer_size,
        )?;
        streaming
            .run_streaming::<_, Error>(|block| sink.write_block_i16(block))?;
        sink.finish()?;
        drop(sink);

        let golden_bytes = std::fs::read(&golden_path)?;
        let stream_bytes = std::fs::read(&stream_path)?;
        assert_eq!(golden_bytes.len(), 25_001);
        assert_eq!(stream_bytes, golden_bytes);
        assert_eq!(
            stream_bytes.last().copied().unwrap_or_default() & 0b0011_1111,
            0
        );

        std::fs::remove_file(&golden_path)?;
        std::fs::remove_file(&stream_path)?;
        Ok(())
    }
}
