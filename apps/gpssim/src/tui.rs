use std::{
    io::{self, IsTerminal},
    sync::mpsc,
    time::Duration,
};

use crossterm::event::{self, Event as CrosstermEvent};

use crate::{Error, cli::Args, tui_config::TuiConfig, utils::LogBuffer};

mod app;
mod terminal;
mod ui;
mod worker;

const UI_TICK_RATE: Duration = Duration::from_millis(100);

pub(crate) fn run(args: &Args, log_buffer: LogBuffer) -> Result<(), Error> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(Error::cli_error(
            "--tui requires an interactive terminal".to_string(),
        ));
    }

    let mut config = TuiConfig::default();
    config.apply_overrides_from_args(args);

    let (worker_events_tx, worker_events_rx) = mpsc::channel();
    let mut app =
        app::App::new(config, log_buffer, worker_events_rx, worker_events_tx);

    let mut terminal_guard = terminal::TerminalGuard::new()?;

    loop {
        app.drain_worker_events();

        terminal_guard.terminal.draw(|frame| ui::ui(frame, &app))?;

        if app.should_exit() {
            break;
        }

        if event::poll(UI_TICK_RATE)? {
            loop {
                let next_event = event::read()?;
                if let CrosstermEvent::Key(key) = next_event {
                    app::handle_key_event(&mut app, key);
                }

                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }
    }

    Ok(())
}
