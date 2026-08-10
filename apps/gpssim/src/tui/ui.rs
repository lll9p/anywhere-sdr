use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
};

use super::{
    app::{ActiveTab, App, InputMode, LastRun, RunState},
    manual_control::{ManualControlSession, format_llh},
    worker::resolve_output_path,
};
use crate::cli::TxBackend;

const CONFIG_KEYS: &str =
    "Keys: Enter=start | q=quit | Tab=switch | Up/Down=scroll | c=clear CLI \
     sources | e=ephemerides | o=output | h=toggle hackrf | n=toggle null | \
     s=frequency | b=bits | d=duration | m=motion | l=manual llh | j=heading \
     | u=cruise | a=accel | t=turn rate | v=verbose | i=iono | r=HackRF \
     frequency | x=HackRF serial";
const UNSET: &str = "<unset>";
const READ_ONLY: &str = "read-only CLI prefill";

const RUN_KEYS_DEFAULT: &str = "Keys: c=cancel | q=quit | Tab=switch";
const RUN_KEYS_MANUAL: &str = "Keys: Left/Right=heading | Up/Down=speed | \
                               Space=stop | Enter=cruise | c/Esc=cancel | \
                               q=quit | Tab=switch";

pub(super) fn ui(frame: &mut ratatui::Frame, app: &App) {
    let area = frame.area();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(4),
        ])
        .split(area);

    let titles = [Line::from("Config"), Line::from("Run"), Line::from("Logs")];

    let tabs = Tabs::new(titles)
        .select(app.tab.index())
        .block(Block::default().borders(Borders::ALL).title("gpssim"))
        .style(Style::default().fg(Color::Gray))
        .highlight_style(Style::default().fg(Color::Yellow));
    frame.render_widget(tabs, layout[0]);

    match app.tab {
        ActiveTab::Config => {
            render_config(frame, layout[1], app);
        }
        ActiveTab::Run => {
            render_run(frame, layout[1], app);
        }
        ActiveTab::Logs => {
            render_logs(frame, layout[1], app);
        }
    }

    render_footer(frame, layout[2], app);
}

fn render_config(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let paragraph = Paragraph::new(Text::from(config_lines(app)))
        .block(Block::default().borders(Borders::ALL).title("Config"))
        .wrap(Wrap { trim: false })
        .scroll((app.config_scroll, 0));
    frame.render_widget(paragraph, area);
}

fn render_run(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let paragraph = Paragraph::new(Text::from(run_lines(app)))
        .block(Block::default().borders(Borders::ALL).title("Run"))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn config_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(CONFIG_KEYS), Line::from("")];
    lines.extend(motion_and_scenario_lines(app));
    lines.extend(manual_config_lines(app));
    lines.extend(generation_and_output_lines(app));
    lines.extend(hackrf_config_lines(app));
    lines
}

fn motion_and_scenario_lines(app: &App) -> Vec<Line<'static>> {
    vec![
        Line::from("Motion and scenario"),
        config_line(
            "motion_source",
            app.config.motion_source.label(),
            "toggle: m",
        ),
        config_line(
            "ephemerides",
            display_optional_path(app.config.ephemerides.as_deref()),
            "edit: e",
        ),
        config_line(
            "user_motion_ecef",
            display_optional_path(app.config.user_motion_ecef.as_deref()),
            "clear: c",
        ),
        config_line(
            "user_motion_llh",
            display_optional_path(app.config.user_motion_llh.as_deref()),
            "clear: c",
        ),
        config_line(
            "nmea_gga",
            display_optional_path(app.config.nmea_gga.as_deref()),
            "clear: c",
        ),
        config_line(
            "location_ecef",
            display_optional_values(app.config.location_ecef.as_deref()),
            "clear: c",
        ),
        config_line(
            "location",
            display_optional_values(app.config.location.as_deref()),
            "clear: c",
        ),
        config_line(
            "leap",
            display_optional_values(app.config.leap.as_deref()),
            READ_ONLY,
        ),
        config_line(
            "time",
            display_optional(app.config.time.as_deref()),
            READ_ONLY,
        ),
        config_line(
            "time_override",
            display_optional(app.config.time_override),
            READ_ONLY,
        ),
    ]
}

