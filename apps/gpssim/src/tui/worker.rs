use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use gps::{
    IqBlockSizing, RuntimeMotionControl, SignalGenerator,
    SignalGeneratorBuilder,
};

use crate::{
    Error,
    cli::TxBackend,
    error::resolve_run_and_finish,
    tui_config::TuiConfig,
    tx::{FileTxSink, HackrfTxConfig, HackrfTxSink, NullTxSink, TxSink, TxTee},
};

#[derive(Debug, Clone)]
pub(super) struct Progress {
    pub(super) blocks: u64,
    pub(super) elapsed: Duration,
    pub(super) sim_seconds: f64,
    pub(super) throughput_msps: f64,
    pub(super) hackrf_underruns: Option<u64>,
}

#[derive(Debug)]
pub(super) enum WorkerEvent {
    Started { sinks: String },
    Log(String),
    Progress(Progress),
    Finished(Progress),
    Cancelled(Progress),
    Error(String),
}

pub(super) enum PendingTerminalOutcome {
    Finished(Progress),
    Cancelled(Progress),
    Error(String),
}

pub(super) struct WorkerHandle {
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) events: mpsc::Receiver<WorkerEvent>,
    pub(super) join: thread::JoinHandle<()>,
    pub(super) pending_terminal: Option<PendingTerminalOutcome>,
    pub(super) events_disconnected: bool,
}

pub(super) fn spawn_worker(
    config: TuiConfig, manual_control: Option<RuntimeMotionControl>,
) -> std::io::Result<WorkerHandle> {
    let (event_tx, events) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_for_thread = cancel.clone();
    let join = thread::Builder::new()
        .name("gpssim-worker".to_string())
        .spawn(move || {
            worker_thread_main(
                config,
                manual_control,
                cancel_for_thread,
                event_tx,
            );
        })?;

    Ok(WorkerHandle {
        cancel,
        events,
        join,
        pending_terminal: None,
        events_disconnected: false,
    })
}

pub(super) fn resolve_output_path(config: &TuiConfig) -> Option<PathBuf> {
    let tx_selected = !config.tx.is_empty();
    if tx_selected {
        config.output.clone()
    } else {
        config
            .output
            .clone()
            .or_else(|| Some(PathBuf::from("gpssim.bin")))
    }
}

pub(super) fn describe_sinks(config: &TuiConfig) -> String {
    let output_path = resolve_output_path(config);

    let mut parts: Vec<String> = Vec::new();
    if let Some(path) = output_path {
        parts.push(format!("file={}", path.display()));
    }

    if config.tx.iter().any(|b| matches!(b, TxBackend::Hackrf)) {
        parts
            .push(format!("tx=hackrf rf_freq_hz={}", config.hackrf_rf_freq_hz));
    }
    if config.tx.iter().any(|b| matches!(b, TxBackend::Null)) {
        parts.push("tx=null".to_string());
    }

    if parts.is_empty() {
        "<none>".to_string()
    } else {
        parts.join(" ")
    }
}

fn worker_thread_main(
    config: TuiConfig, manual_control: Option<RuntimeMotionControl>,
    cancel: Arc<AtomicBool>, event_tx: mpsc::Sender<WorkerEvent>,
) {
    let sinks = describe_sinks(&config);
    if event_tx.send(WorkerEvent::Started { sinks }).is_err() {
        return;
    }

    if event_tx
        .send(WorkerEvent::Log("worker started".to_string()))
        .is_err()
    {
        return;
    }

    match run_streaming_worker(&config, manual_control, &cancel, &event_tx) {
        Ok(WorkerCompletion::Finished(progress)) => {
            if event_tx.send(WorkerEvent::Finished(progress)).is_err() {
                tracing::debug!("ui disconnected before finished event");
            }
        }
        Ok(WorkerCompletion::Cancelled(progress)) => {
            if event_tx.send(WorkerEvent::Cancelled(progress)).is_err() {
                tracing::debug!("ui disconnected before cancelled event");
            }
        }
        Err(err) => {
            if event_tx.send(WorkerEvent::Error(err.to_string())).is_err() {
                tracing::debug!("ui disconnected before error event");
            }
        }
    }
}

