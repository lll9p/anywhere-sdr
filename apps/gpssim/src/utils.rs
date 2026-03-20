//! Utility functions for the gpssim application.
//!
//! This module provides helper functions for logging, diagnostics, and other
//! utility operations needed by the application.

use std::{
    collections::VecDeque,
    io,
    io::Write,
    sync::{Arc, Mutex},
};

use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Debug)]
pub(crate) struct LogBuffer {
    inner: Arc<Mutex<LogBufferInner>>,
}

#[derive(Debug)]
struct LogBufferInner {
    lines: VecDeque<String>,
    capacity: usize,
}

impl LogBuffer {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(LogBufferInner {
                lines: VecDeque::with_capacity(capacity.min(64)),
                capacity,
            })),
        }
    }

    pub(crate) fn push_line(&self, line: impl Into<String>) {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if inner.capacity == 0 {
            return;
        }

        while inner.lines.len() >= inner.capacity {
            inner.lines.pop_front();
        }
        inner.lines.push_back(line.into());
    }

    pub(crate) fn snapshot(&self) -> Vec<String> {
        let inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        inner.lines.iter().cloned().collect()
    }
}

#[derive(Clone, Debug)]
struct LogBufferMakeWriter {
    log_buffer: LogBuffer,
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogBufferMakeWriter {
    type Writer = LogBufferWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LogBufferWriter {
            log_buffer: self.log_buffer.clone(),
            buffer: String::new(),
        }
    }
}

#[derive(Debug)]
struct LogBufferWriter {
    log_buffer: LogBuffer,
    buffer: String,
}

impl LogBufferWriter {
    fn flush_complete_lines(&mut self) {
        while let Some(newline_index) = self.buffer.find('\n') {
            let mut line: String =
                self.buffer.drain(..=newline_index).collect();

            if line.ends_with('\n') {
                line.pop();
            }
            if line.ends_with('\r') {
                line.pop();
            }

            self.log_buffer.push_line(line);
        }
    }
}

impl io::Write for LogBufferWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.push_str(&String::from_utf8_lossy(buf));
        self.flush_complete_lines();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_complete_lines();
        if !self.buffer.is_empty() {
            let mut line = std::mem::take(&mut self.buffer);
            if line.ends_with('\r') {
                line.pop();
            }
            self.log_buffer.push_line(line);
        }
        Ok(())
    }
}

impl Drop for LogBufferWriter {
    fn drop(&mut self) {
        if let Err(err) = self.flush() {
            tracing::debug!(error = %err, "failed to flush LogBufferWriter");
        }
    }
}

#[derive(Clone)]
struct TeeMakeWriter<A, B> {
    left: A,
    right: B,
}

impl<A, B> TeeMakeWriter<A, B> {
    fn new(left: A, right: B) -> Self {
        Self { left, right }
    }
}

impl<'a, A, B> MakeWriter<'a> for TeeMakeWriter<A, B>
where
    A: MakeWriter<'a>,
    B: MakeWriter<'a>,
{
    type Writer = TeeWriter<A::Writer, B::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        TeeWriter {
            left: self.left.make_writer(),
            right: self.right.make_writer(),
        }
    }
}

struct TeeWriter<A, B> {
    left: A,
    right: B,
}

impl<A, B> io::Write for TeeWriter<A, B>
where
    A: io::Write,
    B: io::Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let left_result = self.left.write(buf);
        let right_result = self.right.write(buf);

        match (left_result, right_result) {
            (Err(left_error), Err(_right_error)) => Err(left_error),
            _ => Ok(buf.len()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let left_result = self.left.flush();
        let right_result = self.right.flush();

        match (left_result, right_result) {
            (Err(left_error), Err(_right_error)) => Err(left_error),
            _ => Ok(()),
        }
    }
}

/// Initializes the tracing system for application logging.
///
/// Sets up a daily rolling file logger that writes to app.log in the current
/// directory. Configures the logging level, format, and other options.
///
/// # Returns
/// A guard that must be kept alive for the duration of the application to
/// ensure log messages are properly flushed.
pub fn tracing_init() -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender = tracing_appender::rolling::daily("./", "app.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_file(true)
        .with_line_number(true)
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    guard
}

pub(crate) fn tracing_init_tui(
    log_buffer: LogBuffer,
) -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender = tracing_appender::rolling::daily("./", "app.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let writer =
        TeeMakeWriter::new(non_blocking, LogBufferMakeWriter { log_buffer });

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_file(true)
        .with_line_number(true)
        .with_writer(writer)
        .with_ansi(false)
        .init();

    guard
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_buffer_writer_splits_lines() -> std::io::Result<()> {
        use std::io::Write as _;

        let log_buffer = LogBuffer::new(10);
        let mut writer = LogBufferWriter {
            log_buffer: log_buffer.clone(),
            buffer: String::new(),
        };

        writer.write_all(b"hello\nworld\n")?;

        drop(writer);

        assert_eq!(
            log_buffer.snapshot(),
            vec!["hello".to_string(), "world".to_string()]
        );
        Ok(())
    }

    #[test]
    fn tracing_writes_into_log_buffer() {
        let log_buffer = LogBuffer::new(100);

        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_writer(LogBufferMakeWriter {
                log_buffer: log_buffer.clone(),
            })
            .with_ansi(false)
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("hello tui");
        });

        let joined = log_buffer.snapshot().join("\n");
        assert!(joined.contains("hello tui"));
    }
}