fn manual_config_lines(app: &App) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from("Manual motion"),
        config_line(
            "manual_motion.initial_llh",
            app.config
                .manual_motion
                .initial_llh
                .map_or_else(|| UNSET.to_string(), format_llh),
            "edit: l",
        ),
        config_line(
            "manual_motion.initial_heading_deg",
            format!("{:.1}", app.config.manual_motion.initial_heading_deg),
            "edit: j",
        ),
        config_line(
            "manual_motion.cruise_speed_mps",
            format!("{:.1}", app.config.manual_motion.cruise_speed_mps),
            "edit: u",
        ),
        config_line(
            "manual_motion.accel_limit_mps2",
            format!("{:.1}", app.config.manual_motion.accel_limit_mps2),
            "edit: a",
        ),
        config_line(
            "manual_motion.turn_rate_limit_dps",
            format!("{:.1}", app.config.manual_motion.turn_rate_limit_dps),
            "edit: t",
        ),
    ]
}

fn generation_and_output_lines(app: &App) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from("Generation and output"),
        config_line("duration", duration_text(app), "edit: d"),
        config_line("output", output_text(app), "edit: o"),
        config_line("tx", tx_list(app), "toggle: h/n"),
        config_line("frequency", app.config.frequency.to_string(), "edit: s"),
        config_line("bits", app.config.bits.to_string(), "edit: b"),
        config_line(
            "ionospheric_disable",
            app.config.ionospheric_disable.to_string(),
            "toggle: i",
        ),
        config_line(
            "path_loss",
            display_optional(app.config.path_loss),
            READ_ONLY,
        ),
        config_line("verbose", app.config.verbose.to_string(), "toggle: v"),
    ]
}

fn hackrf_config_lines(app: &App) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from("HackRF"),
        config_line(
            "hackrf_serial",
            display_optional(app.config.hackrf_serial.as_deref()),
            "edit: x",
        ),
        config_line(
            "hackrf_rf_freq_hz",
            app.config.hackrf_rf_freq_hz.to_string(),
            "edit: r",
        ),
        config_line(
            "hackrf_txvga_gain",
            app.config.hackrf_txvga_gain.to_string(),
            READ_ONLY,
        ),
        config_line(
            "hackrf_amp_enable",
            app.config.hackrf_amp_enable.to_string(),
            READ_ONLY,
        ),
        config_line(
            "hackrf_usb_transfer_bytes",
            app.config.hackrf_usb_transfer_bytes.to_string(),
            READ_ONLY,
        ),
        config_line(
            "hackrf_usb_transfers",
            app.config.hackrf_usb_transfers.to_string(),
            READ_ONLY,
        ),
        config_line(
            "hackrf_queue_blocks",
            app.config.hackrf_queue_blocks.to_string(),
            READ_ONLY,
        ),
        config_line(
            "hackrf_prefill_blocks",
            app.config.hackrf_prefill_blocks.to_string(),
            READ_ONLY,
        ),
        config_line(
            "hackrf_drop_on_underrun",
            app.config.hackrf_drop_on_underrun.to_string(),
            READ_ONLY,
        ),
    ]
}

fn run_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = vec![
        labeled_line("state", run_state_label(app.run_state)),
        labeled_line("sinks", app.sinks_desc.as_str()),
    ];

    if let Some(progress) = &app.progress {
        lines.extend(progress_lines(progress));
    }
    if let Some(last_run) = &app.last_run {
        lines.extend(last_run_lines(last_run));
    }
    let manual_session = app.live_manual_session();
    if let Some(session) = manual_session {
        lines.extend(manual_run_lines(session));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(run_keys(manual_session.is_some())));
    lines
}

fn progress_lines(progress: &super::worker::Progress) -> Vec<Line<'static>> {
    let mut lines = vec![
        labeled_line("blocks", progress.blocks.to_string()),
        labeled_line(
            "elapsed",
            format!("{:.3}s", progress.elapsed.as_secs_f64()),
        ),
        labeled_line("sim", format!("{:.3}s", progress.sim_seconds)),
        labeled_line(
            "throughput",
            format!("{:.3} MSps", progress.throughput_msps),
        ),
    ];

    if let Some(underruns) = progress.hackrf_underruns {
        lines.push(labeled_line("hackrf_underruns", underruns.to_string()));
    }

    lines
}

fn last_run_lines(last_run: &LastRun) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        labeled_line("last", last_run_label(last_run)),
    ];
    if let LastRun::Error(error) = last_run {
        lines.push(Line::from(error.to_string()));
    }
    lines
}