enum WorkerCompletion {
    Finished(Progress),
    Cancelled(Progress),
}

fn run_streaming_worker(
    config: &TuiConfig, manual_control: Option<RuntimeMotionControl>,
    cancel: &AtomicBool, event_tx: &mpsc::Sender<WorkerEvent>,
) -> Result<WorkerCompletion, Error> {
    let output_path = resolve_output_path(config);

    let hackrf_underrun_counter = config
        .tx
        .iter()
        .any(|backend| matches!(backend, TxBackend::Hackrf))
        .then(|| Arc::new(AtomicU64::new(0)));

    let mut generator = build_generator(config, manual_control)?;
    generator.initialize()?;

    let sinks = build_sinks(
        config,
        &generator,
        output_path,
        hackrf_underrun_counter.clone(),
    )?;
    if sinks.is_empty() {
        return Err(Error::cli_error(
            "no output selected: use output and/or tx backends".to_string(),
        ));
    }

    let mut tee = TxTee::new(sinks);
    let started = Instant::now();
    let mut last_progress = Instant::now();

    let sample_frequency_hz = generator.sample_frequency;

    let mut blocks: u64 = 0;
    let mut total_samples: u64 = 0;
    let mut cancelled = false;

    let mut on_block = |block: &[i16]| -> Result<(), Error> {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            return Err(Error::msg("cancelled"));
        }

        blocks = blocks.checked_add(1).ok_or_else(|| {
            gps::Error::unsupported_workload("streaming block count overflow")
        })?;
        let block_samples =
            IqBlockSizing::from_interleaved_i16_len(block.len())?
                .complex_samples();
        let block_samples = u64::try_from(block_samples).map_err(|_| {
            gps::Error::unsupported_workload(
                "streaming sample count exceeds supported range",
            )
        })?;
        total_samples =
            total_samples.checked_add(block_samples).ok_or_else(|| {
                gps::Error::unsupported_workload(
                    "streaming sample count overflow",
                )
            })?;

        if last_progress.elapsed() >= Duration::from_millis(200) {
            let hackrf_underruns = hackrf_underrun_counter
                .as_ref()
                .map(|counter| counter.load(Ordering::Relaxed));
            let progress = compute_progress(
                started,
                blocks,
                total_samples,
                sample_frequency_hz,
                hackrf_underruns,
            );
            if event_tx.send(WorkerEvent::Progress(progress)).is_err() {
                cancelled = true;
                return Err(Error::msg("ui gone"));
            }
            last_progress = Instant::now();
        }

        tee.write_block_i16(block)
    };

    let streaming_result = if config.uses_manual_motion() {
        generator.run_streaming_user_control::<_, Error>(&mut on_block)
    } else {
        generator.run_streaming::<_, Error>(&mut on_block)
    };

    let finish_result = tee.finish();

    let hackrf_underruns = hackrf_underrun_counter
        .as_ref()
        .map(|counter| counter.load(Ordering::Relaxed));
    let progress = compute_progress(
        started,
        blocks,
        total_samples,
        sample_frequency_hz,
        hackrf_underruns,
    );

    let run_result = match streaming_result {
        Ok(()) => Ok(WorkerCompletion::Finished(progress)),
        Err(_) if cancelled => Ok(WorkerCompletion::Cancelled(progress)),
        Err(error) => Err(error),
    };
    resolve_run_and_finish(run_result, finish_result)
}

