use std::{
    error::Error as StdError,
    path::PathBuf,
    sync::{atomic::Ordering, mpsc},
    thread,
    time::{Duration, Instant},
};

use super::*;
use crate::{
    cli::TxBackend,
    tui::manual_control::ManualControlSession,
    tui_config::{ManualMotionConfig, MotionSource, TuiConfig},
};

fn manual_config() -> TuiConfig {
    TuiConfig {
        ephemerides: Some(navigation_path()),
        tx: vec![TxBackend::Null],
        motion_source: MotionSource::Manual,
        duration: Some(0.05),
        manual_motion: ManualMotionConfig {
            initial_llh: Some([35.681_298, 139.766_247, 10.0]),
            initial_heading_deg: 45.0,
            cruise_speed_mps: 3.0,
            accel_limit_mps2: 1.0,
            turn_rate_limit_dps: 25.0,
        },
        ..TuiConfig::default()
    }
}

fn navigation_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../resources/brdc0010.22n")
}

fn terminal_progress() -> Progress {
    compute_progress(Instant::now(), 1, 100_000, 1_000_000.0, None)
}

#[test]
fn progress_uses_actual_emitted_samples_for_fractional_blocks() {
    let progress =
        compute_progress(Instant::now(), 2, 150_000, 1_000_000.0, None);
    assert!((progress.sim_seconds - 0.15).abs() < f64::EPSILON);
}

#[test]
fn tui_generator_preserves_coordinate_cardinality_errors() -> Result<(), Error>
{
    for actual in [0, 1, 2, 4] {
        let mut config = TuiConfig {
            ephemerides: Some(navigation_path()),
            location: Some(vec![0.0; actual]),
            ..TuiConfig::default()
        };
        let Err(error) = build_generator(&config, None) else {
            return Err(Error::msg(
                "invalid LLH coordinate count was accepted",
            ));
        };
        assert!(matches!(
            error,
            Error::Gps(gps::Error::InvalidCoordinateCount { actual: count })
                if count == actual
        ));

        config.location = None;
        config.location_ecef = Some(vec![0.0; actual]);
        let Err(error) = build_generator(&config, None) else {
            return Err(Error::msg(
                "invalid ECEF coordinate count was accepted",
            ));
        };
        assert!(matches!(
            error,
            Error::Gps(gps::Error::InvalidCoordinateCount { actual: count })
                if count == actual
        ));
    }
    Ok(())
}

#[test]
fn manual_mode_streaming_runs_without_new_input_until_cancelled()
-> Result<(), String> {
    let config = manual_config();
    let session = ManualControlSession::from_config(&config.manual_motion)
        .map_err(|error| error.to_string())?;
    let handle = spawn_worker(config, Some(session.control.clone()))
        .map_err(|error| error.to_string())?;

    thread::sleep(Duration::from_millis(350));
    let mut observed_events = Vec::new();
    while let Ok(event) = handle.events.try_recv() {
        observed_events.push(format!("{event:?}"));
    }
    assert!(
        !handle.join.is_finished(),
        "manual mode should ignore duration and keep streaming; events: \
         {observed_events:?}"
    );

    for event in observed_events {
        assert!(
            !event.contains("Finished") && !event.contains("Cancelled"),
            "manual worker stopped before cancel: {event}"
        );
        assert!(
            !event.contains("Error("),
            "manual worker errored before cancel: {event}"
        );
    }

    handle.cancel.store(true, Ordering::Relaxed);

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut saw_cancelled = false;
    while Instant::now() < deadline {
        match handle.events.recv_timeout(Duration::from_millis(100)) {
            Ok(WorkerEvent::Cancelled(_)) => {
                saw_cancelled = true;
                break;
            }
            Ok(WorkerEvent::Error(error)) => {
                return Err(format!(
                    "manual worker error after cancel: {error}"
                ));
            }
            Ok(
                WorkerEvent::Started { .. }
                | WorkerEvent::Log(_)
                | WorkerEvent::Progress(_)
                | WorkerEvent::Finished(_),
            )
            | Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(err) => {
                return Err(format!("worker event channel failed: {err}"));
            }
        }
    }

    match handle.join.join() {
        Ok(()) => {}
        Err(_) => return Err("join worker thread panicked".to_string()),
    }
    assert!(saw_cancelled, "manual worker did not emit cancel event");
    Ok(())
}

#[test]
fn typed_cancellation_is_clean_only_after_successful_finalization()
-> Result<(), Error> {
    let completion = resolve_worker_completion(
        Err(Error::RunCancelled),
        Ok(()),
        terminal_progress(),
    )?;
    assert!(matches!(completion, WorkerCompletion::Cancelled(_)));
    Ok(())
}

#[test]
fn cancellation_precedes_finalization_failure() -> Result<(), Error> {
    let Err(error) = resolve_worker_completion(
        Err(Error::RunCancelled),
        Err(Error::tx_backend_msg("finalizer", "finish failed")),
        terminal_progress(),
    ) else {
        return Err(Error::msg(
            "cancellation plus finalization failure was discarded",
        ));
    };
    let source = StdError::source(&error)
        .ok_or_else(|| Error::msg("combined error has no primary source"))?;
    assert_eq!(source.to_string(), "run cancelled");
    let Error::RunAndFinalizationFailed {
        primary,
        finalization,
    } = error
    else {
        return Err(Error::msg("expected combined run/finalization error"));
    };
    assert!(matches!(*primary, Error::RunCancelled));
    assert!(matches!(*finalization, Error::TxBackendMsg {
        backend: "finalizer",
        ..
    }));
    Ok(())
}

#[test]
fn progress_receiver_loss_is_not_user_cancellation() -> Result<(), Error> {
    let (event_tx, events) = mpsc::channel();
    drop(events);
    let progress_error = send_progress(&event_tx, terminal_progress())
        .err()
        .ok_or_else(|| Error::msg("closed progress receiver was accepted"))?;
    let Err(error) = resolve_worker_completion(
        Err(progress_error),
        Ok(()),
        terminal_progress(),
    ) else {
        return Err(Error::msg("progress receiver loss was discarded"));
    };
    assert!(matches!(
        error,
        Error::Other(ref message)
            if message == PROGRESS_RECEIVER_DISCONNECTED
    ));
    Ok(())
}
