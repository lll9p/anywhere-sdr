use std::{
    io::{self, Write},
    sync::{Arc, atomic::Ordering, mpsc},
    time::{Duration, Instant},
};

use super::{HackrfTxConfig, config_context, shutdown::CancellationToken};
use crate::Error;

pub(super) enum WriterOutcome {
    Completed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WriterObservation {
    WaitingForFirstBlock,
    WaitingBetweenBlocks,
    BeforeDataWrite,
    BeforeSilenceWrite,
    BeforeFlush,
}

pub(super) type WriterObserver = Arc<dyn Fn(WriterObservation) + Send + Sync>;

enum ReceiveOutcome {
    Block(Vec<u8>),
    Underrun,
    Disconnected,
    Cancelled,
}

struct WriterMetrics {
    bytes_since_log: u64,
    bytes_total: u64,
    underruns: u64,
    last_log: Instant,
    last_log_total: Instant,
}

impl WriterMetrics {
    fn new() -> Self {
        Self {
            bytes_since_log: 0,
            bytes_total: 0,
            underruns: 0,
            last_log: Instant::now(),
            last_log_total: Instant::now(),
        }
    }

    fn record_bytes(&mut self, bytes: usize) {
        self.bytes_since_log += bytes as u64;
        self.bytes_total += bytes as u64;
    }

    fn record_underrun(&mut self, config: &HackrfTxConfig) {
        self.underruns = self.underruns.wrapping_add(1);
        if let Some(counter) = &config.underrun_counter {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn maybe_log(&mut self) {
        if self.last_log.elapsed() < Duration::from_secs(1) {
            return;
        }
        let elapsed = self.last_log_total.elapsed().as_secs_f64().max(1e-6);
        let mbps = (self.bytes_since_log as f64 / elapsed) / (1024.0 * 1024.0);
        tracing::info!(
            mbps = %format_args!("{mbps:.2}"),
            underruns = self.underruns,
            bytes_sent_total = self.bytes_total,
            "hackrf tx"
        );
        self.bytes_since_log = 0;
        self.last_log = Instant::now();
        self.last_log_total = Instant::now();
    }
}

pub(super) fn writer_thread_main(
    config: HackrfTxConfig, receiver: mpsc::Receiver<Vec<u8>>,
    writer: impl Write, cancellation: CancellationToken,
    cancellation_poll: Duration,
) -> Result<WriterOutcome, Error> {
    writer_thread_main_with_observer(
        config,
        receiver,
        writer,
        cancellation,
        cancellation_poll,
        None,
    )
}

#[cfg(test)]
pub(super) fn writer_thread_main_observed(
    config: HackrfTxConfig, receiver: mpsc::Receiver<Vec<u8>>,
    writer: impl Write, cancellation: CancellationToken,
    cancellation_poll: Duration, observer: WriterObserver,
) -> Result<WriterOutcome, Error> {
    writer_thread_main_with_observer(
        config,
        receiver,
        writer,
        cancellation,
        cancellation_poll,
        Some(observer),
    )
}

fn writer_thread_main_with_observer(
    config: HackrfTxConfig, receiver: mpsc::Receiver<Vec<u8>>,
    mut writer: impl Write, cancellation: CancellationToken,
    cancellation_poll: Duration, observer: Option<WriterObserver>,
) -> Result<WriterOutcome, Error> {
    let observer = observer.as_ref();
    let first_block = loop {
        observe(observer, WriterObservation::WaitingForFirstBlock);
        if cancellation.is_requested() {
            return Ok(WriterOutcome::Cancelled);
        }
        match receiver.recv_timeout(cancellation_poll) {
            Ok(block) => break block,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return complete_writer(
                    &config,
                    &mut writer,
                    &cancellation,
                    observer,
                );
            }
        }
    };
    stream_blocks(
        &config,
        &receiver,
        &mut writer,
        &cancellation,
        cancellation_poll,
        observer,
        first_block,
    )
}

fn stream_blocks(
    config: &HackrfTxConfig, receiver: &mpsc::Receiver<Vec<u8>>,
    writer: &mut impl Write, cancellation: &CancellationToken,
    cancellation_poll: Duration, observer: Option<&WriterObserver>,
    first_block: Vec<u8>,
) -> Result<WriterOutcome, Error> {
    let mut next_block = Some(first_block);
    let mut silence = Vec::new();
    let underrun_timeout = Duration::from_secs_f64(
        (config.step_duration.as_secs_f64() / 2.0).max(0.001),
    );
    let mut metrics = WriterMetrics::new();

    loop {
        let next = next_block.take().map_or_else(
            || {
                receive_until_underrun(
                    receiver,
                    cancellation,
                    cancellation_poll,
                    underrun_timeout,
                    observer,
                )
            },
            ReceiveOutcome::Block,
        );

        match next {
            ReceiveOutcome::Block(block) => {
                if silence.is_empty() {
                    silence.resize(block.len(), 0);
                }
                observe(observer, WriterObservation::BeforeDataWrite);
                if !write_all_cancellable(config, writer, &block, cancellation)?
                {
                    return Ok(WriterOutcome::Cancelled);
                }
                metrics.record_bytes(block.len());
            }
            ReceiveOutcome::Underrun => {
                metrics.record_underrun(config);
                if config.silence_on_underrun && !silence.is_empty() {
                    observe(observer, WriterObservation::BeforeSilenceWrite);
                    if !write_all_cancellable(
                        config,
                        writer,
                        &silence,
                        cancellation,
                    )? {
                        return Ok(WriterOutcome::Cancelled);
                    }
                    metrics.record_bytes(silence.len());
                }
            }
            ReceiveOutcome::Disconnected => {
                return complete_writer(config, writer, cancellation, observer);
            }
            ReceiveOutcome::Cancelled => {
                return Ok(WriterOutcome::Cancelled);
            }
        }
        metrics.maybe_log();
    }
}

fn receive_until_underrun(
    receiver: &mpsc::Receiver<Vec<u8>>, cancellation: &CancellationToken,
    cancellation_poll: Duration, underrun_timeout: Duration,
    observer: Option<&WriterObserver>,
) -> ReceiveOutcome {
    let started = Instant::now();
    loop {
        observe(observer, WriterObservation::WaitingBetweenBlocks);
        if cancellation.is_requested() {
            return ReceiveOutcome::Cancelled;
        }
        let remaining = underrun_timeout.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            return ReceiveOutcome::Underrun;
        }
        match receiver.recv_timeout(cancellation_poll.min(remaining)) {
            Ok(block) => return ReceiveOutcome::Block(block),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return ReceiveOutcome::Disconnected;
            }
        }
    }
}

