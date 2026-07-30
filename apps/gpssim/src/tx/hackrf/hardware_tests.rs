use gps::IqBlockSizing;

use super::{HackrfTxConfig, HackrfTxSink, TxSink};
use crate::Error;

#[test]
#[ignore = "Requires HackRF hardware"]
fn hackrf_tx_smoke() -> Result<(), Error> {
    let config = HackrfTxConfig {
        serial: None,
        rf_freq_hz: 1_575_420_000,
        sample_frequency_hz: 2_600_000.0,
        step_duration: std::time::Duration::from_millis(100),
        txvga_gain: 20,
        amp_enable: false,
        usb_transfer_bytes: 256 * 1024,
        usb_transfers: 16,
        queue_blocks: 8,
        prefill_blocks: 2,
        silence_on_underrun: true,
        underrun_counter: None,
    };

    let sizing = IqBlockSizing::new(1024)?;
    let mut sink = HackrfTxSink::new(config, sizing.interleaved_i16_len())?;
    let block = vec![0_i16; sizing.interleaved_i16_len()];
    for _ in 0..10 {
        sink.write_block_i16(&block)?;
    }
    sink.finish()?;
    Ok(())
}
