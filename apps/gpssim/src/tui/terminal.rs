use crossterm::{
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::Error;

pub(super) struct TerminalGuard {
    pub(super) terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
}

impl TerminalGuard {
    pub(super) fn new() -> Result<Self, Error> {
        enable_raw_mode()?;

        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;
        terminal.hide_cursor()?;

        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if let Err(err) = self.terminal.show_cursor() {
            tracing::warn!(error = %err, "failed to show cursor");
        }
        if let Err(err) = self.terminal.clear() {
            tracing::warn!(error = %err, "failed to clear terminal");
        }
        if let Err(err) =
            execute!(self.terminal.backend_mut(), LeaveAlternateScreen)
        {
            tracing::warn!(error = %err, "failed to leave alternate screen");
        }
        if let Err(err) = disable_raw_mode() {
            tracing::warn!(error = %err, "failed to disable raw mode");
        }
    }
}
