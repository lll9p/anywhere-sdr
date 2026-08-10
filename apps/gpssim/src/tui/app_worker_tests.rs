use std::{
    io,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use super::*;
use crate::{
    cli::TxBackend,
    tui::{
        app::{ActiveTab, LastRun, RunState},
        worker::{Progress, WorkerEvent},
    },
    tui_config::{ManualMotionConfig, MotionSource, TuiConfig},
    utils::LogBuffer,
};

fn navigation_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../resources/brdc0010.22n")
}

fn valid_config() -> TuiConfig {
    TuiConfig {
        ephemerides: Some(navigation_path()),
        tx: vec![TxBackend::Null],
        ..TuiConfig::default()
    }
}

fn manual_config() -> TuiConfig {
    TuiConfig {
        motion_source: MotionSource::Manual,
        manual_motion: ManualMotionConfig {
            initial_llh: Some([35.0, 139.0, 10.0]),
            initial_heading_deg: 90.0,
            cruise_speed_mps: 4.0,
            accel_limit_mps2: 1.5,
            turn_rate_limit_dps: 30.0,
        },
        ..valid_config()
    }
}

fn new_app(config: TuiConfig) -> App {
    App::new(config, LogBuffer::new(64))
}

fn progress(blocks: u64) -> Progress {
    Progress {
        blocks,
        elapsed: Duration::from_millis(50),
        sim_seconds: 0.1,
        throughput_msps: 0.2,
        hackrf_underruns: None,
    }
}

fn synthetic_worker<F>(action: F) -> io::Result<WorkerHandle>
where
    F: FnOnce(mpsc::Sender<WorkerEvent>, Arc<AtomicBool>) + Send + 'static,
{
    let (event_tx, events) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_for_thread = cancel.clone();
    let join = thread::Builder::new().spawn(move || {
        action(event_tx, cancel_for_thread);
    })?;
    Ok(WorkerHandle {
        cancel,
        events,
        join,
        pending_terminal: None,
        events_disconnected: false,
    })
}

fn install_worker(app: &mut App, worker: WorkerHandle) {
    app.worker = Some(worker);
    app.run_state = RunState::Running;
}

fn drain_until_idle(app: &mut App) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(2);
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

fn send_event(event_tx: &mpsc::Sender<WorkerEvent>, event: WorkerEvent) {
    if event_tx.send(event).is_err() {
        tracing::debug!("test app dropped worker event receiver");
    }
}

fn signal(signal_tx: &mpsc::Sender<()>) {
    if signal_tx.send(()).is_err() {
        tracing::debug!("test signal receiver was dropped");
    }
}

fn assert_drain_nonblocking(
    app: &mut App, release_tx: &mpsc::Sender<()>,
) -> Result<(), String> {
    let (done_tx, done_rx) = mpsc::channel();
    let release_for_watchdog = release_tx.clone();
    let watchdog = thread::Builder::new()
        .spawn(move || {
            if done_rx.recv_timeout(Duration::from_secs(2)).is_err() {
                signal(&release_for_watchdog);
                true
            } else {
                false
            }
        })
        .map_err(|error| error.to_string())?;

    app.drain_worker_events();
    signal(&done_tx);
    let timed_out = watchdog
        .join()
        .map_err(|_| "nonblocking watchdog panicked".to_string())?;
    if timed_out {
        Err("worker drain blocked on an unfinished worker".to_string())
    } else {
        Ok(())
    }
}

#[test]
fn normal_terminal_event_is_applied_after_join() -> Result<(), String> {
    let mut app = new_app(valid_config());
    install_worker(
        &mut app,
        synthetic_worker(|event_tx, _| {
            send_event(&event_tx, WorkerEvent::Finished(progress(3)));
        })
        .map_err(|error| error.to_string())?,
    );

    drain_until_idle(&mut app)?;

    assert_eq!(app.run_state, RunState::Idle);
    assert!(matches!(app.last_run, Some(LastRun::Finished)));
    assert_eq!(app.progress.as_ref().map(|value| value.blocks), Some(3));
    Ok(())
}

#[test]
fn worker_error_event_preserves_existing_error_state() -> Result<(), String> {
    let mut app = new_app(valid_config());
    install_worker(
        &mut app,
        synthetic_worker(|event_tx, _| {
            send_event(
                &event_tx,
                WorkerEvent::Error(Error::tx_backend_msg(
                    "synthetic",
                    "run failure",
                )),
            );
        })
        .map_err(|error| error.to_string())?,
    );

    drain_until_idle(&mut app)?;

    assert!(matches!(
        &app.last_run,
        Some(LastRun::Error(Error::TxBackendMsg { backend, message }))
            if *backend == "synthetic" && message == "run failure"
    ));
    assert_eq!(app.run_state, RunState::Idle);
    Ok(())
}

