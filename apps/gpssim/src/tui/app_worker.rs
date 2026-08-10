use std::{any::Any, io, sync::mpsc};

use gps::RuntimeMotionControl;

use super::{
    app::{ActiveTab, App, LastRun, RunState},
    manual_control::ManualControlSession,
    worker::{
        PendingTerminalOutcome, WorkerEvent, WorkerHandle, describe_sinks,
        spawn_worker,
    },
};
use crate::{Error, tui_config::TuiConfig};

const NON_STRING_PANIC_PAYLOAD: &str = "non-string panic payload";

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else {
        NON_STRING_PANIC_PAYLOAD.to_string()
    }
}

impl App {
    pub(super) fn drain_worker_events(&mut self) {
        let Some(mut worker) = self.worker.take() else {
            return;
        };

        if !worker.events_disconnected {
            self.drain_worker_channel(
                &worker.events,
                &mut worker.pending_terminal,
                &mut worker.events_disconnected,
            );
        }

        if !worker.join.is_finished() {
            self.worker = Some(worker);
            return;
        }

        self.reap_finished_worker(worker);
    }

    pub(super) fn start_run(&mut self) {
        self.start_run_with_spawn(spawn_worker);
    }

    pub(super) fn start_run_with_spawn<F>(&mut self, spawn: F)
    where
        F: FnOnce(
            TuiConfig,
            Option<RuntimeMotionControl>,
        ) -> io::Result<WorkerHandle>,
    {
        if self.worker.is_some() {
            self.message = Some("already running".to_string());
            return;
        }

        if let Err(message) = self.config.validate_for_run() {
            self.message = Some(message);
            return;
        }

        let manual_session = if self.config.uses_manual_motion() {
            match ManualControlSession::from_config(&self.config.manual_motion)
            {
                Ok(session) => Some(session),
                Err(error) => {
                    self.message = Some(error.to_string());
                    return;
                }
            }
        } else {
            None
        };

        let config = self.config.clone();
        let manual_control = manual_session
            .as_ref()
            .map(|session| session.control.clone());
        let sinks_desc = describe_sinks(&config);
        let worker = match spawn(config, manual_control) {
            Ok(worker) => worker,
            Err(error) => {
                self.message = Some(format!("failed to start worker: {error}"));
                return;
            }
        };

        self.last_run = None;
        self.progress = None;
        self.run_state = RunState::Running;
        self.sinks_desc = sinks_desc;
        self.manual_session = manual_session;
        self.worker = Some(worker);
        self.message = None;
        self.tab = ActiveTab::Run;
    }

    fn drain_worker_channel(
        &mut self, events: &mpsc::Receiver<WorkerEvent>,
        pending_terminal: &mut Option<PendingTerminalOutcome>,
        events_disconnected: &mut bool,
    ) {
        loop {
            match events.try_recv() {
                Ok(event) => self.handle_worker_event(pending_terminal, event),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    *events_disconnected = true;
                    break;
                }
            }
        }
    }

    fn handle_worker_event(
        &mut self, pending_terminal: &mut Option<PendingTerminalOutcome>,
        event: WorkerEvent,
    ) {
        match event {
            WorkerEvent::Started { sinks } => {
                if pending_terminal.is_none() {
                    self.sinks_desc = sinks;
                    self.push_log("run started");
                    self.run_state = RunState::Running;
                    self.refresh_manual_snapshot();
                } else {
                    self.push_log("worker sent started after terminal event");
                }
            }
            WorkerEvent::Log(line) => self.push_log(line),
            WorkerEvent::Progress(progress) => {
                if pending_terminal.is_none() {
                    self.progress = Some(progress);
                    self.run_state = RunState::Running;
                    self.refresh_manual_snapshot();
                } else {
                    self.push_log("worker sent progress after terminal event");
                }
            }
            WorkerEvent::Finished(progress) => self.record_terminal_outcome(
                pending_terminal,
                PendingTerminalOutcome::Finished(progress),
            ),
            WorkerEvent::Cancelled(progress) => self.record_terminal_outcome(
                pending_terminal,
                PendingTerminalOutcome::Cancelled(progress),
            ),
            WorkerEvent::Error(error) => self.record_terminal_outcome(
                pending_terminal,
                PendingTerminalOutcome::Error(error),
            ),
        }
    }

    fn record_terminal_outcome(
        &mut self, pending_terminal: &mut Option<PendingTerminalOutcome>,
        outcome: PendingTerminalOutcome,
    ) {
        if pending_terminal.is_none() {
            *pending_terminal = Some(outcome);
            self.manual_session = None;
            self.run_state = RunState::Stopping;
        } else {
            self.push_log("worker sent multiple terminal events");
        }
    }

    fn reap_finished_worker(&mut self, worker: WorkerHandle) {
        self.manual_session = None;
        let WorkerHandle {
            events,
            join,
            mut pending_terminal,
            mut events_disconnected,
            ..
        } = worker;
        let join_result = join.join();
        self.drain_worker_channel(
            &events,
            &mut pending_terminal,
            &mut events_disconnected,
        );

        match join_result {
            Ok(()) => self.apply_terminal_outcome(pending_terminal),
            Err(payload) => self.record_run_error(Error::WorkerPanicked {
                message: panic_message(payload),
            }),
        }
        self.run_state = RunState::Idle;
    }

    fn apply_terminal_outcome(
        &mut self, pending_terminal: Option<PendingTerminalOutcome>,
    ) {
        match pending_terminal {
            Some(PendingTerminalOutcome::Finished(progress)) => {
                self.progress = Some(progress);
                self.last_run = Some(LastRun::Finished);
                self.push_log("run finished");
            }
            Some(PendingTerminalOutcome::Cancelled(progress)) => {
                self.progress = Some(progress);
                self.last_run = Some(LastRun::Cancelled);
                self.push_log("run cancelled");
            }
            Some(PendingTerminalOutcome::Error(error)) => {
                self.record_run_error(error);
            }
            None => {
                self.record_run_error(Error::WorkerExitedWithoutTerminalEvent);
            }
        }
    }

    fn record_run_error(&mut self, error: Error) {
        self.push_log(format!("run error: {error}"));
        self.last_run = Some(LastRun::Error(error));
    }
}

#[cfg(test)]
#[path = "app_worker_session_tests.rs"]
mod session_tests;
#[cfg(test)]
#[path = "app_worker_tests.rs"]
mod tests;