fn write_all_cancellable(
    config: &HackrfTxConfig, writer: &mut impl Write, mut buffer: &[u8],
    cancellation: &CancellationToken,
) -> Result<bool, Error> {
    while !buffer.is_empty() {
        if cancellation.is_requested() {
            return Ok(false);
        }
        let written = writer
            .write(buffer)
            .map_err(|error| writer_error(config, error))?;
        if written == 0 {
            return Err(writer_error(
                config,
                io::Error::new(
                    io::ErrorKind::WriteZero,
                    "failed to write the complete HackRF block",
                ),
            ));
        }
        buffer = &buffer[written..];
    }
    Ok(!cancellation.is_requested())
}

fn complete_writer(
    config: &HackrfTxConfig, writer: &mut impl Write,
    cancellation: &CancellationToken, observer: Option<&WriterObserver>,
) -> Result<WriterOutcome, Error> {
    observe(observer, WriterObservation::BeforeFlush);
    if cancellation.is_requested() {
        return Ok(WriterOutcome::Cancelled);
    }
    writer
        .flush()
        .map_err(|error| writer_error(config, error))?;
    if cancellation.is_requested() {
        Ok(WriterOutcome::Cancelled)
    } else {
        Ok(WriterOutcome::Completed)
    }
}

fn observe(observer: Option<&WriterObserver>, observation: WriterObservation) {
    if let Some(observer) = observer {
        observer(observation);
    }
}

fn writer_error(config: &HackrfTxConfig, error: std::io::Error) -> Error {
    Error::tx_backend_with_source("hackrf", config_context(config), error)
}
