use std::{
    io,
    sync::{Arc, atomic::AtomicBool, mpsc},
    thread,
    time::{Duration, Instant},
};

use super::*;
use crate::{
    tui::{
        app::{LastRun, RunState},
        manual_control::ManualControlSession,
        worker::{Progress, WorkerEvent, WorkerHandle},
    },
    tui_config::{ManualMotionConfig, MotionSource, TuiConfig},
    utils::LogBuffer,
};

const TEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug)]
enum TerminalCase {
    Finished,
    Cancelled,
    Error,
}

impl TerminalCase {
    fn event(self) -> WorkerEvent {
        match self {
            Self::Finished => WorkerEvent::Finished(progress()),
            Self::Cancelled => WorkerEvent::Cancelled(progress()),
            Self::Error => WorkerEvent::Error(Error::tx_backend_msg(
                "synthetic",
                "terminal failure",
            )),
        }
    }

    fn assert_last_run(self, app: &App) {
        match self {
            Self::Finished => {
                assert!(matches!(app.last_run, Some(LastRun::Finished)));
            }
            Self::Cancelled => {
                assert!(matches!(app.last_run, Some(LastRun::Cancelled)));
            }
            Self::Error => assert!(matches!(
                &app.last_run,
                Some(LastRun::Error(Error::TxBackendMsg {
                    backend,
                    message,
                })) if *backend == "synthetic" && message == "terminal failure"
            )),
        }
    }
}

fn progress() -> Progress {
    Progress {
        blocks: 5,
        elapsed: Duration::from_millis(50),
        sim_seconds: 0.1,
        throughput_msps: 0.2,
        hackrf_underruns: None,
    }
}

fn manual_app() -> Result<App, String> {
    let config = TuiConfig {
        motion_source: MotionSource::Manual,
        manual_motion: ManualMotionConfig {
            initial_llh: Some([35.0, 139.0, 10.0]),
            initial_heading_deg: 90.0,
            cruise_speed_mps: 4.0,
            accel_limit_mps2: 1.5,
            turn_rate_limit_dps: 30.0,
        },
        ..TuiConfig::default()
    };
    let manual_session =
        ManualControlSession::from_config(&config.manual_motion)
            .map_err(|error| error.to_string())?;
    let mut app = App::new(config, LogBuffer::new(64));
    app.manual_session = Some(manual_session);
    app.run_state = RunState::Running;
    Ok(app)
}

