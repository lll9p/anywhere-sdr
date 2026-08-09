use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use crate::{
    cli::TxBackend,
    tui::{
        manual_control::ManualControlSession,
        worker::{Progress, WorkerEvent, WorkerHandle},
    },
    tui_config::{ManualMotionConfig, MotionSource, TuiConfig},
    utils::LogBuffer,
};

fn manual_config() -> TuiConfig {
    TuiConfig {
        ephemerides: Some(navigation_path()),
        tx: vec![TxBackend::Null],
        motion_source: MotionSource::Manual,
        manual_motion: ManualMotionConfig {
            initial_llh: Some([35.681_298, 139.766_247, 10.0]),
            initial_heading_deg: 90.0,
            cruise_speed_mps: 4.0,
            accel_limit_mps2: 1.5,
            turn_rate_limit_dps: 30.0,
        },
        ..TuiConfig::default()
    }
}

fn navigation_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../resources/brdc0010.22n")
}

fn new_app(config: TuiConfig) -> App {
    App::new(config, LogBuffer::new(64))
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn manual_mode_requires_initial_position_before_start() {
    let mut app = new_app(manual_config());
    app.config.manual_motion.initial_llh = None;

    app.start_run();

    assert_eq!(
        app.message.as_deref(),
        Some("manual mode requires initial LLH position")
    );
    assert!(app.worker.is_none());
}

#[test]
fn manual_mode_rejects_motion_file_conflicts_before_start() {
    let mut app = new_app(manual_config());
    app.config.user_motion_llh = Some(PathBuf::from("motion.csv"));

    app.start_run();

    assert_eq!(
        app.message.as_deref(),
        Some(
            "manual mode cannot be combined with user_motion_ecef, \
             user_motion_llh, or nmea_gga"
        )
    );
    assert!(app.worker.is_none());
}

#[test]
fn manual_mode_preserves_geometry_error_for_invalid_initial_llh() {
    let mut config = manual_config();
    config.manual_motion.initial_llh = Some([91.0, 0.0, 0.0]);
    assert!(matches!(
        ManualControlSession::from_config(&config.manual_motion),
        Err(crate::Error::Geometry(
            geometry::Error::InvalidCoordinates { .. }
        ))
    ));

    let mut app = new_app(config);
    app.start_run();
    assert!(app.message.as_deref().is_some_and(|message| {
        message.contains("Invalid geodetic coordinates")
    }));
    assert!(app.worker.is_none());
}

#[test]
fn manual_session_propagates_initial_submit_error() {
    let mut config = manual_config();
    config.manual_motion.initial_heading_deg = f64::NAN;
    assert!(matches!(
        ManualControlSession::from_config(&config.manual_motion),
        Err(crate::Error::Gps(
            gps::Error::NonFiniteMotionCommandValue {
                command: "SetHeadingSpeed",
                field: "heading_deg",
                value,
            }
        )) if value.is_nan()
    ));
}

#[test]
fn manual_mode_hotkeys_update_targets() -> Result<(), String> {
    let mut app = new_app(manual_config());
    app.tab = ActiveTab::Run;
    app.run_state = RunState::Running;
    app.manual_session = Some(
        ManualControlSession::from_config(&app.config.manual_motion)
            .map_err(|error| error.to_string())?,
    );

    handle_key_event(&mut app, key(KeyCode::Right));
    handle_key_event(&mut app, key(KeyCode::Up));
    handle_key_event(&mut app, key(KeyCode::Char(' ')));
    handle_key_event(&mut app, key(KeyCode::Enter));

    let session = app
        .manual_session
        .as_ref()
        .ok_or_else(|| "manual session remains active".to_string())?;
    assert_close(session.target_heading_deg, 95.0);
    assert_close(session.target_speed_mps, 5.0);
    assert_close(session.cruise_speed_mps, 5.0);
    Ok(())
}

#[test]
fn rejected_manual_command_preserves_session_worker_and_last_run()
-> Result<(), String> {
    let mut app = new_app(manual_config());
    app.tab = ActiveTab::Run;
    app.run_state = RunState::Running;
    app.last_run = Some(LastRun::Finished);
    let mut session =
        ManualControlSession::from_config(&app.config.manual_motion)
            .map_err(|error| error.to_string())?;
    session.turn_rate_limit_dps = f64::NAN;
    app.manual_session = Some(session);

    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_for_thread = cancel.clone();
    let (event_tx, events) = mpsc::channel();
    let join = thread::Builder::new()
        .spawn(move || {
            let _event_tx = event_tx;
            while !cancel_for_thread.load(Ordering::Relaxed) {
                thread::yield_now();
            }
        })
        .map_err(|error| error.to_string())?;
    app.worker = Some(WorkerHandle {
        cancel: cancel.clone(),
        events,
        join,
        pending_terminal: None,
        events_disconnected: false,
    });

    handle_key_event(&mut app, key(KeyCode::Right));

    let session = app
        .manual_session
        .as_ref()
        .ok_or_else(|| "manual session remains active".to_string())?;
    assert_close(session.target_heading_deg, 90.0);
    assert!(app.worker.is_some());
    assert!(!cancel.load(Ordering::Relaxed));
    assert_eq!(app.run_state, RunState::Running);
    assert!(matches!(app.last_run, Some(LastRun::Finished)));
    assert!(app.message.as_deref().is_some_and(|message| {
        message.contains("SetTargetHeading")
            && message.contains("turn_rate_limit_dps")
    }));
    assert!(app.log_buffer.snapshot().iter().any(|line| {
        line.contains("manual motion command rejected")
            && line.contains("turn_rate_limit_dps")
    }));

    cancel.store(true, Ordering::Relaxed);
    let worker = app
        .worker
        .take()
        .ok_or_else(|| "worker remains active".to_string())?;
    worker
        .join
        .join()
        .map_err(|_| "manual test worker panicked".to_string())?;
    Ok(())
}

#[test]
fn cancel_event_resets_state_and_joins_worker() -> Result<(), String> {
    let mut app = new_app(TuiConfig::default());
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_for_thread = cancel.clone();
    let (event_tx, events) = mpsc::channel();
    let join = thread::Builder::new()
        .spawn(move || {
            while !cancel_for_thread.load(Ordering::Relaxed) {
                thread::yield_now();
            }
            if event_tx
                .send(WorkerEvent::Cancelled(Progress {
                    blocks: 1,
                    elapsed: Duration::from_millis(50),
                    sim_seconds: 0.1,
                    throughput_msps: 0.2,
                    hackrf_underruns: None,
                }))
                .is_err()
            {
                tracing::debug!("test app dropped worker event receiver");
            }
        })
        .map_err(|error| error.to_string())?;
    app.worker = Some(WorkerHandle {
        cancel: cancel.clone(),
        events,
        join,
        pending_terminal: None,
        events_disconnected: false,
    });
    app.run_state = RunState::Running;

    app.request_cancel();

    assert_eq!(app.run_state, RunState::Stopping);
    assert!(cancel.load(Ordering::Relaxed));

    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while app.worker.is_some() && std::time::Instant::now() < deadline {
        app.drain_worker_events();
        thread::yield_now();
    }

    assert_eq!(app.run_state, RunState::Idle);
    assert!(app.worker.is_none());
    assert!(matches!(app.last_run, Some(LastRun::Cancelled)));
    Ok(())
}
