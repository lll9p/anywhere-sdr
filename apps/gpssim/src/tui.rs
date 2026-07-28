use std::{
    io::{self, IsTerminal},
    time::{Duration, Instant},
};

use crossterm::event::{self, Event as CrosstermEvent};
use ratatui::{Terminal, backend::Backend};

use crate::{
    Error, cli::Args, error::resolve_tui_and_terminal_cleanup,
    tui_config::TuiConfig, utils::LogBuffer,
};

mod app;
mod app_worker;
mod config_edit;
mod manual_control;
mod terminal;
mod ui;
mod worker;

const UI_REDRAW_RATE: Duration = Duration::from_millis(100);
const MANUAL_INPUT_POLL_RATE: Duration = Duration::from_millis(25);
const DRAW_TERMINAL: &str = "draw terminal";
const POLL_TERMINAL_EVENTS: &str = "poll terminal events";
const READ_TERMINAL_EVENT: &str = "read terminal event";

trait EventSource {
    fn poll(&mut self, timeout: Duration) -> io::Result<bool>;
    fn read(&mut self) -> io::Result<CrosstermEvent>;
}

struct CrosstermEventSource;

impl EventSource for CrosstermEventSource {
    fn poll(&mut self, timeout: Duration) -> io::Result<bool> {
        event::poll(timeout)
    }

    fn read(&mut self) -> io::Result<CrosstermEvent> {
        event::read()
    }
}

pub(crate) fn run(args: &Args, log_buffer: LogBuffer) -> Result<(), Error> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(Error::cli_error(
            "--tui requires an interactive terminal".to_string(),
        ));
    }

    let mut config = TuiConfig::default();
    config.apply_overrides_from_args(args)?;

    run_with_environment(
        config,
        log_buffer,
        terminal::CrosstermEnvironment,
        CrosstermEventSource,
    )
}

fn run_with_environment<T, E>(
    config: TuiConfig, log_buffer: LogBuffer, environment: T,
    mut event_source: E,
) -> Result<(), Error>
where
    T: terminal::TerminalEnvironment,
    E: EventSource,
{
    let mut app = app::App::new(config, log_buffer);
    let mut terminal_guard = terminal::TerminalGuard::acquire(environment)?;
    let tui_result = match terminal_guard.terminal_mut() {
        Ok(terminal) => run_event_loop(terminal, &mut app, &mut event_source),
        Err(error) => Err(error),
    };
    let cleanup_result = terminal_guard.restore();
    resolve_tui_and_terminal_cleanup(tui_result, cleanup_result)
}

fn run_event_loop<B, E>(
    terminal: &mut Terminal<B>, app: &mut app::App, event_source: &mut E,
) -> Result<(), Error>
where
    B: Backend<Error = io::Error>,
    E: EventSource,
{
    let mut last_draw = Instant::now();
    let mut needs_draw = true;

    loop {
        app.drain_worker_events();
        app.refresh_manual_snapshot();

        if needs_draw || last_draw.elapsed() >= UI_REDRAW_RATE {
            terminal
                .draw(|frame| ui::ui(frame, app))
                .map_err(|source| {
                    Error::terminal_operation(DRAW_TERMINAL, source)
                })?;
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

        if event_source.poll(poll_timeout).map_err(|source| {
            Error::terminal_operation(POLL_TERMINAL_EVENTS, source)
        })? {
            loop {
                let next_event = event_source.read().map_err(|source| {
                    Error::terminal_operation(READ_TERMINAL_EVENT, source)
                })?;
                if let CrosstermEvent::Key(key) = next_event {
                    app::handle_key_event(app, key);
                    needs_draw = true;
                }

                if !event_source.poll(Duration::ZERO).map_err(|source| {
                    Error::terminal_operation(POLL_TERMINAL_EVENTS, source)
                })? {
                    break;
                }
            }
        }
    }

    Ok(())
}