fn manual_run_lines(session: &ManualControlSession) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("mode", Style::default().fg(Color::Yellow)),
            Span::raw(": manual (runs until cancel)"),
        ]),
    ];
    lines.push(labeled_line(
        "target_heading",
        format!("{:.1} deg", session.target_heading_deg),
    ));
    lines.push(labeled_line(
        "target_speed",
        format!("{:.1} m/s", session.target_speed_mps),
    ));

    if let Some(snapshot) = &session.last_snapshot {
        lines.push(labeled_line(
            "actual_heading",
            format!("{:.1} deg", snapshot.heading_deg),
        ));
        lines.push(labeled_line(
            "actual_speed",
            format!("{:.1} m/s", snapshot.speed_mps),
        ));
        lines.push(labeled_line(
            "position_llh",
            match session.actual_position_llh() {
                Ok(Some(location)) => format_llh(location),
                Ok(None) => "<unavailable>".to_string(),
                Err(error) => format!("<invalid: {error}>"),
            },
        ));
    } else {
        lines.push(labeled_line("actual_state", "waiting for snapshot"));
    }

    lines
}

fn config_line(
    label: &'static str, value: impl Into<String>, interaction: &'static str,
) -> Line<'static> {
    labeled_line(label, format!("{} ({interaction})", value.into()))
}

fn labeled_line(
    label: &'static str, value: impl Into<String>,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(label, Style::default().fg(Color::Cyan)),
        Span::raw(": "),
        Span::raw(value.into()),
    ])
}

fn display_optional<T: ToString>(value: Option<T>) -> String {
    value.map_or_else(|| UNSET.to_string(), |value| value.to_string())
}

fn display_optional_path(path: Option<&std::path::Path>) -> String {
    path.map_or_else(|| UNSET.to_string(), |value| value.display().to_string())
}

fn display_optional_values<T: ToString>(values: Option<&[T]>) -> String {
    values.map_or_else(
        || UNSET.to_string(),
        |values| {
            values
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        },
    )
}

fn tx_list(app: &App) -> String {
    if app.config.tx.is_empty() {
        "<none>".to_string()
    } else {
        app.config
            .tx
            .iter()
            .map(|backend| match backend {
                TxBackend::Hackrf => "hackrf",
                TxBackend::Null => "null",
            })
            .collect::<Vec<_>>()
            .join(",")
    }
}

fn output_text(app: &App) -> String {
    app.config.output.as_deref().map_or_else(
        || {
            resolve_output_path(&app.config).map_or_else(
                || UNSET.to_string(),
                |path| {
                    format!("{UNSET} (effective default: {})", path.display())
                },
            )
        },
        |path| path.display().to_string(),
    )
}

fn duration_text(app: &App) -> String {
    app.config.duration.map_or_else(
        || {
            if app.config.uses_manual_motion() {
                format!("{UNSET} (ignored in manual mode)")
            } else {
                UNSET.to_string()
            }
        },
        |duration| {
            if app.config.uses_manual_motion() {
                format!("{duration} (ignored in manual mode)")
            } else {
                duration.to_string()
            }
        },
    )
}

fn run_state_label(run_state: RunState) -> &'static str {
    match run_state {
        RunState::Idle => "idle",
        RunState::Running => "running",
        RunState::Stopping => "stopping",
    }
}

fn last_run_label(last_run: &LastRun) -> &'static str {
    match last_run {
        LastRun::Finished => "finished",
        LastRun::Cancelled => "cancelled",
        LastRun::Error(_) => "error",
    }
}

fn run_keys(is_manual: bool) -> &'static str {
    if is_manual {
        RUN_KEYS_MANUAL
    } else {
        RUN_KEYS_DEFAULT
    }
}

fn render_logs(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let lines = app.log_buffer.snapshot();

    let mut text = String::new();
    for line in lines {
        text.push_str(&line);
        text.push('\n');
    }

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Logs"))
        .wrap(Wrap { trim: false })
        .scroll((app.log_scroll, 0));
    frame.render_widget(paragraph, area);
}

fn render_footer(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let mut lines: Vec<Line> = Vec::new();

    if let Some(message) = &app.message {
        lines.push(Line::from(message.clone()));
    }

    if app.input_mode == InputMode::Editing {
        lines.push(Line::from(format!("> {}", app.input_buffer)));
    }

    let footer = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .wrap(Wrap { trim: false });
    frame.render_widget(footer, area);
}

#[cfg(test)]
#[path = "ui_tests.rs"]
mod tests;
