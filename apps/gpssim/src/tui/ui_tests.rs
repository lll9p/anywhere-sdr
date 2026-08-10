use clap::Parser;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend, text::Line};

use super::*;
use crate::{
    cli::Args,
    tui::{
        app::{ActiveTab, App, RunState, handle_key_event},
        manual_control::ManualControlSession,
    },
    tui_config::{MotionSource, TuiConfig},
    utils::LogBuffer,
};

fn fully_prefilled_app() -> Result<App, Box<dyn std::error::Error>> {
    let args = Args::try_parse_from([
        "gpssim",
        "--tui",
        "--ephemerides",
        "nav.rnx",
        "--user-motion-ecef",
        "ecef.csv",
        "--user-motion-llh",
        "llh.csv",
        "--nmea-gga",
        "gga.txt",
        "--location-ecef",
        "1.25,2.5,3.75",
        "--location",
        "35.5,139.75,10.25",
        "--leap",
        "2347,3,19",
        "--time",
        "2026-07-14T00:00:00Z",
        "--time-override",
        "true",
        "--duration",
        "30.5",
        "--output",
        "samples.bin",
        "--tx",
        "hackrf",
        "--tx",
        "null",
        "--frequency",
        "3000000",
        "--bits",
        "8",
        "--ionospheric-disable",
        "--path-loss",
        "42",
        "--verbose",
        "--hackrf-serial",
        "abc123",
        "--hackrf-rf-freq-hz",
        "1576000000",
        "--hackrf-txvga-gain",
        "30",
        "--hackrf-amp-enable",
        "--hackrf-usb-transfer-bytes",
        "131072",
        "--hackrf-usb-transfers",
        "12",
        "--hackrf-queue-blocks",
        "6",
        "--hackrf-prefill-blocks",
        "3",
        "--hackrf-drop-on-underrun",
    ])?;
    let mut config = TuiConfig::default();
    config.apply_overrides_from_args(&args)?;
    Ok(App::new(config, LogBuffer::new(64)))
}

fn text(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

fn rendered_config(app: &App) -> Vec<String> {
    config_lines(app).iter().map(text).collect()
}

fn rendered_run(app: &App) -> String {
    run_lines(app)
        .iter()
        .map(text)
        .collect::<Vec<_>>()
        .join("\n")
}

fn rendered_screen(app: &App) -> Result<String, Box<dyn std::error::Error>> {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| ui(frame, app))?;
    Ok(terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect())
}

fn has_interaction_marker(line: &str) -> bool {
    line.contains("(edit:")
        || line.contains("(toggle:")
        || line.contains("(clear:")
        || line.contains("(read-only CLI prefill)")
}

fn field_line<'a>(lines: &'a [String], label: &str) -> &'a str {
    let prefix = format!("{label}: ");
    lines
        .iter()
        .find(|line| line.starts_with(&prefix))
        .map_or_else(
            || panic!("missing Config row for {label}"),
            String::as_str,
        )
}

#[test]
fn config_projection_covers_every_effective_field_and_interaction()
-> Result<(), Box<dyn std::error::Error>> {
    let app = fully_prefilled_app()?;
    let lines = rendered_config(&app);
    let expected = [
        ("motion_source", "preconfigured", "toggle: m"),
        (
            "manual_motion.initial_llh",
            "35.500000, 139.750000, 10.2",
            "edit: l",
        ),
        ("manual_motion.initial_heading_deg", "0.0", "edit: j"),
        ("manual_motion.cruise_speed_mps", "1.0", "edit: u"),
        ("manual_motion.accel_limit_mps2", "1.0", "edit: a"),
        ("manual_motion.turn_rate_limit_dps", "45.0", "edit: t"),
        ("ephemerides", "nav.rnx", "edit: e"),
        ("user_motion_ecef", "ecef.csv", "clear: c"),
        ("user_motion_llh", "llh.csv", "clear: c"),
        ("nmea_gga", "gga.txt", "clear: c"),
        ("location_ecef", "1.25,2.5,3.75", "clear: c"),
        ("location", "35.5,139.75,10.25", "clear: c"),
        ("leap", "2347,3,19", READ_ONLY),
        ("time", "2026-07-14T00:00:00Z", READ_ONLY),
        ("time_override", "true", READ_ONLY),
        ("duration", "30.5", "edit: d"),
        ("output", "samples.bin", "edit: o"),
        ("tx", "hackrf,null", "toggle: h/n"),
        ("frequency", "3000000", "edit: s"),
        ("bits", "8", "edit: b"),
        ("ionospheric_disable", "true", "toggle: i"),
        ("path_loss", "42", READ_ONLY),
        ("verbose", "true", "toggle: v"),
        ("hackrf_serial", "abc123", "edit: x"),
        ("hackrf_rf_freq_hz", "1576000000", "edit: r"),
        ("hackrf_txvga_gain", "30", READ_ONLY),
        ("hackrf_amp_enable", "true", READ_ONLY),
        ("hackrf_usb_transfer_bytes", "131072", READ_ONLY),
        ("hackrf_usb_transfers", "12", READ_ONLY),
        ("hackrf_queue_blocks", "6", READ_ONLY),
        ("hackrf_prefill_blocks", "3", READ_ONLY),
        ("hackrf_drop_on_underrun", "true", READ_ONLY),
    ];

    let rendered_fields = lines
        .iter()
        .filter(|line| has_interaction_marker(line))
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), 32);
    assert_eq!(rendered_fields.len(), expected.len());

    for (label, value, interaction) in expected {
        let prefix = format!("{label}: ");
        assert_eq!(
            rendered_fields
                .iter()
                .filter(|line| line.starts_with(&prefix))
                .count(),
            1,
            "{label} must have exactly one Config row"
        );
        let line = field_line(&lines, label);
        assert!(
            line.contains(value),
            "{label} did not render {value}: {line}"
        );
        assert!(
            line.ends_with(&format!("({interaction})")),
            "{label} has the wrong interaction marker: {line}"
        );
    }
    Ok(())
}

