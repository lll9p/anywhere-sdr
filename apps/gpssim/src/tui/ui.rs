use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
};

use super::{
    app::{ActiveTab, App, InputMode, LastRun, RunState},
    worker::resolve_output_path,
};
use crate::cli::TxBackend;

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
    let ephemerides = app.config.ephemerides.as_ref().map_or_else(
        || "<unset>".to_string(),
        |path| path.display().to_string(),
    );

    let output_effective = resolve_output_path(&app.config).map_or_else(
        || "<none>".to_string(),
        |path| path.display().to_string(),
    );

    let tx_list = if app.config.tx.is_empty() {
        "<none>".to_string()
    } else {
        app.config
            .tx
            .iter()
            .map(|b| match b {
                TxBackend::Hackrf => "hackrf",
                TxBackend::Null => "null",
            })
            .collect::<Vec<_>>()
            .join(",")
    };

    let lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("ephemerides", Style::default().fg(Color::Cyan)),
            Span::raw(": "),
            Span::raw(ephemerides),
        ]),
        Line::from(vec![
            Span::styled("output", Style::default().fg(Color::Cyan)),
            Span::raw(": "),
            Span::raw(output_effective),
        ]),
        Line::from(vec![
            Span::styled("tx", Style::default().fg(Color::Cyan)),
            Span::raw(": "),
            Span::raw(tx_list),
        ]),
        Line::from(vec![
            Span::styled("frequency", Style::default().fg(Color::Cyan)),
            Span::raw(": "),
            Span::raw(app.config.frequency.to_string()),
        ]),
        Line::from(vec![
            Span::styled("bits", Style::default().fg(Color::Cyan)),
            Span::raw(": "),
            Span::raw(app.config.bits.to_string()),
        ]),
        Line::from(vec![
            Span::styled("duration", Style::default().fg(Color::Cyan)),
            Span::raw(": "),
            Span::raw(app.config.duration.map_or_else(
                || "<default>".to_string(),
                |duration| duration.to_string(),
            )),
        ]),
        Line::from(vec![
            Span::styled("hackrf_rf_freq_hz", Style::default().fg(Color::Cyan)),
            Span::raw(": "),
            Span::raw(app.config.hackrf_rf_freq_hz.to_string()),
            Span::raw(" (edit: r)"),
        ]),
        Line::from(vec![
            Span::styled("hackrf_serial", Style::default().fg(Color::Cyan)),
            Span::raw(": "),
            Span::raw(
                app.config
                    .hackrf_serial
                    .clone()
                    .unwrap_or_else(|| "<first device>".to_string()),
            ),
            Span::raw(" (edit: x)"),
        ]),
        Line::from(""),
        Line::from(
            "Keys: Enter=start | q=quit | Tab=switch | e=ephemerides | \
             o=output | h=toggle hackrf | n=toggle null | s=frequency | \
             b=bits | d=duration | v=verbose | i=iono",
        ),
    ];

    let paragraph = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title("Config"))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn render_run(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("state", Style::default().fg(Color::Cyan)),
        Span::raw(": "),
        Span::raw(match app.run_state {
            RunState::Idle => "idle",
            RunState::Running => "running",
            RunState::Stopping => "stopping",
        }),
    ]));
    lines.push(Line::from(vec![
        Span::styled("sinks", Style::default().fg(Color::Cyan)),
        Span::raw(": "),
        Span::raw(app.sinks_desc.clone()),
    ]));

    if let Some(progress) = &app.progress {
        lines.push(Line::from(vec![
            Span::styled("blocks", Style::default().fg(Color::Cyan)),
            Span::raw(": "),
            Span::raw(progress.blocks.to_string()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("elapsed", Style::default().fg(Color::Cyan)),
            Span::raw(": "),
            Span::raw(format!("{:.3}s", progress.elapsed.as_secs_f64())),
        ]));
        lines.push(Line::from(vec![
            Span::styled("sim", Style::default().fg(Color::Cyan)),
            Span::raw(": "),
            Span::raw(format!("{:.3}s", progress.sim_seconds)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("throughput", Style::default().fg(Color::Cyan)),
            Span::raw(": "),
            Span::raw(format!("{:.3} MSps", progress.throughput_msps)),
        ]));
        if let Some(underruns) = progress.hackrf_underruns {
            lines.push(Line::from(vec![
                Span::styled(
                    "hackrf_underruns",
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(": "),
                Span::raw(underruns.to_string()),
            ]));
        }
    }

    if let Some(last_run) = &app.last_run {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("last", Style::default().fg(Color::Cyan)),
            Span::raw(": "),
            Span::raw(match last_run {
                LastRun::Finished => "finished",
                LastRun::Cancelled => "cancelled",
                LastRun::Error(_) => "error",
            }),
        ]));
        if let LastRun::Error(message) = last_run {
            lines.push(Line::from(message.clone()));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from("Keys: c=cancel | q=quit | Tab=switch"));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title("Run"))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
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
