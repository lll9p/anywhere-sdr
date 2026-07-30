use std::panic::{AssertUnwindSafe, catch_unwind};

use gps::IqBlockSizing;

use super::{HackrfTxSink, TxSink, test_support::*};

#[test]
fn hackrf_sink_remains_send() {
    fn assert_send<T: Send>() {}

    assert_send::<HackrfTxSink>();
}

#[test]
fn invalid_config_is_rejected_before_open() {
    let base = valid_config();
    let mut cases = Vec::new();

    cases.push(("expected_i16_len=0", base.clone(), 0));
    cases.push(("expected_i16_len=3", base.clone(), 3));
    cases.push((
        "expected_i16_len above sizing limit",
        base.clone(),
        IqBlockSizing::MAX_INTERLEAVED_I16_VALUES + 2,
    ));

    let mut config = base.clone();
    config.rf_freq_hz = 999_999;
    cases.push(("rf below minimum", config, 4));
    let mut config = base.clone();
    config.rf_freq_hz = 6_000_000_001;
    cases.push(("rf above maximum", config, 4));

    let mut config = base.clone();
    config.sample_frequency_hz = f64::NAN;
    cases.push(("sample rate NaN", config, 4));
    let mut config = base.clone();
    config.sample_frequency_hz = 1_999_999.0;
    cases.push(("sample rate below minimum", config, 4));
    let mut config = base.clone();
    config.sample_frequency_hz = 20_000_001.0;
    cases.push(("sample rate above maximum", config, 4));

    let mut config = base.clone();
    config.step_duration = std::time::Duration::ZERO;
    cases.push(("zero step duration", config, 4));
    let mut config = base.clone();
    config.txvga_gain = 48;
    cases.push(("gain above maximum", config, 4));
    let mut config = base.clone();
    config.usb_transfer_bytes = 0;
    cases.push(("zero transfer bytes", config, 4));
    let mut config = base.clone();
    config.usb_transfers = 0;
    cases.push(("zero transfer count", config, 4));
    let mut config = base;
    config.queue_blocks = 0;
    cases.push(("zero queue depth", config, 4));

    for (name, config, expected_i16_len) in cases {
        let (result, events, _) =
            build_sink(config, expected_i16_len, Scenario::default());
        let Err(error) = result else {
            panic!("{name} unexpectedly opened a device");
        };
        let message = error.to_string();
        assert!(message.contains("TX backend `hackrf`"), "{name}: {message}");
        assert!(message.contains("rf_freq_hz="), "{name}: {message}");
        assert!(events.snapshot().is_empty(), "{name} invoked the opener");
    }
}

#[test]
fn zero_transfer_count_never_reaches_nusb_assertion() {
    let mut config = valid_config();
    config.usb_transfers = 0;
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        build_sink(config, 4, Scenario::default()).0
    }));
    let Ok(result) = outcome else {
        panic!("zero transfer count panicked");
    };
    let Err(error) = result else {
        panic!("zero transfer count unexpectedly succeeded");
    };
    assert!(error.to_string().contains("usb_transfers"));
}

#[test]
fn valid_boundary_config_reaches_open() {
    let mut minimum = valid_config();
    minimum.rf_freq_hz = 1_000_000;
    minimum.sample_frequency_hz = 2_000_000.0;
    minimum.txvga_gain = 0;

    let mut maximum = valid_config();
    maximum.rf_freq_hz = 6_000_000_000;
    maximum.sample_frequency_hz = 20_000_000.0;
    maximum.txvga_gain = 47;

    for (name, config) in [("minimum", minimum), ("maximum", maximum)] {
        let (result, events, _) = build_sink(config, 4, Scenario::default());
        let mut sink = match result {
            Ok(sink) => sink,
            Err(error) => panic!("{name} boundary failed: {error}"),
        };
        assert!(events.snapshot().contains(&Event::Open));
        assert!(sink.finish().is_ok(), "{name} boundary failed to finish");
    }
}
