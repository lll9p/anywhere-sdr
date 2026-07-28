use std::io;

use crossterm::{
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{
    Error,
    error::{attach_terminal_cleanup, resolve_terminal_cleanup_failures},
};

const ENABLE_RAW_MODE: &str = "enable raw mode";
const DISABLE_RAW_MODE: &str = "disable raw mode";
const ENTER_ALTERNATE_SCREEN: &str = "enter alternate screen";
const LEAVE_ALTERNATE_SCREEN: &str = "leave alternate screen";
const CREATE_TERMINAL: &str = "create terminal";
const CLEAR_TERMINAL: &str = "clear terminal";
const HIDE_CURSOR: &str = "hide cursor";
const SHOW_CURSOR: &str = "show cursor";

pub(super) trait TerminalEnvironment {
    type Backend: ratatui::backend::Backend<Error = io::Error>;

    fn enable_raw_mode(&mut self) -> io::Result<()>;
    fn disable_raw_mode(&mut self) -> io::Result<()>;
    fn enter_alternate_screen(&mut self) -> io::Result<()>;
    fn leave_alternate_screen(&mut self) -> io::Result<()>;
    fn create_terminal(&mut self) -> io::Result<Terminal<Self::Backend>>;
    fn clear_terminal(
        &mut self, terminal: &mut Terminal<Self::Backend>,
    ) -> io::Result<()>;
    fn hide_cursor(
        &mut self, terminal: &mut Terminal<Self::Backend>,
    ) -> io::Result<()>;
    fn show_cursor(
        &mut self, terminal: &mut Terminal<Self::Backend>,
    ) -> io::Result<()>;
}

pub(super) struct CrosstermEnvironment;

impl TerminalEnvironment for CrosstermEnvironment {
    type Backend = CrosstermBackend<io::Stdout>;

    fn enable_raw_mode(&mut self) -> io::Result<()> {
        enable_raw_mode()
    }

    fn disable_raw_mode(&mut self) -> io::Result<()> {
        disable_raw_mode()
    }

    fn enter_alternate_screen(&mut self) -> io::Result<()> {
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)
    }

    fn leave_alternate_screen(&mut self) -> io::Result<()> {
        let mut stdout = io::stdout();
        execute!(stdout, LeaveAlternateScreen)
    }

    fn create_terminal(&mut self) -> io::Result<Terminal<Self::Backend>> {
        Terminal::new(CrosstermBackend::new(io::stdout()))
    }

    fn clear_terminal(
        &mut self, terminal: &mut Terminal<Self::Backend>,
    ) -> io::Result<()> {
        terminal.clear()
    }

    fn hide_cursor(
        &mut self, terminal: &mut Terminal<Self::Backend>,
    ) -> io::Result<()> {
        terminal.hide_cursor()
    }

    fn show_cursor(
        &mut self, terminal: &mut Terminal<Self::Backend>,
    ) -> io::Result<()> {
        terminal.show_cursor()
    }
}

pub(super) struct TerminalGuard<E: TerminalEnvironment> {
    environment: E,
    terminal: Option<Terminal<E::Backend>>,
    raw_mode_pending: bool,
    alternate_screen_pending: bool,
    clear_pending: bool,
    cursor_show_pending: bool,
}

impl<E: TerminalEnvironment> TerminalGuard<E> {
    pub(super) fn acquire(environment: E) -> Result<Self, Error> {
        let mut guard = Self {
            environment,
            terminal: None,
            raw_mode_pending: false,
            alternate_screen_pending: false,
            clear_pending: false,
            cursor_show_pending: false,
        };

        guard.raw_mode_pending = true;
        if let Err(source) = guard.environment.enable_raw_mode() {
            return Err(guard
                .rollback(Error::terminal_operation(ENABLE_RAW_MODE, source)));
        }

        guard.alternate_screen_pending = true;
        if let Err(source) = guard.environment.enter_alternate_screen() {
            return Err(guard.rollback(Error::terminal_operation(
                ENTER_ALTERNATE_SCREEN,
                source,
            )));
        }

        match guard.environment.create_terminal() {
            Ok(terminal) => guard.terminal = Some(terminal),
            Err(source) => {
                return Err(guard.rollback(Error::terminal_operation(
                    CREATE_TERMINAL,
                    source,
                )));
            }
        }

        guard.clear_pending = true;
        let clear_result = match guard.terminal.as_mut() {
            Some(terminal) => guard.environment.clear_terminal(terminal),
            None => Err(io::Error::other("terminal is not acquired")),
        };
        if let Err(source) = clear_result {
            return Err(guard
                .rollback(Error::terminal_operation(CLEAR_TERMINAL, source)));
        }

        guard.cursor_show_pending = true;
        let hide_result = match guard.terminal.as_mut() {
            Some(terminal) => guard.environment.hide_cursor(terminal),
            None => Err(io::Error::other("terminal is not acquired")),
        };
        if let Err(source) = hide_result {
            return Err(
                guard.rollback(Error::terminal_operation(HIDE_CURSOR, source))
            );
        }

        Ok(guard)
    }

    pub(super) fn terminal_mut(
        &mut self,
    ) -> Result<&mut Terminal<E::Backend>, Error> {
        self.terminal
            .as_mut()
            .ok_or_else(|| Error::msg("terminal is not acquired"))
    }

    pub(super) fn restore(&mut self) -> Result<(), Error> {
        let mut failures = Vec::new();

        if self.cursor_show_pending {
            let result = match self.terminal.as_mut() {
                Some(terminal) => self.environment.show_cursor(terminal),
                None => Err(io::Error::other("terminal is not acquired")),
            };
            match result {
                Ok(()) => self.cursor_show_pending = false,
                Err(source) => failures
                    .push(Error::terminal_operation(SHOW_CURSOR, source)),
            }
        }

        if self.clear_pending {
            let result = match self.terminal.as_mut() {
                Some(terminal) => self.environment.clear_terminal(terminal),
                None => Err(io::Error::other("terminal is not acquired")),
            };
            match result {
                Ok(()) => self.clear_pending = false,
                Err(source) => failures
                    .push(Error::terminal_operation(CLEAR_TERMINAL, source)),
            }
        }

        if self.alternate_screen_pending {
            match self.environment.leave_alternate_screen() {
                Ok(()) => self.alternate_screen_pending = false,
                Err(source) => failures.push(Error::terminal_operation(
                    LEAVE_ALTERNATE_SCREEN,
                    source,
                )),
            }
        }

        if self.raw_mode_pending {
            match self.environment.disable_raw_mode() {
                Ok(()) => self.raw_mode_pending = false,
                Err(source) => failures
                    .push(Error::terminal_operation(DISABLE_RAW_MODE, source)),
            }
        }

        resolve_terminal_cleanup_failures(failures)
    }

    fn rollback(&mut self, primary: Error) -> Error {
        let cleanup = self.restore();
        attach_terminal_cleanup(primary, cleanup)
    }
}

impl<E: TerminalEnvironment> Drop for TerminalGuard<E> {
    fn drop(&mut self) {
        if let Err(error) = self.restore() {
            tracing::warn!(%error, "failed to restore terminal during drop");
        }
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