#[test]
fn completion_without_terminal_event_unblocks_exit_and_restart()
-> Result<(), String> {
    let mut app = new_app(valid_config());
    install_worker(
        &mut app,
        synthetic_worker(|_, _| {}).map_err(|error| error.to_string())?,
    );
    app.request_exit();

    drain_until_idle(&mut app)?;

    assert!(app.should_exit());
    assert!(matches!(
        &app.last_run,
        Some(LastRun::Error(Error::WorkerExitedWithoutTerminalEvent))
    ));

    app.start_run_with_spawn(|_, _| {
        synthetic_worker(|event_tx, _| {
            send_event(&event_tx, WorkerEvent::Finished(progress(4)));
        })
    });
    assert!(app.worker.is_some());
    drain_until_idle(&mut app)?;
    assert!(matches!(app.last_run, Some(LastRun::Finished)));
    Ok(())
}

#[test]
fn disconnected_unfinished_worker_is_not_joined_or_removed()
-> Result<(), String> {
    let mut app = new_app(valid_config());
    let (disconnected_tx, disconnected_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    install_worker(
        &mut app,
        synthetic_worker(move |event_tx, _| {
            drop(event_tx);
            signal(&disconnected_tx);
            if release_rx.recv().is_err() {
                tracing::debug!("test release sender was dropped");
            }
        })
        .map_err(|error| error.to_string())?,
    );
    disconnected_rx
        .recv_timeout(Duration::from_secs(1))
        .map_err(|error| error.to_string())?;

    assert_drain_nonblocking(&mut app, &release_tx)?;

    let Some(worker) = app.worker.as_ref() else {
        return Err("disconnected unfinished worker was removed".to_string());
    };
    assert!(worker.events_disconnected);
    assert!(!worker.join.is_finished());

    signal(&release_tx);
    drain_until_idle(&mut app)?;
    assert!(matches!(app.last_run, Some(LastRun::Error(_))));
    Ok(())
}

#[test]
fn terminal_event_does_not_join_until_worker_finishes() -> Result<(), String> {
    let mut app = new_app(valid_config());
    let (event_sent_tx, event_sent_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    install_worker(
        &mut app,
        synthetic_worker(move |event_tx, _| {
            send_event(&event_tx, WorkerEvent::Finished(progress(5)));
            signal(&event_sent_tx);
            if release_rx.recv().is_err() {
                tracing::debug!("test release sender was dropped");
            }
        })
        .map_err(|error| error.to_string())?,
    );
    event_sent_rx
        .recv_timeout(Duration::from_secs(1))
        .map_err(|error| error.to_string())?;

    assert_drain_nonblocking(&mut app, &release_tx)?;

    let Some(worker) = app.worker.as_ref() else {
        return Err("unfinished worker was removed after terminal event".into());
    };
    assert!(worker.pending_terminal.is_some());
    assert!(!worker.join.is_finished());
    assert!(app.last_run.is_none());

    signal(&release_tx);
    drain_until_idle(&mut app)?;
    assert!(matches!(app.last_run, Some(LastRun::Finished)));
    Ok(())
}

#[test]
fn post_join_drain_captures_final_event() -> Result<(), String> {
    let mut app = new_app(valid_config());
    let worker = synthetic_worker(|event_tx, _| {
        send_event(&event_tx, WorkerEvent::Finished(progress(6)));
    })
    .map_err(|error| error.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(1);
    while !worker.join.is_finished() && Instant::now() < deadline {
        thread::yield_now();
    }
    if !worker.join.is_finished() {
        return Err("synthetic worker did not finish".to_string());
    }

    app.reap_finished_worker(worker);

    assert!(matches!(app.last_run, Some(LastRun::Finished)));
    assert_eq!(app.progress.as_ref().map(|value| value.blocks), Some(6));
    Ok(())
}

#[cfg(panic = "unwind")]
#[test]
fn panic_without_terminal_event_is_reaped() -> Result<(), String> {
    let mut app = new_app(manual_config());
    app.manual_session = Some(
        ManualControlSession::from_config(&app.config.manual_motion)
            .map_err(|error| error.to_string())?,
    );
    install_worker(
        &mut app,
        synthetic_worker(|_, _| panic!("synthetic worker panic"))
            .map_err(|error| error.to_string())?,
    );
    app.request_exit();

    drain_until_idle(&mut app)?;

    assert!(matches!(
        &app.last_run,
        Some(LastRun::Error(Error::WorkerPanicked { message }))
            if message == "synthetic worker panic"
    ));
    assert_eq!(app.run_state, RunState::Idle);
    assert!(app.worker.is_none());
    assert!(app.should_exit());
    assert!(app.manual_session.is_none());
    Ok(())
}

#[cfg(panic = "unwind")]
#[test]
fn panic_overrides_optimistic_terminal_event() -> Result<(), String> {
    let mut app = new_app(valid_config());
    let (event_sent_tx, event_sent_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    install_worker(
        &mut app,
        synthetic_worker(move |event_tx, _| {
            send_event(&event_tx, WorkerEvent::Finished(progress(7)));
            signal(&event_sent_tx);
            if release_rx.recv().is_err() {
                return;
            }
            panic!("panic after terminal event");
        })
        .map_err(|error| error.to_string())?,
    );
    event_sent_rx
        .recv_timeout(Duration::from_secs(1))
        .map_err(|error| error.to_string())?;
    app.drain_worker_events();
    assert!(app.worker.is_some());
    assert!(app.last_run.is_none());

    signal(&release_tx);
    drain_until_idle(&mut app)?;

    assert!(matches!(
        &app.last_run,
        Some(LastRun::Error(Error::WorkerPanicked { message }))
            if message == "panic after terminal event"
    ));
    assert_eq!(app.run_state, RunState::Idle);
    assert!(app.worker.is_none());

    app.start_run_with_spawn(|_, _| {
        synthetic_worker(|event_tx, _| {
            send_event(&event_tx, WorkerEvent::Finished(progress(8)));
        })
    });
    assert!(app.worker.is_some());
    drain_until_idle(&mut app)?;
    assert!(matches!(app.last_run, Some(LastRun::Finished)));
    Ok(())
}

#[test]
fn exit_request_reaps_finished_cancelled_worker_without_event()
-> Result<(), String> {
    let mut app = new_app(valid_config());
    install_worker(
        &mut app,
        synthetic_worker(|_, cancel| {
            while !cancel.load(Ordering::Relaxed) {
                thread::yield_now();
            }
        })
        .map_err(|error| error.to_string())?,
    );

    app.request_exit();
    drain_until_idle(&mut app)?;

    assert!(app.should_exit());
    assert!(matches!(app.last_run, Some(LastRun::Error(_))));
    Ok(())
}

#[test]
fn old_worker_events_cannot_leak_into_a_later_run() -> Result<(), String> {
    let mut app = new_app(valid_config());
    let (old_event_tx, old_events) = mpsc::channel();
    let stale_event_tx = old_event_tx.clone();
    let first_join = thread::Builder::new()
        .spawn(move || drop(old_event_tx))
        .map_err(|error| error.to_string())?;
    install_worker(&mut app, WorkerHandle {
        cancel: Arc::new(AtomicBool::new(false)),
        events: old_events,
        join: first_join,
        pending_terminal: None,
        events_disconnected: false,
    });
    drain_until_idle(&mut app)?;

    let (release_tx, release_rx) = mpsc::channel();
    let stale_sender = thread::Builder::new()
        .spawn(move || {
            if release_rx.recv().is_err() {
                return false;
            }
            stale_event_tx
                .send(WorkerEvent::Error(Error::msg("stale event")))
                .is_err()
        })
        .map_err(|error| error.to_string())?;
    app.start_run_with_spawn(|_, _| {
        synthetic_worker(|event_tx, _| {
            send_event(&event_tx, WorkerEvent::Finished(progress(10)));
        })
    });
    signal(&release_tx);
    let stale_rejected = stale_sender
        .join()
        .map_err(|_| "stale sender thread panicked".to_string())?;
    assert!(stale_rejected);

    drain_until_idle(&mut app)?;
    assert!(matches!(app.last_run, Some(LastRun::Finished)));
    assert_eq!(app.progress.as_ref().map(|value| value.blocks), Some(10));
    Ok(())
}

#[test]
fn spawn_failure_preserves_prior_app_state() -> Result<(), String> {
    let mut app = new_app(manual_config());
    app.tab = ActiveTab::Logs;
    app.last_run = Some(LastRun::Finished);
    app.progress = Some(progress(9));
    app.run_state = RunState::Idle;
    app.sinks_desc = "previous sinks".to_string();
    app.manual_session = Some(
        ManualControlSession::from_config(&app.config.manual_motion)
            .map_err(|error| error.to_string())?,
    );

    app.start_run_with_spawn(|_, _| {
        Err(io::Error::other("injected spawn failure"))
    });

    assert!(app.worker.is_none());
    assert_eq!(app.tab, ActiveTab::Logs);
    assert_eq!(app.run_state, RunState::Idle);
    assert!(matches!(app.last_run, Some(LastRun::Finished)));
    assert_eq!(app.progress.as_ref().map(|value| value.blocks), Some(9));
    assert_eq!(app.sinks_desc, "previous sinks");
    assert_eq!(
        app.manual_session
            .as_ref()
            .map(|session| session.target_heading_deg),
        Some(90.0)
    );
    assert_eq!(
        app.message.as_deref(),
        Some("failed to start worker: injected spawn failure")
    );
    Ok(())
}
