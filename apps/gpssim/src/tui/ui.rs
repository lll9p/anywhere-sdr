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

const CONFIG_KEYS: &str = "Keys: Enter=start | q=quit | Tab=switch | \
                           e=ephemerides | o=output | h=toggle hackrf | \
                           n=toggle null | s=frequency | b=bits | d=duration \
                           | m=motion | l=manual llh | j=heading | u=cruise | \
                           a=accel | t=turn rate | v=verbose | i=iono";

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
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn render_run(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let paragraph = Paragraph::new(Text::from(run_lines(app)))
        .block(Block::default().borders(Borders::ALL).title("Run"))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn config_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = base_config_lines(app);
    lines.extend(manual_config_lines(app));
    lines.extend(hackrf_config_lines(app));
    lines.push(Line::from(""));
    lines.push(Line::from(CONFIG_KEYS));
    lines
}

fn base_config_lines(app: &App) -> Vec<Line<'static>> {
    vec![
        labeled_line(
            "motion_source",
            format!("{} (toggle: m)", app.config.motion_source.label()),
        ),
        labeled_line(
            "ephemerides",
            display_optional_path(app.config.ephemerides.as_deref(), "<unset>"),
        ),
        labeled_line(
            "output",
            resolve_output_path(&app.config).map_or_else(
                || "<none>".to_string(),
                |path| path.display().to_string(),
            ),
        ),
        labeled_line("tx", tx_list(app)),
        labeled_line("frequency", app.config.frequency.to_string()),
        labeled_line("bits", app.config.bits.to_string()),
        labeled_line("duration", duration_text(app)),
    ]
}

fn manual_config_lines(app: &App) -> Vec<Line<'static>> {
    vec![
        labeled_line(
            "manual_initial_llh",
            format!(
                "{} (edit: l)",
                app.config
                    .manual_motion
                    .initial_llh
                    .map_or_else(|| "<unset>".to_string(), format_llh)
            ),
        ),
        labeled_line(
            "manual_heading_deg",
            format!(
                "{:.1} (edit: j)",
                app.config.manual_motion.initial_heading_deg
            ),
        ),
        labeled_line(
            "manual_cruise_speed_mps",
            format!(
                "{:.1} (edit: u)",
                app.config.manual_motion.cruise_speed_mps
            ),
        ),
        labeled_line(
            "manual_accel_limit_mps2",
            format!(
                "{:.1} (edit: a)",
                app.config.manual_motion.accel_limit_mps2
            ),
        ),
        labeled_line(
            "manual_turn_rate_limit_dps",
            format!(
                "{:.1} (edit: t)",
                app.config.manual_motion.turn_rate_limit_dps
            ),
        ),
    ]
}

fn hackrf_config_lines(app: &App) -> Vec<Line<'static>> {
    vec![
        labeled_line(
            "hackrf_rf_freq_hz",
            format!("{} (edit: r)", app.config.hackrf_rf_freq_hz),
        ),
        labeled_line(
            "hackrf_serial",
            format!(
                "{} (edit: x)",
                app.config
                    .hackrf_serial
                    .as_deref()
                    .unwrap_or("<first device>")
            ),
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
    if let Some(session) = &app.manual_session {
        lines.extend(manual_run_lines(session));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(run_keys(app.manual_session.is_some())));
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
    if let LastRun::Error(message) = last_run {
        lines.push(Line::from(message.clone()));
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
            session
                .actual_position_llh()
                .map_or_else(|| "<unavailable>".to_string(), format_llh),
        ));
    } else {
        lines.push(labeled_line("actual_state", "waiting for snapshot"));
    }

    lines
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

fn display_optional_path(
    path: Option<&std::path::Path>, fallback: &'static str,
) -> String {
    path.map_or_else(
        || fallback.to_string(),
        |value| value.display().to_string(),
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

fn duration_text(app: &App) -> String {
    if app.config.uses_manual_motion() {
        app.config.duration.map_or_else(
            || "ignored in manual mode".to_string(),
            |duration| format!("{duration} (ignored in manual mode)"),
        )
    } else {
        app.config.duration.map_or_else(
            || "<default>".to_string(),
            |duration| duration.to_string(),
        )
    }
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