#[test]
fn optional_and_vector_formatting_is_deterministic() {
    let app = App::new(TuiConfig::default(), LogBuffer::new(64));
    let lines = rendered_config(&app);
    let expected = [
        ("ephemerides", format!("{UNSET} (edit: e)")),
        ("leap", format!("{UNSET} ({READ_ONLY})")),
        ("time_override", format!("{UNSET} ({READ_ONLY})")),
        ("duration", format!("{UNSET} (edit: d)")),
        (
            "output",
            format!("{UNSET} (effective default: gpssim.bin) (edit: o)"),
        ),
        ("tx", "<none> (toggle: h/n)".to_string()),
    ];

    for (label, value) in expected {
        assert_eq!(field_line(&lines, label), format!("{label}: {value}"));
    }
}

#[test]
fn clear_action_removes_all_cli_sources_but_preserves_manual_start()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = fully_prefilled_app()?;
    app.config.motion_source = MotionSource::Manual;
    let initial_llh = app.config.manual_motion.initial_llh;
    assert!(app.config.validate_for_run().is_err());

    handle_key_event(
        &mut app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
    );

    assert!(app.config.user_motion_ecef.is_none());
    assert!(app.config.user_motion_llh.is_none());
    assert!(app.config.nmea_gga.is_none());
    assert!(app.config.location_ecef.is_none());
    assert!(app.config.location.is_none());
    assert_eq!(app.config.manual_motion.initial_llh, initial_llh);
    assert_eq!(app.config.validate_for_run(), Ok(()));
    assert_eq!(
        app.message.as_deref(),
        Some("cleared CLI-prefilled motion and location sources")
    );

    let lines = rendered_config(&app);
    for label in [
        "user_motion_ecef",
        "user_motion_llh",
        "nmea_gga",
        "location_ecef",
        "location",
    ] {
        assert_eq!(
            field_line(&lines, label),
            format!("{label}: {UNSET} (clear: c)")
        );
    }
    Ok(())
}

#[test]
fn config_and_log_scrolling_are_independent() {
    let mut app = App::new(TuiConfig::default(), LogBuffer::new(64));
    app.log_scroll = 4;

    handle_key_event(&mut app, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(app.config_scroll, 1);
    assert_eq!(app.log_scroll, 4);

    app.tab = ActiveTab::Logs;
    handle_key_event(&mut app, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(app.config_scroll, 1);
    assert_eq!(app.log_scroll, 5);

    handle_key_event(
        &mut app,
        KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
    );
    assert_eq!(app.config_scroll, 1);
    assert_eq!(app.log_scroll, 4);
}

#[test]
fn config_scrolling_reaches_the_last_effective_field()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = fully_prefilled_app()?;
    assert!(!rendered_screen(&app)?.contains("hackrf_drop_on_underrun"));

    let mut reached_last_field = false;
    for _ in 0..128 {
        handle_key_event(
            &mut app,
            KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
        );
        if rendered_screen(&app)?.contains("hackrf_drop_on_underrun") {
            reached_last_field = true;
            break;
        }
    }

    assert!(reached_last_field);
    assert!(app.config_scroll > 0);
    assert_eq!(app.log_scroll, 0);
    Ok(())
}

#[test]
fn control_c_keeps_exit_priority_over_config_clear()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = fully_prefilled_app()?;
    handle_key_event(
        &mut app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    );

    assert!(app.should_exit());
    assert!(app.config.user_motion_ecef.is_some());
    assert!(app.config.location.is_some());
    Ok(())
}

#[test]
fn manual_panel_and_help_require_live_run_state() -> Result<(), String> {
    let mut app = App::new(TuiConfig::default(), LogBuffer::new(64));
    app.config.manual_motion.initial_llh = Some([35.0, 139.0, 10.0]);
    app.manual_session = Some(
        ManualControlSession::from_config(&app.config.manual_motion)
            .map_err(|error| error.to_string())?,
    );

    let idle = rendered_run(&app);
    assert!(!idle.contains("target_heading"));
    assert!(!idle.contains("Left/Right=heading"));
    assert!(!app.prefers_fast_input_poll());

    app.run_state = RunState::Running;
    let running = rendered_run(&app);
    assert!(running.contains("target_heading"));
    assert!(running.contains("Left/Right=heading"));
    assert!(app.prefers_fast_input_poll());

    app.run_state = RunState::Stopping;
    let stopping = rendered_run(&app);
    assert!(!stopping.contains("target_heading"));
    assert!(!stopping.contains("Left/Right=heading"));
    assert!(!app.prefers_fast_input_poll());
    Ok(())
}
