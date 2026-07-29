use std::{
    error::Error as StdError,
    io,
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool, mpsc},
    thread,
    time::{Duration, Instant},
};

use super::{
    app::{App, LastRun},
    worker::{Progress, WorkerEvent, WorkerHandle},
};
use crate::{
    Error, cli::TxBackend, error::resolve_tui_and_terminal_cleanup,
    tui_config::TuiConfig, utils::LogBuffer,
};

fn valid_config() -> TuiConfig {
    TuiConfig {
        ephemerides: Some(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../resources/brdc0010.22n"),
        ),
        tx: vec![TxBackend::Null],
        ..TuiConfig::default()
    }
}

fn progress(blocks: u64) -> Progress {
    Progress {
        blocks,
        elapsed: Duration::from_millis(1),
        sim_seconds: 0.1,
        throughput_msps: 0.2,
        hackrf_underruns: None,
    }
}

fn worker_with_event(event: WorkerEvent) -> io::Result<WorkerHandle> {
    let (event_tx, events) = mpsc::channel();
    event_tx
        .send(event)
        .map_err(|_| io::Error::other("synthetic event receiver closed"))?;
    drop(event_tx);
    let join = thread::Builder::new().spawn(|| {})?;
    Ok(WorkerHandle {
        cancel: Arc::new(AtomicBool::new(false)),
        events,
        join,
        pending_terminal: None,
        events_disconnected: false,
    })
}

fn drain_until_idle(app: &mut App) -> Result<(), Error> {
    let deadline = Instant::now() + Duration::from_secs(2);
    while app.worker.is_some() && Instant::now() < deadline {
        app.drain_worker_events();
        thread::yield_now();
    }
    if app.worker.is_some() {
        Err(Error::msg(
            "synthetic worker was not reaped before deadline",
        ))
    } else {
        Ok(())
    }
}

fn complete_run(app: &mut App, event: WorkerEvent) -> Result<(), Error> {
    app.start_run_with_spawn(|_, _| worker_with_event(event));
    if let Some(message) = app.message.take() {
        return Err(Error::msg(message));
    }
    drain_until_idle(app)
}

#[test]
fn typed_worker_error_reaches_session_result() -> Result<(), Error> {
    let mut app = App::new(valid_config(), LogBuffer::new(16));
    complete_run(
        &mut app,
        WorkerEvent::Error(Error::tx_backend_msg("synthetic", "run failed")),
    )?;

    let Err(error) = app.take_session_result() else {
        return Err(Error::msg("typed worker error became session success"));
    };
    assert!(matches!(
        error,
        Error::TxBackendMsg {
            backend: "synthetic",
            ref message,
        } if message == "run failed"
    ));
    Ok(())
}

#[test]
fn later_success_or_cancellation_clears_earlier_failure() -> Result<(), Error> {
    let mut app = App::new(valid_config(), LogBuffer::new(16));
    complete_run(
        &mut app,
        WorkerEvent::Error(Error::tx_backend_msg("synthetic", "first failed")),
    )?;
    assert!(matches!(&app.last_run, Some(LastRun::Error(_))));

    complete_run(&mut app, WorkerEvent::Finished(progress(2)))?;
    assert!(app.take_session_result().is_ok());

    complete_run(
        &mut app,
        WorkerEvent::Error(Error::tx_backend_msg("synthetic", "third failed")),
    )?;
    complete_run(&mut app, WorkerEvent::Cancelled(progress(4)))?;
    assert!(app.take_session_result().is_ok());
    Ok(())
}

#[cfg(panic = "unwind")]
#[test]
fn non_string_worker_panic_uses_stable_fallback() -> Result<(), Error> {
    let (event_tx, events) = mpsc::channel();
    drop(event_tx);
    let join = thread::Builder::new().spawn(|| std::panic::panic_any(42_u8))?;
    let worker = WorkerHandle {
        cancel: Arc::new(AtomicBool::new(false)),
        events,
        join,
        pending_terminal: None,
        events_disconnected: false,
    };
    let mut app = App::new(valid_config(), LogBuffer::new(16));
    app.start_run_with_spawn(|_, _| Ok(worker));
    drain_until_idle(&mut app)?;

    let Err(error) = app.take_session_result() else {
        return Err(Error::msg("worker panic became session success"));
    };
    assert!(matches!(
        error,
        Error::WorkerPanicked { ref message }
            if message == "non-string panic payload"
    ));
    Ok(())
}

#[test]
fn session_failure_precedes_terminal_cleanup_failure() -> Result<(), Error> {
    let mut app = App::new(valid_config(), LogBuffer::new(16));
    complete_run(
        &mut app,
        WorkerEvent::Error(Error::tx_backend_msg("session", "run failed")),
    )?;
    let session_result = app.take_session_result();
    let cleanup_result = Err(Error::terminal_operation(
        "restore test terminal",
        io::Error::other("cleanup failed"),
    ));

    let Err(error) =
        resolve_tui_and_terminal_cleanup(session_result, cleanup_result)
    else {
        return Err(Error::msg("session and cleanup failures were discarded"));
    };
    let source = StdError::source(&error).ok_or_else(|| {
        Error::msg("combined TUI error has no primary source")
    })?;
    assert!(source.to_string().contains("run failed"));
    let Error::TuiAndTerminalCleanupFailed { primary, cleanup } = error else {
        return Err(Error::msg("expected typed TUI/cleanup aggregate"));
    };
    assert!(matches!(*primary, Error::TxBackendMsg {
        backend: "session",
        ..
    }));
    assert!(matches!(*cleanup, Error::TerminalOperation {
        operation: "restore test terminal",
        ..
    }));
    Ok(())
}
