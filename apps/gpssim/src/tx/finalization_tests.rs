use std::{
    error::Error as StdError,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use super::*;
use crate::error::resolve_run_and_finish;

#[derive(Clone)]
struct SinkProbe {
    writes: Arc<AtomicUsize>,
    finishes: Arc<AtomicUsize>,
}

impl SinkProbe {
    fn new() -> Self {
        Self {
            writes: Arc::new(AtomicUsize::new(0)),
            finishes: Arc::new(AtomicUsize::new(0)),
        }
    }
}

struct FakeSink {
    backend: &'static str,
    probe: SinkProbe,
    write_failure: Option<&'static str>,
    finish_failure: Option<&'static str>,
}

impl FakeSink {
    fn new(
        backend: &'static str, probe: SinkProbe,
        write_failure: Option<&'static str>,
        finish_failure: Option<&'static str>,
    ) -> Self {
        Self {
            backend,
            probe,
            write_failure,
            finish_failure,
        }
    }
}

impl TxSink for FakeSink {
    fn backend(&self) -> &'static str {
        self.backend
    }

    fn write_block_i16(
        &mut self, _interleaved_iq_i16: &[i16],
    ) -> Result<(), Error> {
        self.probe.writes.fetch_add(1, Ordering::SeqCst);
        match self.write_failure {
            Some(message) => Err(Error::tx_backend_msg(self.backend, message)),
            None => Ok(()),
        }
    }

    fn finish(&mut self) -> Result<(), Error> {
        self.probe.finishes.fetch_add(1, Ordering::SeqCst);
        match self.finish_failure {
            Some(message) => Err(Error::tx_backend_msg(self.backend, message)),
            None => Ok(()),
        }
    }
}

fn backend_name(error: &Error) -> Option<&'static str> {
    match error {
        Error::TxBackendWithSource { backend, .. }
        | Error::TxBackendMsg { backend, .. } => Some(*backend),
        _ => None,
    }
}

fn unique_output_path(prefix: &str) -> Result<PathBuf, Error> {
    let output_directory =
        PathBuf::from(env!("CARGO_WORKSPACE_DIR")).join("output");
    std::fs::create_dir_all(&output_directory)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| Error::msg(format!("system time error: {error}")))?;
    Ok(output_directory.join(format!(
        "{prefix}-{}-{}.bin",
        std::process::id(),
        timestamp.as_nanos()
    )))
}

#[test]
fn write_failure_does_not_eagerly_finish_sinks() -> Result<(), Error> {
    let first = SinkProbe::new();
    let failing = SinkProbe::new();
    let skipped = SinkProbe::new();
    let mut tee = TxTee::new(vec![
        Box::new(FakeSink::new("first", first.clone(), None, None)),
        Box::new(FakeSink::new(
            "failing",
            failing.clone(),
            Some("write failed"),
            None,
        )),
        Box::new(FakeSink::new("skipped", skipped.clone(), None, None)),
    ]);

    let run_result = tee.write_block_i16(&[1, -1]);
    assert!(run_result.is_err());
    assert_eq!(first.writes.load(Ordering::SeqCst), 1);
    assert_eq!(failing.writes.load(Ordering::SeqCst), 1);
    assert_eq!(skipped.writes.load(Ordering::SeqCst), 0);
    for probe in [&first, &failing, &skipped] {
        assert_eq!(probe.finishes.load(Ordering::SeqCst), 0);
    }

    resolve_run_and_finish(run_result, tee.finish()).map_or_else(
        |error| {
            assert_eq!(backend_name(&error), Some("failing"));
            Ok(())
        },
        |()| Err(Error::msg("write failure was unexpectedly discarded")),
    )?;
    for probe in [&first, &failing, &skipped] {
        assert_eq!(probe.finishes.load(Ordering::SeqCst), 1);
    }
    Ok(())
}

#[test]
fn single_finish_failure_is_returned_unchanged() -> Result<(), Error> {
    let successful = SinkProbe::new();
    let failing = SinkProbe::new();
    let mut tee = TxTee::new(vec![
        Box::new(FakeSink::new("successful", successful.clone(), None, None)),
        Box::new(FakeSink::new(
            "failing",
            failing.clone(),
            None,
            Some("finish failed"),
        )),
    ]);

    let Err(error) = tee.finish() else {
        return Err(Error::msg("configured finish failure was discarded"));
    };
    assert_eq!(backend_name(&error), Some("failing"));
    assert_eq!(successful.finishes.load(Ordering::SeqCst), 1);
    assert_eq!(failing.finishes.load(Ordering::SeqCst), 1);
    Ok(())
}

