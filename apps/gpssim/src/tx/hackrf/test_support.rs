use std::{
    io::{self, Write},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

use super::{
    BufferObserver, HackrfTxConfig, HackrfTxSink, config_context,
    startup::{ActivationGuard, HackrfDeviceControl},
    writer::writer_thread_main,
};
use crate::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Event {
    Open,
    SetFreq,
    SetSampleRate,
    SetAmp,
    SetGain,
    PrepareWriter,
    SpawnWriter,
    Buffer(usize),
    Activate,
    Write(u8),
    Flush,
    Stop,
}

#[derive(Clone, Default)]
pub(super) struct EventLog(Arc<Mutex<Vec<Event>>>);

impl EventLog {
    pub(super) fn push(&self, event: Event) {
        match self.0.lock() {
            Ok(mut events) => events.push(event),
            Err(poisoned) => poisoned.into_inner().push(event),
        }
    }

    pub(super) fn snapshot(&self) -> Vec<Event> {
        match self.0.lock() {
            Ok(events) => events.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    pub(super) fn clear(&self) {
        match self.0.lock() {
            Ok(mut events) => events.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FailurePoint {
    Open,
    SetFreq,
    SetSampleRate,
    SetAmp,
    SetGain,
    PrepareWriter,
    Activate,
    PanicActivate,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum WriterBehavior {
    #[default]
    Success,
    FailWrite,
    FailFlush,
    PanicWrite,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum SpawnBehavior {
    #[default]
    Standard,
    Fail,
    DropReceiver,
    DropAfterOne,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Scenario {
    pub(super) failure: Option<FailurePoint>,
    pub(super) writer: WriterBehavior,
    pub(super) spawn: SpawnBehavior,
    pub(super) stop_failures: usize,
}

pub(super) fn valid_config() -> HackrfTxConfig {
    HackrfTxConfig {
        serial: None,
        rf_freq_hz: 1_575_420_000,
        sample_frequency_hz: 2_600_000.0,
        step_duration: Duration::from_mins(1),
        txvga_gain: 20,
        amp_enable: false,
        usb_transfer_bytes: 4096,
        usb_transfers: 2,
        queue_blocks: 8,
        prefill_blocks: 2,
        silence_on_underrun: true,
        underrun_counter: None,
    }
}

pub(super) fn iq_block(index: u8) -> [i16; 4] {
    let value = i16::from(index) * 16;
    [value, -value, value, -value]
}

pub(super) fn build_sink(
    config: HackrfTxConfig, expected_i16_len: usize, scenario: Scenario,
) -> (
    Result<HackrfTxSink, Error>,
    EventLog,
    Option<mpsc::Receiver<()>>,
) {
    let events = EventLog::default();
    let observer_events = events.clone();
    let observer: BufferObserver = Arc::new(move |index| {
        observer_events.push(Event::Buffer(index));
    });

    let opener_events = events.clone();
    let writer_events = events.clone();
    let spawn_events = events.clone();
    let thread_events = events.clone();
    let (closed_sender, closed_receiver) = mpsc::channel();
    let closed_notification = if scenario.spawn == SpawnBehavior::DropAfterOne {
        Some(closed_receiver)
    } else {
        None
    };

    let result = HackrfTxSink::new_with(
        config,
        expected_i16_len,
        move |_| {
            opener_events.push(Event::Open);
            if scenario.failure == Some(FailurePoint::Open) {
                return Err(synthetic_error("open"));
            }
            Ok(Box::new(FakeDevice {
                events: opener_events,
                failure: scenario.failure,
                stop_failures: scenario.stop_failures,
                writer: Some(Box::new(FakeWriter {
                    events: writer_events,
                    behavior: scenario.writer,
                })),
            }))
        },
        move |config, receiver, writer| {
            spawn_events.push(Event::SpawnWriter);
            match scenario.spawn {
                SpawnBehavior::Standard => thread::Builder::new()
                    .name("hackrf-test-writer".to_string())
                    .spawn(move || {
                        writer_thread_main(config, receiver, writer)
                    }),
                SpawnBehavior::Fail => {
                    Err(io::Error::other("synthetic writer spawn failure"))
                }
                SpawnBehavior::DropReceiver => {
                    let (ready_sender, ready_receiver) = mpsc::channel();
                    let handle = thread::Builder::new()
                        .name("hackrf-test-disconnect".to_string())
                        .spawn(move || {
                            drop(receiver);
                            drop(writer);
                            ready_sender.send(()).map_err(|error| {
                                Error::tx_backend_with_source(
                                    "hackrf",
                                    config_context(&config),
                                    error,
                                )
                            })?;
                            Ok(())
                        })?;
                    ready_receiver.recv().map_err(|error| {
                        io::Error::other(format!(
                            "disconnect synchronization failed: {error}"
                        ))
                    })?;
                    Ok(handle)
                }
                SpawnBehavior::DropAfterOne => thread::Builder::new()
                    .name("hackrf-test-one-block".to_string())
                    .spawn(move || {
                        let block = receiver.recv().map_err(|error| {
                            Error::tx_backend_with_source(
                                "hackrf",
                                config_context(&config),
                                error,
                            )
                        })?;
                        thread_events.push(Event::Write(
                            block.first().copied().unwrap_or_default(),
                        ));
                        drop(receiver);
                        drop(writer);
                        closed_sender.send(()).map_err(|error| {
                            Error::tx_backend_with_source(
                                "hackrf",
                                config_context(&config),
                                error,
                            )
                        })?;
                        Ok(())
                    }),
            }
        },
        Some(observer),
    );

    (result, events, closed_notification)
}

pub(super) fn fake_activation_guard(
    config: &HackrfTxConfig, scenario: Scenario, events: EventLog,
) -> ActivationGuard {
    let writer_events = events.clone();
    ActivationGuard::new(
        Box::new(FakeDevice {
            events,
            failure: scenario.failure,
            stop_failures: scenario.stop_failures,
            writer: Some(Box::new(FakeWriter {
                events: writer_events,
                behavior: scenario.writer,
            })),
        }),
        config_context(config),
    )
}

struct FakeDevice {
    events: EventLog,
    failure: Option<FailurePoint>,
    stop_failures: usize,
    writer: Option<Box<dyn Write + Send>>,
}

impl FakeDevice {
    fn operation(
        &self, event: Event, failure: FailurePoint, name: &str,
    ) -> Result<(), Error> {
        self.events.push(event);
        if self.failure == Some(failure) {
            Err(synthetic_error(name))
        } else {
            Ok(())
        }
    }
}

impl HackrfDeviceControl for FakeDevice {
    fn set_freq(&mut self, _frequency_hz: u64) -> Result<(), Error> {
        self.operation(Event::SetFreq, FailurePoint::SetFreq, "set frequency")
    }

    fn set_sample_rate_auto(
        &mut self, _frequency_hz: f64,
    ) -> Result<(), Error> {
        self.operation(
            Event::SetSampleRate,
            FailurePoint::SetSampleRate,
            "set sample rate",
        )
    }

    fn set_amp_enable(&mut self, _enabled: bool) -> Result<(), Error> {
        self.operation(Event::SetAmp, FailurePoint::SetAmp, "set amplifier")
    }

    fn set_txvga_gain(&mut self, _gain: u16) -> Result<(), Error> {
        self.operation(Event::SetGain, FailurePoint::SetGain, "set TX gain")
    }

    fn prepare_tx_writer(
        &mut self, _transfer_bytes: usize, _transfers: usize,
    ) -> Result<Box<dyn Write + Send>, Error> {
        self.operation(
            Event::PrepareWriter,
            FailurePoint::PrepareWriter,
            "prepare writer",
        )?;
        self.writer
            .take()
            .ok_or_else(|| synthetic_error("writer already prepared"))
    }

    fn enter_tx_mode(&mut self) -> Result<(), Error> {
        self.events.push(Event::Activate);
        assert!(
            self.failure != Some(FailurePoint::PanicActivate),
            "synthetic activation panic"
        );
        if self.failure == Some(FailurePoint::Activate) {
            Err(synthetic_error("activate"))
        } else {
            Ok(())
        }
    }

    fn stop_tx(&mut self) -> Result<(), Error> {
        self.events.push(Event::Stop);
        if self.stop_failures > 0 {
            self.stop_failures -= 1;
            Err(synthetic_error("rollback stop"))
        } else {
            Ok(())
        }
    }
}

struct FakeWriter {
    events: EventLog,
    behavior: WriterBehavior,
}

impl Write for FakeWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.events
            .push(Event::Write(buffer.first().copied().unwrap_or_default()));
        match self.behavior {
            WriterBehavior::FailWrite => {
                Err(io::Error::other("synthetic writer write failure"))
            }
            WriterBehavior::PanicWrite => {
                panic!("synthetic writer panic");
            }
            WriterBehavior::Success | WriterBehavior::FailFlush => {
                Ok(buffer.len())
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.events.push(Event::Flush);
        if self.behavior == WriterBehavior::FailFlush {
            Err(io::Error::other("synthetic writer flush failure"))
        } else {
            Ok(())
        }
    }
}

fn synthetic_error(operation: &str) -> Error {
    Error::tx_backend_msg("hackrf", format!("synthetic {operation} failure"))
}

#[derive(Clone, Default)]
struct CapturedWriter(Arc<Mutex<Vec<u8>>>);

impl Write for CapturedWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        match self.0.lock() {
            Ok(mut output) => output.extend_from_slice(buffer),
            Err(poisoned) => poisoned.into_inner().extend_from_slice(buffer),
        }
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn capture_tracing<T>(operation: impl FnOnce() -> T) -> (T, String) {
    let writer = CapturedWriter::default();
    let output = writer.0.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_target(false)
        .with_writer(move || writer.clone())
        .finish();
    let result = tracing::subscriber::with_default(subscriber, operation);
    let bytes = match output.lock() {
        Ok(bytes) => bytes.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    };
    (result, String::from_utf8_lossy(&bytes).into_owned())
}
