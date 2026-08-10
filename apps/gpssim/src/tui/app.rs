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
use crate::{Error, cli::TxBackend, tui_config::TuiConfig, utils::LogBuffer};

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

#[derive(Debug)]
pub(super) enum LastRun {
    Finished,
    Cancelled,
    Error(Error),
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
    pub(super) config_scroll: u16,
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
            config_scroll: 0,
            log_scroll: 0,
            manual_session: None,
            worker: None,
        }
    }

    pub(super) fn should_exit(&self) -> bool {
        self.exit_requested && self.worker.is_none()
    }

    pub(super) fn take_session_result(&mut self) -> Result<(), Error> {
        match self.last_run.take() {
            Some(LastRun::Error(error)) => Err(error),
            Some(LastRun::Finished | LastRun::Cancelled) | None => Ok(()),
        }
    }

    pub(super) fn push_log(&mut self, line: impl Into<String>) {
        self.log_buffer.push_line(line);
    }

    pub(super) fn prefers_fast_input_poll(&self) -> bool {
        self.live_manual_session().is_some()
    }

    pub(super) fn live_manual_session(&self) -> Option<&ManualControlSession> {
        if self.run_state == RunState::Running {
            self.manual_session.as_ref()
        } else {
            None
        }
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
            } else {
                apply_manual_action(app, ManualControlSession::resume_cruise);
            }
        }
        KeyCode::Esc => {
            if app.worker.is_some() {
                app.request_cancel();
            }
        }
        KeyCode::Left => {
            apply_manual_action(app, |session| {
                session.adjust_heading(-MANUAL_HEADING_STEP_DEG)
            });
        }
        KeyCode::Right => {
            apply_manual_action(app, |session| {
                session.adjust_heading(MANUAL_HEADING_STEP_DEG)
            });
        }
        KeyCode::Up => {
            if !apply_manual_action(app, |session| {
                session.adjust_speed(MANUAL_SPEED_STEP_MPS)
            }) {
                match app.tab {
                    ActiveTab::Config => {
                        app.config_scroll = app.config_scroll.saturating_add(1);
                    }
                    ActiveTab::Logs => {
                        app.log_scroll = app.log_scroll.saturating_add(1);
                    }
                    ActiveTab::Run => {}
                }
            }
        }
        KeyCode::Down => {
            if !apply_manual_action(app, |session| {
                session.adjust_speed(-MANUAL_SPEED_STEP_MPS)
            }) {
                match app.tab {
                    ActiveTab::Config => {
                        app.config_scroll = app.config_scroll.saturating_sub(1);
                    }
                    ActiveTab::Logs => {
                        app.log_scroll = app.log_scroll.saturating_sub(1);
                    }
                    ActiveTab::Run => {}
                }
            }
        }
        KeyCode::Char(' ') => {
            apply_manual_action(app, ManualControlSession::stop);
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
        (ActiveTab::Config, 'c') => {
            app.config.clear_preconfigured_sources();
            app.message = Some(
                "cleared CLI-prefilled motion and location sources".to_string(),
            );
        }
        (ActiveTab::Run, 'c') => {
            app.request_cancel();
        }
        _ => {}
    }
}

fn apply_manual_action<F>(app: &mut App, action: F) -> bool
where
    F: FnOnce(&mut ManualControlSession) -> Result<(), Error>,
{
    let result = {
        let Some(session) = running_manual_session_mut(app) else {
            return false;
        };
        let result = action(session);
        if result.is_ok() {
            session.refresh_snapshot();
        }
        result
    };

    if let Err(error) = result {
        let message = format!("manual motion command rejected: {error}");
        app.message = Some(message.clone());
        app.push_log(message);
    }
    true
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