#[test]
fn multiple_finish_failures_are_typed_and_ordered() -> Result<(), Error> {
    let first_probe = SinkProbe::new();
    let second_probe = SinkProbe::new();
    let third_probe = SinkProbe::new();
    let mut tee = TxTee::new(vec![
        Box::new(FakeSink::new(
            "first",
            first_probe.clone(),
            None,
            Some("first finish"),
        )),
        Box::new(FakeSink::new(
            "second",
            second_probe.clone(),
            None,
            Some("second finish"),
        )),
        Box::new(FakeSink::new(
            "third",
            third_probe.clone(),
            None,
            Some("third finish"),
        )),
    ]);

    let Err(error) = tee.finish() else {
        return Err(Error::msg("multiple finish failures were discarded"));
    };
    let display = error.to_string();
    let Some(source) = StdError::source(&error) else {
        return Err(Error::msg(
            "finalization aggregate did not expose its first failure",
        ));
    };
    assert!(source.to_string().contains("first finish"));
    let Error::MultipleFinalizationFailures { first, additional } = error
    else {
        return Err(Error::msg("expected ordered finalization aggregate"));
    };
    assert_eq!(backend_name(&first), Some("first"));
    assert_eq!(
        additional
            .iter()
            .filter_map(backend_name)
            .collect::<Vec<_>>(),
        vec!["second", "third"]
    );
    assert!(display.contains("first finish"));
    assert!(display.contains("second finish"));
    assert!(display.contains("third finish"));
    for probe in [&first_probe, &second_probe, &third_probe] {
        assert_eq!(probe.finishes.load(Ordering::SeqCst), 1);
    }
    Ok(())
}

#[test]
fn run_and_single_finish_failure_remain_typed() -> Result<(), Error> {
    let probe = SinkProbe::new();
    let mut tee = TxTee::new(vec![Box::new(FakeSink::new(
        "combined",
        probe.clone(),
        Some("write failed"),
        Some("finish failed"),
    ))]);

    let run_result = tee.write_block_i16(&[1, -1]);
    let finish_result = tee.finish();
    let Err(error) = resolve_run_and_finish(run_result, finish_result) else {
        return Err(Error::msg("combined failures were discarded"));
    };
    let Error::RunAndFinalizationFailed {
        primary,
        finalization,
    } = error
    else {
        return Err(Error::msg("expected typed run/finalization error"));
    };
    assert_eq!(backend_name(&primary), Some("combined"));
    assert_eq!(backend_name(&finalization), Some("combined"));
    assert_eq!(probe.finishes.load(Ordering::SeqCst), 1);
    Ok(())
}

#[test]
fn run_and_multiple_finish_failures_remain_typed() -> Result<(), Error> {
    let write_probe = SinkProbe::new();
    let finish_probe = SinkProbe::new();
    let mut tee = TxTee::new(vec![
        Box::new(FakeSink::new(
            "write",
            write_probe.clone(),
            Some("write failed"),
            Some("write sink finish failed"),
        )),
        Box::new(FakeSink::new(
            "finish",
            finish_probe.clone(),
            None,
            Some("second finish failed"),
        )),
    ]);

    let run_result = tee.write_block_i16(&[1, -1]);
    let finish_result = tee.finish();
    let Err(error) = resolve_run_and_finish(run_result, finish_result) else {
        return Err(Error::msg("combined failures were discarded"));
    };
    let display = error.to_string();
    let Some(source) = StdError::source(&error) else {
        return Err(Error::msg("combined error did not expose its primary"));
    };
    assert!(source.to_string().contains("write failed"));
    let Error::RunAndFinalizationFailed {
        primary,
        finalization,
    } = error
    else {
        return Err(Error::msg("expected typed run/finalization error"));
    };
    assert_eq!(backend_name(&primary), Some("write"));
    let Error::MultipleFinalizationFailures { first, additional } =
        *finalization
    else {
        return Err(Error::msg("expected typed finalization aggregate"));
    };
    assert_eq!(backend_name(&first), Some("write"));
    assert_eq!(
        additional
            .iter()
            .filter_map(backend_name)
            .collect::<Vec<_>>(),
        vec!["finish"]
    );
    assert!(display.contains("write failed"));
    assert!(display.contains("write sink finish failed"));
    assert!(display.contains("second finish failed"));
    assert_eq!(write_probe.finishes.load(Ordering::SeqCst), 1);
    assert_eq!(finish_probe.finishes.load(Ordering::SeqCst), 1);
    Ok(())
}

#[test]
fn tiny_file_outputs_are_complete_before_sink_drop() -> Result<(), Error> {
    for (format, expected) in [
        (DataFormat::Bits1, vec![0b1000_0000]),
        (DataFormat::Bits8, vec![0, 255]),
        (DataFormat::Bits16, gps::as_bytes_i16(&[1, -1]).to_vec()),
    ] {
        let path = unique_output_path(&format!("gpssim-{format:?}"))?;
        let mut sink = FileTxSink::new(path.clone(), format, 1)?;
        sink.write_block_i16(&[1, -1])?;
        sink.finish()?;

        assert_eq!(std::fs::read(&path)?, expected);

        drop(sink);
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn dev_full_file_sink_reports_flush_failure() -> Result<(), Error> {
    let mut sink =
        FileTxSink::new(PathBuf::from("/dev/full"), DataFormat::Bits8, 1)?;
    sink.write_block_i16(&[1, -1])?;

    let Err(error) = sink.finish() else {
        return Err(Error::msg(
            "/dev/full finalization unexpectedly succeeded",
        ));
    };
    let Error::TxBackendWithSource {
        backend, source, ..
    } = error
    else {
        return Err(Error::msg("expected contextual file backend error"));
    };
    assert_eq!(backend, "file");
    assert!(matches!(
        source.downcast_ref::<gps::Error>(),
        Some(gps::Error::Io(_))
    ));
    Ok(())
}

#[test]
fn null_sink_finishes_successfully() -> Result<(), Error> {
    let mut sink = NullTxSink::new();
    sink.write_block_i16(&[1, -1])?;
    sink.finish()
}
