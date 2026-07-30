use std::{
    io::Write,
    sync::{atomic::Ordering, mpsc},
    time::{Duration, Instant},
};

use super::{HackrfTxConfig, config_context};
use crate::Error;

pub(super) fn writer_thread_main(
    config: HackrfTxConfig, receiver: mpsc::Receiver<Vec<u8>>,
    mut writer: impl Write,
) -> Result<(), Error> {
    let Ok(first_block) = receiver.recv() else {
        return flush_writer(&config, &mut writer);
    };

    let mut next_block = Some(first_block);
    let mut silence = Vec::new();
    let underrun_timeout = Duration::from_secs_f64(
        (config.step_duration.as_secs_f64() / 2.0).max(0.001),
    );

    let mut bytes_sent_since_log = 0_u64;
    let mut bytes_sent_total = 0_u64;
    let mut underruns = 0_u64;
    let mut last_log = Instant::now();
    let mut last_log_total = Instant::now();
    let log_interval = Duration::from_secs(1);

    loop {
        let next = if let Some(block) = next_block.take() {
            Some(block)
        } else {
            match receiver.recv_timeout(underrun_timeout) {
                Ok(block) => Some(block),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        };

        if let Some(block) = next {
            if silence.is_empty() {
                silence.resize(block.len(), 0);
            }
            writer
                .write_all(&block)
                .map_err(|error| writer_error(&config, error))?;
            bytes_sent_since_log += block.len() as u64;
            bytes_sent_total += block.len() as u64;
        } else {
            underruns = underruns.wrapping_add(1);
            if let Some(counter) = &config.underrun_counter {
                counter.fetch_add(1, Ordering::Relaxed);
            }

            if config.silence_on_underrun {
                if silence.is_empty() {
                    continue;
                }
                writer
                    .write_all(&silence)
                    .map_err(|error| writer_error(&config, error))?;
                bytes_sent_since_log += silence.len() as u64;
                bytes_sent_total += silence.len() as u64;
            }
        }

        if last_log.elapsed() >= log_interval {
            let elapsed = last_log_total.elapsed().as_secs_f64().max(1e-6);
            let mbps =
                (bytes_sent_since_log as f64 / elapsed) / (1024.0 * 1024.0);
            tracing::info!(
                mbps = %format_args!("{mbps:.2}"),
                underruns,
                bytes_sent_total,
                "hackrf tx"
            );
            bytes_sent_since_log = 0;
            last_log = Instant::now();
            last_log_total = Instant::now();
        }
    }

    flush_writer(&config, &mut writer)
}

fn flush_writer(
    config: &HackrfTxConfig, writer: &mut impl Write,
) -> Result<(), Error> {
    writer.flush().map_err(|error| writer_error(config, error))
}

fn writer_error(config: &HackrfTxConfig, error: std::io::Error) -> Error {
    Error::tx_backend_with_source("hackrf", config_context(config), error)
}
