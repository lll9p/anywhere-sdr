use std::sync::{atomic::Ordering, mpsc};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::worker::{
    Progress, WorkerEvent, WorkerHandle, describe_sinks, spawn_worker,
};
use super::{
    config_edit::{
        EditField, begin_edit, handle_edit_key, toggle_motion_source,
        toggle_tx_backend_and_update_status,
    },
    manual_control::{
        MANUAL_HEADING_STEP_DEG, MANUAL_SPEED_STEP_MPS, ManualControlSession,
    },
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
    pub(super) input_field: Option<EditField>,
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
    pub(super) manual_session: Option<ManualControlSession>,

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
            manual_session: None,
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

    pub(super) fn prefers_fast_input_poll(&self) -> bool {
        self.manual_session.is_some() && self.run_state == RunState::Running
    }

    pub(super) fn refresh_manual_snapshot(&mut self) {
        if let Some(session) = &mut self.manual_session {
            session.refresh_snapshot();
        }
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
                self.refresh_manual_snapshot();
            }
            WorkerEvent::Log(line) => {
                self.push_log(line);
            }
            WorkerEvent::Progress(progress) => {
                self.progress = Some(progress);
                self.run_state = RunState::Running;
                self.refresh_manual_snapshot();
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
            } else if let Some(session) = running_manual_session_mut(app) {
                session.resume_cruise();
                session.refresh_snapshot();
            }
        }
        KeyCode::Esc => {
            if app.worker.is_some() {
                app.request_cancel();
            }
        }
        KeyCode::Left => {
            if let Some(session) = running_manual_session_mut(app) {
                session.adjust_heading(-MANUAL_HEADING_STEP_DEG);
                session.refresh_snapshot();
            }
        }
        KeyCode::Right => {
            if let Some(session) = running_manual_session_mut(app) {
                session.adjust_heading(MANUAL_HEADING_STEP_DEG);
                session.refresh_snapshot();
            }
        }
        KeyCode::Up => {
            if let Some(session) = running_manual_session_mut(app) {
                session.adjust_speed(MANUAL_SPEED_STEP_MPS);
                session.refresh_snapshot();
            } else if app.tab == ActiveTab::Logs {
                app.log_scroll = app.log_scroll.saturating_add(1);
            }
        }
        KeyCode::Down => {
            if let Some(session) = running_manual_session_mut(app) {
                session.adjust_speed(-MANUAL_SPEED_STEP_MPS);
                session.refresh_snapshot();
            } else if app.tab == ActiveTab::Logs {
                app.log_scroll = app.log_scroll.saturating_sub(1);
            }
        }
        KeyCode::Char(' ') => {
            if let Some(session) = running_manual_session_mut(app) {
                session.stop();
                session.refresh_snapshot();
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
        (ActiveTab::Config, 'm') => {
            toggle_motion_source(app);
        }
        (ActiveTab::Config, 'l') => {
            begin_edit(app, EditField::ManualInitialLlh);
        }
        (ActiveTab::Config, 'j') => {
            begin_edit(app, EditField::ManualHeading);
        }
        (ActiveTab::Config, 'u') => {
            begin_edit(app, EditField::ManualCruiseSpeed);
        }
        (ActiveTab::Config, 'a') => {
            begin_edit(app, EditField::ManualAccelLimit);
        }
        (ActiveTab::Config, 't') => {
            begin_edit(app, EditField::ManualTurnRate);
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

fn running_manual_session_mut(
    app: &mut App,
) -> Option<&mut ManualControlSession> {
    if app.tab != ActiveTab::Run || app.run_state != RunState::Running {
        return None;
    }

    app.manual_session.as_mut()
}

fn start_run(app: &mut App) {
    if app.worker.is_some() {
        app.message = Some("already running".to_string());
        return;
    }

    if let Err(message) = app.config.validate_for_run() {
        app.message = Some(message);
        return;
    }

    let manual_session = if app.config.uses_manual_motion() {
        match ManualControlSession::from_config(&app.config.manual_motion) {
            Ok(session) => Some(session),
            Err(message) => {
                app.message = Some(message);
                return;
            }
        }
    } else {
        None
    };

    let config = app.config.clone();
    app.last_run = None;
    app.progress = None;
    app.run_state = RunState::Running;
    app.sinks_desc = describe_sinks(&config);
    app.manual_session = manual_session;
    app.message = None;

    let handle = spawn_worker(
        config,
        app.manual_session
            .as_ref()
            .map(|session| session.control.clone()),
        app.worker_events_tx.clone(),
    );
    app.worker = Some(handle);
    app.tab = ActiveTab::Run;
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
