use std::sync::atomic::Ordering;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::{
    config_edit::{
        EditField, begin_edit, handle_edit_key, toggle_motion_source,
        toggle_tx_backend_and_update_status,
    },
    manual_control::{
        MANUAL_HEADING_STEP_DEG, MANUAL_SPEED_STEP_MPS, ManualControlSession,
    },
    worker::{Progress, WorkerHandle, describe_sinks},
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

    pub(super) worker: Option<WorkerHandle>,
}

impl App {
    pub(super) fn new(config: TuiConfig, log_buffer: LogBuffer) -> Self {
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
        }
    }

    pub(super) fn should_exit(&self) -> bool {
        self.exit_requested && self.worker.is_none()
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

    pub(super) fn request_exit(&mut self) {
        self.exit_requested = true;
        self.request_cancel();
    }

    pub(super) fn request_cancel(&mut self) {
        if let Some(worker) = &self.worker {
            worker.cancel.store(true, Ordering::Relaxed);
            self.run_state = RunState::Stopping;
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
                app.start_run();
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

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
