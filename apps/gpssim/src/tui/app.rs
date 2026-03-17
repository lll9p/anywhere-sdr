use std::{
    path::PathBuf,
    sync::{atomic::Ordering, mpsc},
};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::worker::{
    Progress, WorkerEvent, WorkerHandle, describe_sinks, spawn_worker,
};
use crate::{cli::TxBackend, tui_config::TuiConfig, utils::LogBuffer};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ActiveTab {
    Config,
    Run,
    Logs,
}

impl ActiveTab {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Config => Self::Run,
            Self::Run => Self::Logs,
            Self::Logs => Self::Config,
        }
    }

    pub(super) fn prev(self) -> Self {
        match self {
            Self::Config => Self::Logs,
            Self::Run => Self::Config,
            Self::Logs => Self::Run,
        }
    }

    pub(super) fn index(self) -> usize {
        match self {
            Self::Config => 0,
            Self::Run => 1,
            Self::Logs => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditField {
    Ephemerides,
    Output,
    Frequency,
    Bits,
    Duration,
    HackrfSerial,
    HackrfRfFreqHz,
}

impl EditField {
    fn label(self) -> &'static str {
        match self {
            Self::Ephemerides => "ephemerides",
            Self::Output => "output",
            Self::Frequency => "frequency",
            Self::Bits => "bits",
            Self::Duration => "duration",
            Self::HackrfSerial => "hackrf_serial",
            Self::HackrfRfFreqHz => "hackrf_rf_freq_hz",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InputMode {
    Normal,
    Editing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RunState {
    Idle,
    Running,
    Stopping,
}

#[derive(Debug, Clone)]
pub(super) enum LastRun {
    Finished,
    Cancelled,
    Error(String),
}

pub(super) struct App {
    pub(super) tab: ActiveTab,
    pub(super) input_mode: InputMode,
    input_field: Option<EditField>,
    pub(super) input_buffer: String,
    pub(super) message: Option<String>,
    exit_requested: bool,

    pub(super) config: TuiConfig,

    pub(super) run_state: RunState,
    pub(super) sinks_desc: String,
    pub(super) progress: Option<Progress>,
    pub(super) last_run: Option<LastRun>,

    pub(super) log_buffer: LogBuffer,
    pub(super) log_scroll: u16,

    worker: Option<WorkerHandle>,
    worker_events_rx: mpsc::Receiver<WorkerEvent>,
    worker_events_tx: mpsc::Sender<WorkerEvent>,
}

impl App {
    pub(super) fn new(
        config: TuiConfig, log_buffer: LogBuffer,
        worker_events_rx: mpsc::Receiver<WorkerEvent>,
        worker_events_tx: mpsc::Sender<WorkerEvent>,
    ) -> Self {
        let sinks_desc = describe_sinks(&config);
        Self {
            tab: ActiveTab::Config,
            input_mode: InputMode::Normal,
            input_field: None,
            input_buffer: String::new(),
            message: None,
            exit_requested: false,
            config,
            run_state: RunState::Idle,
            sinks_desc,
            progress: None,
            last_run: None,
            log_buffer,
            log_scroll: 0,
            worker: None,
            worker_events_rx,
            worker_events_tx,
        }
    }

    pub(super) fn should_exit(&self) -> bool {
        self.exit_requested && self.worker.is_none()
    }

    pub(super) fn drain_worker_events(&mut self) {
        while let Ok(event) = self.worker_events_rx.try_recv() {
            self.handle_worker_event(event);
        }
    }

    pub(super) fn push_log(&mut self, line: impl Into<String>) {
        self.log_buffer.push_line(line);
    }

    fn request_exit(&mut self) {
        self.exit_requested = true;
        self.request_cancel();
    }

    fn request_cancel(&mut self) {
        if let Some(worker) = &self.worker {
            worker.cancel.store(true, Ordering::Relaxed);
            self.run_state = RunState::Stopping;
        }
    }

    fn join_worker_if_done(&mut self) {
        let Some(worker) = self.worker.take() else {
            return;
        };

        match worker.join.join() {
            Ok(()) => {}
            Err(_) => {
                self.push_log("worker thread panicked");
            }
        }
        self.run_state = RunState::Idle;
    }

    fn handle_worker_event(&mut self, event: WorkerEvent) {
        match event {
            WorkerEvent::Started { sinks } => {
                self.sinks_desc = sinks;
                self.push_log("run started");
                self.run_state = RunState::Running;
            }
            WorkerEvent::Log(line) => {
                self.push_log(line);
            }
            WorkerEvent::Progress(progress) => {
                self.progress = Some(progress);
                self.run_state = RunState::Running;
            }
            WorkerEvent::Finished(progress) => {
                self.progress = Some(progress);
                self.last_run = Some(LastRun::Finished);
                self.push_log("run finished");
                self.join_worker_if_done();
            }
            WorkerEvent::Cancelled(progress) => {
                self.progress = Some(progress);
                self.last_run = Some(LastRun::Cancelled);
                self.push_log("run cancelled");
                self.join_worker_if_done();
            }
            WorkerEvent::Error(message) => {
                self.last_run = Some(LastRun::Error(message.clone()));
                self.push_log(format!("run error: {message}"));
                self.join_worker_if_done();
            }
        }
    }
}

pub(super) fn handle_key_event(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
        return;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('c'))
    {
        app.request_exit();
        return;
    }

    if app.input_mode == InputMode::Editing {
        handle_edit_key(app, key);
        return;
    }

    match key.code {
        KeyCode::Char('q' | 'Q') => {
            app.request_exit();
        }
        KeyCode::Tab => {
            app.tab = app.tab.next();
        }
        KeyCode::BackTab => {
            app.tab = app.tab.prev();
        }
        KeyCode::Enter => {
            if app.tab == ActiveTab::Config {
                start_run(app);
            }
        }
        KeyCode::Esc => {
            if app.worker.is_some() {
                app.request_cancel();
            }
        }
        KeyCode::Up => {
            if app.tab == ActiveTab::Logs {
                app.log_scroll = app.log_scroll.saturating_add(1);
            }
        }
        KeyCode::Down => {
            if app.tab == ActiveTab::Logs {
                app.log_scroll = app.log_scroll.saturating_sub(1);
            }
        }
        KeyCode::Char(ch) => {
            handle_char_shortcut(app, ch);
        }
        _ => {}
    }
}

fn handle_char_shortcut(app: &mut App, ch: char) {
    let ch = ch.to_ascii_lowercase();
    match (app.tab, ch) {
        (ActiveTab::Config, 'e') => {
            begin_edit(app, EditField::Ephemerides);
        }
        (ActiveTab::Config, 'o') => begin_edit(app, EditField::Output),
        (ActiveTab::Config, 's') => {
            begin_edit(app, EditField::Frequency);
        }
        (ActiveTab::Config, 'b') => begin_edit(app, EditField::Bits),
        (ActiveTab::Config, 'd') => {
            begin_edit(app, EditField::Duration);
        }
        (ActiveTab::Config, 'h') => {
            toggle_tx_backend_and_update_status(app, TxBackend::Hackrf);
        }
        (ActiveTab::Config, 'n') => {
            toggle_tx_backend_and_update_status(app, TxBackend::Null);
        }
        (ActiveTab::Config, 'v') => {
            app.config.verbose = !app.config.verbose;
        }
        (ActiveTab::Config, 'i') => {
            app.config.ionospheric_disable = !app.config.ionospheric_disable;
        }
        (ActiveTab::Config, 'r') => {
            begin_edit(app, EditField::HackrfRfFreqHz);
        }
        (ActiveTab::Config, 'x') => {
            begin_edit(app, EditField::HackrfSerial);
        }
        (ActiveTab::Run, 'c') => {
            app.request_cancel();
        }
        _ => {}
    }
}

fn toggle_tx_backend_and_update_status(app: &mut App, backend: TxBackend) {
    toggle_tx_backend(&mut app.config.tx, backend);
    app.sinks_desc = describe_sinks(&app.config);

    if !app.config.tx.is_empty() && app.config.output.is_none() {
        app.message = Some(
            "tx enabled: file output disabled unless output path is set \
             (press o)"
                .to_string(),
        );
    } else {
        app.message = None;
    }
}

fn begin_edit(app: &mut App, field: EditField) {
    app.input_mode = InputMode::Editing;
    app.input_field = Some(field);
    app.input_buffer = match field {
        EditField::Ephemerides => app
            .config
            .ephemerides
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        EditField::Output => app
            .config
            .output
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        EditField::Frequency => app.config.frequency.to_string(),
        EditField::Bits => app.config.bits.to_string(),
        EditField::Duration => app
            .config
            .duration
            .map_or_else(String::new, |d| d.to_string()),
        EditField::HackrfSerial => {
            app.config.hackrf_serial.clone().unwrap_or_default()
        }
        EditField::HackrfRfFreqHz => app.config.hackrf_rf_freq_hz.to_string(),
    };
    app.message = Some(format!(
        "editing {} (Enter=save, Esc=cancel)",
        field.label()
    ));
}

fn handle_edit_key(app: &mut App, key: KeyEvent) {
    let Some(field) = app.input_field else {
        app.input_mode = InputMode::Normal;
        return;
    };

    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.input_field = None;
            app.input_buffer.clear();
            app.message = None;
        }
        KeyCode::Enter => match apply_edit(app, field) {
            Ok(()) => {
                app.input_mode = InputMode::Normal;
                app.input_field = None;
                app.input_buffer.clear();
                app.message = None;
                app.sinks_desc = describe_sinks(&app.config);
            }
            Err(message) => {
                app.message = Some(message);
            }
        },
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(ch) => {
            if !key.modifiers.contains(KeyModifiers::CONTROL) {
                app.input_buffer.push(ch);
            }
        }
        _ => {}
    }
}

fn apply_edit(app: &mut App, field: EditField) -> Result<(), String> {
    let value = app.input_buffer.trim();

    match field {
        EditField::Ephemerides => {
            app.config.ephemerides = if value.is_empty() {
                None
            } else {
                Some(PathBuf::from(value))
            };
        }
        EditField::Output => {
            app.config.output = if value.is_empty() {
                None
            } else {
                Some(PathBuf::from(value))
            };
        }
        EditField::Frequency => {
            let frequency = value
                .parse::<usize>()
                .map_err(|err| format!("invalid frequency: {err}"))?;
            if frequency == 0 {
                return Err("frequency must be > 0".to_string());
            }
            app.config.frequency = frequency;
        }
        EditField::Bits => {
            let bits = value
                .parse::<usize>()
                .map_err(|err| format!("invalid bits: {err}"))?;
            if !matches!(bits, 1 | 8 | 16) {
                return Err("bits must be one of: 1, 8, 16".to_string());
            }
            app.config.bits = bits;
        }
        EditField::Duration => {
            if value.is_empty() {
                app.config.duration = None;
            } else {
                let duration = value
                    .parse::<f64>()
                    .map_err(|err| format!("invalid duration: {err}"))?;
                app.config.duration = Some(duration);
            }
        }
        EditField::HackrfSerial => {
            app.config.hackrf_serial = if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            };
        }
        EditField::HackrfRfFreqHz => {
            let rf_freq_hz = value
                .parse::<u64>()
                .map_err(|err| format!("invalid rf freq: {err}"))?;
            if rf_freq_hz == 0 {
                return Err("hackrf_rf_freq_hz must be > 0".to_string());
            }
            app.config.hackrf_rf_freq_hz = rf_freq_hz;
        }
    }

    Ok(())
}

fn toggle_tx_backend(tx: &mut Vec<TxBackend>, backend: TxBackend) {
    if let Some(position) = tx.iter().position(|b| *b == backend) {
        tx.remove(position);
    } else {
        tx.push(backend);
    }
}

fn validate_config_for_run(config: &TuiConfig) -> Result<(), String> {
    if config.ephemerides.is_none() {
        return Err("ephemerides is required".to_string());
    }

    if config.tx.iter().any(|b| matches!(b, TxBackend::Hackrf)) {
        if config.hackrf_usb_transfer_bytes == 0 {
            return Err("hackrf_usb_transfer_bytes must be > 0".to_string());
        }
        if config.hackrf_usb_transfers == 0 {
            return Err("hackrf_usb_transfers must be > 0".to_string());
        }
        if config.hackrf_queue_blocks == 0 {
            return Err("hackrf_queue_blocks must be > 0".to_string());
        }
        if config.hackrf_txvga_gain > 47 {
            return Err("hackrf_txvga_gain must be in 0..=47".to_string());
        }
    }

    Ok(())
}

fn start_run(app: &mut App) {
    if app.worker.is_some() {
        app.message = Some("already running".to_string());
        return;
    }

    if let Err(message) = validate_config_for_run(&app.config) {
        app.message = Some(message);
        return;
    }

    let config = app.config.clone();
    app.last_run = None;
    app.progress = None;
    app.run_state = RunState::Running;
    app.sinks_desc = describe_sinks(&config);

    let handle = spawn_worker(config, app.worker_events_tx.clone());
    app.worker = Some(handle);
    app.tab = ActiveTab::Run;
}
