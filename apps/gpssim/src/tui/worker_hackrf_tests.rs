use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use super::*;
use crate::tx::{TxSink, TxTee};

struct FullQueueSink {
    external: Arc<AtomicBool>,
    internal: Arc<AtomicBool>,
    full_sender: Option<mpsc::Sender<()>>,
    fail_finish: bool,
}

impl TxSink for FullQueueSink {
    fn backend(&self) -> &'static str {
        "hackrf"
    }

    fn write_block_i16(&mut self, _block: &[i16]) -> Result<(), Error> {
        if let Some(sender) = self.full_sender.take()
            && sender.send(()).is_err()
        {
            return Err(Error::msg("full-queue observer dropped"));
        }
        while !self.external.load(Ordering::Acquire) {
            thread::park_timeout(Duration::from_millis(1));
        }
        Err(Error::RunCancelled)
    }

    fn request_cancel(&mut self) {
        self.internal.store(true, Ordering::Release);
    }

    fn finish(&mut self) -> Result<(), Error> {
        if !self.internal.load(Ordering::Acquire) {
            return Err(Error::tx_backend_msg(
                "hackrf",
                "finish ran before cancellation",
            ));
        }
        if self.fail_finish {
            Err(Error::tx_backend_msg("hackrf", "writer shutdown timed out"))
        } else {
            Ok(())
        }
    }
}

fn progress() -> Progress {
    compute_progress(Instant::now(), 1, 2, 2_000_000.0, Some(0))
}

fn run_full_queue_case(fail_finish: bool) -> Result<WorkerCompletion, Error> {
    let cancellation = Arc::new(AtomicBool::new(false));
    let internal = Arc::new(AtomicBool::new(false));
    let (full_sender, full_receiver) = mpsc::channel();
    let (result_sender, result_receiver) = mpsc::channel();
    let thread_cancellation = cancellation.clone();
    let handle = thread::spawn(move || {
        let mut tee = TxTee::new(vec![Box::new(FullQueueSink {
            external: thread_cancellation.clone(),
            internal,
            full_sender: Some(full_sender),
            fail_finish,
        })]);
        let run = tee.write_block_i16(&[1, -1]);
        let (run, finish) =
            finish_streaming_run(&thread_cancellation, &mut tee, run);
        let result = resolve_worker_completion(run, finish, progress());
        if result_sender.send(result).is_err() {
            tracing::debug!("worker test result receiver dropped");
        }
    });

    full_receiver
        .recv_timeout(Duration::from_secs(1))
        .map_err(|error| Error::msg(error.to_string()))?;
    cancellation.store(true, Ordering::Release);
    let result = result_receiver
        .recv_timeout(Duration::from_secs(1))
        .map_err(|error| Error::msg(error.to_string()))?;
    if handle.join().is_err() {
        return Err(Error::msg("full-queue worker panicked"));
    }
    result
}

#[test]
fn tui_cancel_reaches_full_hackrf_sink_and_worker_becomes_reapable()
-> Result<(), Error> {
    let completion = run_full_queue_case(false)?;
    assert!(matches!(completion, WorkerCompletion::Cancelled(_)));
    Ok(())
}

#[test]
fn tui_cancellation_plus_shutdown_timeout_is_typed_error() {
    let Err(error) = run_full_queue_case(true) else {
        panic!("shutdown timeout became clean cancellation");
    };
    assert!(matches!(error, Error::RunAndFinalizationFailed { .. }));
    assert!(error.to_string().contains("writer shutdown timed out"));
}

struct LatchSink {
    external: Arc<AtomicBool>,
    cancel_requested: Arc<AtomicBool>,
}

impl TxSink for LatchSink {
    fn backend(&self) -> &'static str {
        "latch"
    }

    fn write_block_i16(&mut self, _block: &[i16]) -> Result<(), Error> {
        Ok(())
    }

    fn request_cancel(&mut self) {
        self.cancel_requested.store(true, Ordering::Release);
    }

    fn finish(&mut self) -> Result<(), Error> {
        self.external.store(true, Ordering::Release);
        Ok(())
    }
}

#[test]
fn cancellation_before_latch_is_cancelled_but_after_latch_is_too_late() {
    let external = Arc::new(AtomicBool::new(true));
    let cancel_requested = Arc::new(AtomicBool::new(false));
    let mut tee = TxTee::new(vec![Box::new(LatchSink {
        external: external.clone(),
        cancel_requested: cancel_requested.clone(),
    })]);
    let (run, finish) = finish_streaming_run(&external, &mut tee, Ok(()));
    assert!(matches!(run, Err(Error::RunCancelled)));
    assert!(finish.is_ok());
    assert!(cancel_requested.load(Ordering::Acquire));

    let external = Arc::new(AtomicBool::new(false));
    let cancel_requested = Arc::new(AtomicBool::new(false));
    let mut tee = TxTee::new(vec![Box::new(LatchSink {
        external: external.clone(),
        cancel_requested: cancel_requested.clone(),
    })]);
    let (run, finish) = finish_streaming_run(&external, &mut tee, Ok(()));
    assert!(run.is_ok());
    assert!(finish.is_ok());
    assert!(external.load(Ordering::Acquire));
    assert!(!cancel_requested.load(Ordering::Acquire));
}