fn compute_progress(
    started: Instant, blocks: u64, total_samples: u64,
    sample_frequency_hz: f64, hackrf_underruns: Option<u64>,
) -> Progress {
    let elapsed = started.elapsed();
    let elapsed_seconds = elapsed.as_secs_f64();
    let samples_per_second = if elapsed_seconds > 0.0 {
        total_samples as f64 / elapsed_seconds
    } else {
        0.0
    };
    Progress {
        blocks,
        elapsed,
        sim_seconds: total_samples as f64 / sample_frequency_hz,
        throughput_msps: samples_per_second / 1_000_000.0,
        hackrf_underruns,
    }
}

fn build_generator(
    config: &TuiConfig, manual_control: Option<RuntimeMotionControl>,
) -> Result<SignalGenerator, Error> {
    let mut builder = SignalGeneratorBuilder::default()
        .navigation_file(config.ephemerides.clone())?
        .leap(config.leap.clone())
        .utc_time(config.time.clone())?
        .time_override(config.time_override)
        .duration(config.effective_duration())
        .output_file(None)
        .frequency(Some(config.frequency))?
        .data_format(Some(config.bits))?
        .ionospheric_disable(Some(config.ionospheric_disable))
        .path_loss(config.path_loss)
        .verbose(Some(config.verbose));

    if config.uses_manual_motion() {
        let Some(control) = manual_control else {
            return Err(Error::cli_error(
                "manual mode requires a runtime motion control handle"
                    .to_string(),
            ));
        };
        builder = builder.runtime_motion_control(Some(control))?;
    } else {
        builder = builder
            .user_motion_file(config.user_motion_ecef.clone())?
            .user_motion_llh_file(config.user_motion_llh.clone())?
            .user_motion_nmea_gga_file(config.nmea_gga.clone())?
            .location_ecef(config.location_ecef.clone())?
            .location(config.location.clone())?;
    }

    builder.build().map_err(Into::into)
}

fn build_sinks(
    config: &TuiConfig, generator: &SignalGenerator,
    output_path: Option<PathBuf>,
    hackrf_underrun_counter: Option<Arc<AtomicU64>>,
) -> Result<Vec<Box<dyn TxSink>>, Error> {
    let mut sinks: Vec<Box<dyn TxSink>> = Vec::new();

    if let Some(path) = output_path {
        sinks.push(Box::new(FileTxSink::new(
            path,
            generator.data_format,
            generator.iq_buffer_size,
        )?));
    }

    for backend in &config.tx {
        match backend {
            TxBackend::Hackrf => {
                sinks.push(Box::new(build_hackrf_sink(
                    config,
                    generator,
                    hackrf_underrun_counter.clone(),
                )?));
            }
            TxBackend::Null => {
                if !sinks.iter().any(|s| s.backend() == "null") {
                    sinks.push(Box::new(NullTxSink::new()));
                }
            }
        }
    }

    Ok(sinks)
}

fn build_hackrf_sink(
    config: &TuiConfig, generator: &SignalGenerator,
    underrun_counter: Option<Arc<AtomicU64>>,
) -> Result<HackrfTxSink, Error> {
    let tx_config = HackrfTxConfig {
        serial: config.hackrf_serial.clone(),
        rf_freq_hz: config.hackrf_rf_freq_hz,
        sample_frequency_hz: generator.sample_frequency,
        step_duration: Duration::from_secs_f64(generator.sample_rate),
        txvga_gain: config.hackrf_txvga_gain,
        amp_enable: config.hackrf_amp_enable,
        usb_transfer_bytes: config.hackrf_usb_transfer_bytes,
        usb_transfers: config.hackrf_usb_transfers,
        queue_blocks: config.hackrf_queue_blocks,
        prefill_blocks: config.hackrf_prefill_blocks,
        silence_on_underrun: !config.hackrf_drop_on_underrun,
        underrun_counter,
    };

    let expected_i16_len =
        IqBlockSizing::new(generator.iq_buffer_size)?.interleaved_i16_len();
    HackrfTxSink::new(tx_config, expected_i16_len)
}

#[cfg(test)]
#[path = "worker_tests.rs"]
mod tests;
