use std::{io, sync::mpsc};

use gps::RuntimeMotionControl;

use super::{
    app::{ActiveTab, App, LastRun, RunState},
    manual_control::ManualControlSession,
    worker::{
        PendingTerminalOutcome, WorkerEvent, WorkerHandle, describe_sinks,
        spawn_worker,
    },
};
use crate::tui_config::TuiConfig;

const WORKER_PANICKED: &str = "worker thread panicked";
const WORKER_MISSING_TERMINAL: &str = "worker exited without a terminal event";

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
                self.sinks_desc = sinks;
                self.push_log("run started");
                self.run_state = RunState::Running;
                self.refresh_manual_snapshot();
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
            WorkerEvent::Error(message) => self.record_terminal_outcome(
                pending_terminal,
                PendingTerminalOutcome::Error(message),
            ),
        }
    }

    fn record_terminal_outcome(
        &mut self, pending_terminal: &mut Option<PendingTerminalOutcome>,
        outcome: PendingTerminalOutcome,
    ) {
        if pending_terminal.is_none() {
            *pending_terminal = Some(outcome);
        } else {
            self.push_log("worker sent multiple terminal events");
        }
    }

    fn reap_finished_worker(&mut self, worker: WorkerHandle) {
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

        if join_result.is_err() {
            self.last_run = Some(LastRun::Error(WORKER_PANICKED.to_string()));
            self.push_log(WORKER_PANICKED);
        } else {
            self.apply_terminal_outcome(pending_terminal);
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
            Some(PendingTerminalOutcome::Error(message)) => {
                self.last_run = Some(LastRun::Error(message.clone()));
                self.push_log(format!("run error: {message}"));
            }
            None => {
                self.last_run =
                    Some(LastRun::Error(WORKER_MISSING_TERMINAL.to_string()));
                self.push_log(format!("run error: {WORKER_MISSING_TERMINAL}"));
            }
        }
    }
}

#[cfg(test)]
#[path = "app_worker_tests.rs"]
mod tests;
