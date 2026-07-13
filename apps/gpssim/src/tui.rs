use std::{
    io::{self, IsTerminal},
    sync::mpsc,
    time::{Duration, Instant},
};

use crossterm::event::{self, Event as CrosstermEvent};

use crate::{Error, cli::Args, tui_config::TuiConfig, utils::LogBuffer};

mod app;
mod config_edit;
mod manual_control;
mod terminal;
mod ui;
mod worker;

const UI_REDRAW_RATE: Duration = Duration::from_millis(100);
const MANUAL_INPUT_POLL_RATE: Duration = Duration::from_millis(25);

pub(crate) fn run(args: &Args, log_buffer: LogBuffer) -> Result<(), Error> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(Error::cli_error(
            "--tui requires an interactive terminal".to_string(),
        ));
    }

    let mut config = TuiConfig::default();
    config.apply_overrides_from_args(args)?;

    let (worker_events_tx, worker_events_rx) = mpsc::channel();
    let mut app =
        app::App::new(config, log_buffer, worker_events_rx, worker_events_tx);

    let mut terminal_guard = terminal::TerminalGuard::new()?;
    let mut last_draw = Instant::now();
    let mut needs_draw = true;

    loop {
        app.drain_worker_events();
        app.refresh_manual_snapshot();

        if needs_draw || last_draw.elapsed() >= UI_REDRAW_RATE {
            terminal_guard.terminal.draw(|frame| ui::ui(frame, &app))?;
            last_draw = Instant::now();
            needs_draw = false;
        }

        if app.should_exit() {
            break;
        }

        let input_poll_rate = if app.prefers_fast_input_poll() {
            MANUAL_INPUT_POLL_RATE
        } else {
            UI_REDRAW_RATE
        };
        let redraw_due_in = UI_REDRAW_RATE.saturating_sub(last_draw.elapsed());
        let poll_timeout =
            input_poll_rate.min(redraw_due_in.max(Duration::ZERO));

        if event::poll(poll_timeout)? {
            loop {
                let next_event = event::read()?;
                if let CrosstermEvent::Key(key) = next_event {
                    app::handle_key_event(&mut app, key);
                    needs_draw = true;
                }

                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }
    }

    Ok(())
}
