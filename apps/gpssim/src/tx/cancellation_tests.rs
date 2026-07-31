use std::{
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use gps::DataFormat;

use super::{TxSink, TxTee};
use crate::Error;

struct CancellationSink {
    name: &'static str,
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl TxSink for CancellationSink {
    fn backend(&self) -> &'static str {
        self.name
    }

    fn write_block_i16(&mut self, _block: &[i16]) -> Result<(), Error> {
        Ok(())
    }

    fn request_cancel(&mut self) {
        match self.events.lock() {
            Ok(mut events) => events.push(self.name),
            Err(poisoned) => poisoned.into_inner().push(self.name),
        }
    }

    fn finish(&mut self) -> Result<(), Error> {
        match self.events.lock() {
            Ok(mut events) => events.push("finish"),
            Err(poisoned) => poisoned.into_inner().push("finish"),
        }
        Ok(())
    }
}

#[test]
fn tee_forwards_cancellation_in_sink_order_before_exact_once_finish() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut tee = TxTee::new(vec![
        Box::new(CancellationSink {
            name: "first",
            events: events.clone(),
        }),
        Box::new(CancellationSink {
            name: "second",
            events: events.clone(),
        }),
    ]);
    tee.request_cancel();
    assert!(tee.finish().is_ok());

    let events = match events.lock() {
        Ok(events) => events.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    };
    assert_eq!(events, vec!["first", "second", "finish", "finish"]);
}

#[test]
fn file_and_null_default_cancellation_hooks_are_noops() -> Result<(), Error> {
    let mut null = super::NullTxSink::new();
    null.request_cancel();
    null.finish()?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| Error::msg(error.to_string()))?;
    let path = std::env::temp_dir().join(format!(
        "gpssim-cancel-hook-{}-{}.bin",
        std::process::id(),
        timestamp.as_nanos()
    ));
    let mut file = super::FileTxSink::new(path.clone(), DataFormat::Bits8, 1)?;
    file.request_cancel();
    file.write_block_i16(&[1, -1])?;
    file.finish()?;
    std::fs::remove_file(path)?;
    Ok(())
}
