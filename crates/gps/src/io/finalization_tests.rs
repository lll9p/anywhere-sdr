use std::{
    error::Error as StdError,
    io::{self, Write},
};

use super::*;

#[derive(Debug)]
struct FakeWriter {
    bytes: Vec<u8>,
    maximum_write: usize,
    fail_write_after: Option<usize>,
    flush_error: Option<io::ErrorKind>,
    flush_count: usize,
    events: Vec<&'static str>,
}

impl FakeWriter {
    fn new(maximum_write: usize) -> Self {
        Self {
            bytes: Vec::new(),
            maximum_write,
            fail_write_after: None,
            flush_error: None,
            flush_count: 0,
            events: Vec::new(),
        }
    }
}

impl Write for FakeWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let remaining_before_failure = match self.fail_write_after {
            Some(limit) => limit.saturating_sub(self.bytes.len()),
            None => usize::MAX,
        };
        if !buffer.is_empty() && remaining_before_failure == 0 {
            self.events.push("write_error");
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "sentinel write failure",
            ));
        }

        let written = buffer
            .len()
            .min(self.maximum_write)
            .min(remaining_before_failure);
        self.bytes.extend(buffer.iter().take(written).copied());
        self.events.push("write");
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_count += 1;
        self.events.push("flush");
        match self.flush_error {
            Some(kind) => Err(io::Error::new(kind, "sentinel flush failure")),
            None => Ok(()),
        }
    }
}

fn assert_io_kind(error: Error, expected: io::ErrorKind) -> Result<(), Error> {
    match error {
        Error::Io(error) => {
            assert_eq!(error.kind(), expected);
            Ok(())
        }
        error => Err(Error::msg(format!("expected I/O error, got {error:?}"))),
    }
}

#[test]
fn formatted_write_retries_partial_writes() -> Result<(), Error> {
    let mut writer = FakeWriter::new(1);
    let mut state = Bits1PackingState::default();
    write_samples_to(
        &mut writer,
        DataFormat::Bits8,
        &[16, -16, 32, -32],
        2,
        &mut state,
    )?;

    assert_eq!(writer.bytes, vec![1, 255, 2, 254]);
    assert_eq!(writer.events, vec!["write", "write", "write", "write"]);
    Ok(())
}

#[test]
fn formatted_write_preserves_midstream_failure() -> Result<(), Error> {
    let mut writer = FakeWriter::new(1);
    writer.fail_write_after = Some(2);
    let mut state = Bits1PackingState::default();

    let Err(error) = write_samples_to(
        &mut writer,
        DataFormat::Bits8,
        &[16, -16, 32, -32],
        2,
        &mut state,
    ) else {
        return Err(Error::msg(
            "configured writer did not fail after two bytes",
        ));
    };

    assert_eq!(writer.bytes, vec![1, 255]);
    assert_io_kind(error, io::ErrorKind::BrokenPipe)
}

#[test]
fn write_and_finalization_failures_remain_ordered() -> Result<(), Error> {
    let mut writer = FakeWriter::new(1);
    writer.fail_write_after = Some(2);
    let mut state = Bits1PackingState::default();
    let primary = write_samples_to(
        &mut writer,
        DataFormat::Bits8,
        &[16, -16, 32, -32],
        2,
        &mut state,
    );
    writer.flush_error = Some(io::ErrorKind::Other);
    let finalization =
        finish_writer(&mut writer, DataFormat::Bits8, &mut state);

    let Err(error) =
        crate::error::resolve_with_finalization(primary, finalization)
    else {
        return Err(Error::msg("combined write and flush failures were lost"));
    };
    let Some(source) = StdError::source(&error) else {
        return Err(Error::msg("combined error did not expose its primary"));
    };
    assert!(source.to_string().contains("sentinel write failure"));
    let Error::OutputFinalization {
        primary,
        finalization,
    } = error
    else {
        return Err(Error::msg("expected combined output finalization error"));
    };
    assert_io_kind(*primary, io::ErrorKind::BrokenPipe)?;
    assert_io_kind(*finalization, io::ErrorKind::Other)?;
    assert_eq!(writer.flush_count, 1);
    assert_eq!(writer.events, vec![
        "write",
        "write",
        "write_error",
        "flush"
    ]);
    Ok(())
}

#[test]
fn finish_returns_flush_only_failure() -> Result<(), Error> {
    let mut writer = FakeWriter::new(8);
    writer.flush_error = Some(io::ErrorKind::Other);
    let mut state = Bits1PackingState::default();

    let Err(error) = finish_writer(&mut writer, DataFormat::Bits8, &mut state)
    else {
        return Err(Error::msg("configured flush did not fail"));
    };

    assert_eq!(writer.flush_count, 1);
    assert_eq!(writer.events, vec!["flush"]);
    assert_io_kind(error, io::ErrorKind::Other)
}

#[test]
fn finish_retains_packing_and_flush_failures() -> Result<(), Error> {
    let mut writer = FakeWriter::new(8);
    writer.fail_write_after = Some(0);
    writer.flush_error = Some(io::ErrorKind::Other);
    let mut packed = Vec::new();
    let mut state = Bits1PackingState::default().push(&[1], &mut packed);
    assert!(packed.is_empty());

    let Err(error) = finish_writer(&mut writer, DataFormat::Bits1, &mut state)
    else {
        return Err(Error::msg("packing and flush unexpectedly succeeded"));
    };

    let Error::OutputFinalization {
        primary,
        finalization,
    } = error
    else {
        return Err(Error::msg("expected combined output finalization error"));
    };
    assert_io_kind(*primary, io::ErrorKind::BrokenPipe)?;
    assert_io_kind(*finalization, io::ErrorKind::Other)?;
    assert_eq!(state.pending_bits, 1);
    assert_eq!(writer.flush_count, 1);
    assert_eq!(writer.events, vec!["write_error", "flush"]);
    Ok(())
}

#[test]
fn successful_finish_is_padding_idempotent_and_still_flushes()
-> Result<(), Error> {
    let mut writer = FakeWriter::new(8);
    let mut packed = Vec::new();
    let mut state = Bits1PackingState::default().push(&[1, -1, 1], &mut packed);
    assert!(packed.is_empty());

    finish_writer(&mut writer, DataFormat::Bits1, &mut state)?;
    finish_writer(&mut writer, DataFormat::Bits1, &mut state)?;

    assert_eq!(writer.bytes, vec![0b1010_0000]);
    assert_eq!(state.pending_bits, 0);
    assert_eq!(writer.flush_count, 2);
    assert_eq!(writer.events, vec!["write", "flush", "flush"]);
    Ok(())
}