fn blocked_worker(
    worker_events: Vec<WorkerEvent>,
) -> io::Result<(WorkerHandle, mpsc::Receiver<()>, mpsc::Sender<()>)> {
    let (event_tx, events) = mpsc::channel();
    let (event_sent_tx, event_sent_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let join = thread::Builder::new().spawn(move || {
        for event in worker_events {
            if event_tx.send(event).is_err() {
                tracing::debug!("test app dropped worker event receiver");
                break;
            }
        }
        if event_sent_tx.send(()).is_err() {
            tracing::debug!("test event signal receiver was dropped");
        }
        if release_rx.recv().is_err() {
            tracing::debug!("test release sender was dropped");
        }
    })?;
    Ok((
        WorkerHandle {
            cancel: Arc::new(AtomicBool::new(false)),
            events,
            join,
            pending_terminal: None,
            events_disconnected: false,
        },
        event_sent_rx,
        release_tx,
    ))
}

fn drain_nonblocking(
    app: &mut App, release_tx: &mpsc::Sender<()>,
) -> Result<(), String> {
    let (done_tx, done_rx) = mpsc::channel();
    let release_for_watchdog = release_tx.clone();
    let watchdog = thread::Builder::new()
        .spawn(move || {
            let timed_out = done_rx.recv_timeout(TEST_TIMEOUT).is_err();
            if timed_out && release_for_watchdog.send(()).is_err() {
                tracing::debug!("test release receiver was dropped");
            }
            timed_out
        })
        .map_err(|error| error.to_string())?;

    app.drain_worker_events();
    if done_tx.send(()).is_err() {
        tracing::debug!("test watchdog receiver was dropped");
    }
    let timed_out = watchdog
        .join()
        .map_err(|_| "nonblocking watchdog panicked".to_string())?;
    if timed_out {
        Err("worker drain blocked before join became safe".to_string())
    } else {
        Ok(())
    }
}

fn drain_until_idle(app: &mut App) -> Result<(), String> {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while app.worker.is_some() && Instant::now() < deadline {
        app.drain_worker_events();
        thread::yield_now();
    }
    if app.worker.is_some() {
        Err("worker was not reaped before deadline".to_string())
    } else {
        Ok(())
    }
}

#[test]
fn every_terminal_event_clears_manual_state_before_join() -> Result<(), String>
{
    for case in [
        TerminalCase::Finished,
        TerminalCase::Cancelled,
        TerminalCase::Error,
    ] {
        let mut app = manual_app()?;
        let (worker, event_sent_rx, release_tx) =
            blocked_worker(vec![case.event()])
                .map_err(|error| error.to_string())?;
        app.worker = Some(worker);
        event_sent_rx
            .recv_timeout(TEST_TIMEOUT)
            .map_err(|error| error.to_string())?;

        drain_nonblocking(&mut app, &release_tx)?;

        let worker = app.worker.as_ref().ok_or_else(|| {
            format!("{case:?} worker was joined before it finished")
        })?;
        assert!(!worker.join.is_finished());
        assert!(worker.pending_terminal.is_some());
        assert!(app.manual_session.is_none());
        assert_eq!(app.run_state, RunState::Stopping);
        assert!(app.last_run.is_none());

        release_tx.send(()).map_err(|error| error.to_string())?;
        drain_until_idle(&mut app)?;
        assert_eq!(app.run_state, RunState::Idle);
        assert!(app.manual_session.is_none());
        case.assert_last_run(&app);
    }
    Ok(())
}

#[test]
fn events_after_terminal_cannot_restore_live_state() -> Result<(), String> {
    let mut app = manual_app()?;
    app.sinks_desc = "initial sinks".to_string();
    let mut late_progress = progress();
    late_progress.blocks = 99;
    let (worker, event_sent_rx, release_tx) = blocked_worker(vec![
        WorkerEvent::Finished(progress()),
        WorkerEvent::Started {
            sinks: "late sinks".to_string(),
        },
        WorkerEvent::Progress(late_progress),
    ])
    .map_err(|error| error.to_string())?;
    app.worker = Some(worker);
    event_sent_rx
        .recv_timeout(TEST_TIMEOUT)
        .map_err(|error| error.to_string())?;

    drain_nonblocking(&mut app, &release_tx)?;

    assert_eq!(app.run_state, RunState::Stopping);
    assert!(app.manual_session.is_none());
    assert_eq!(app.sinks_desc, "initial sinks");
    assert!(app.progress.is_none());
    assert!(!app.prefers_fast_input_poll());
    let logs = app.log_buffer.snapshot();
    assert!(
        logs.iter()
            .any(|line| line.contains("started after terminal event"))
    );
    assert!(
        logs.iter()
            .any(|line| line.contains("progress after terminal event"))
    );

    release_tx.send(()).map_err(|error| error.to_string())?;
    drain_until_idle(&mut app)?;
    assert!(matches!(app.last_run, Some(LastRun::Finished)));
    assert_eq!(app.progress.as_ref().map(|value| value.blocks), Some(5));
    Ok(())
}

#[test]
fn missing_terminal_event_clears_manual_state_during_reap() -> Result<(), String>
{
    let mut app = manual_app()?;
    let (event_tx, events) = mpsc::channel::<WorkerEvent>();
    let join = thread::Builder::new()
        .spawn(move || drop(event_tx))
        .map_err(|error| error.to_string())?;
    app.worker = Some(WorkerHandle {
        cancel: Arc::new(AtomicBool::new(false)),
        events,
        join,
        pending_terminal: None,
        events_disconnected: false,
    });

    drain_until_idle(&mut app)?;

    assert!(app.manual_session.is_none());
    assert!(matches!(
        app.last_run,
        Some(LastRun::Error(Error::WorkerExitedWithoutTerminalEvent))
    ));
    Ok(())
}
