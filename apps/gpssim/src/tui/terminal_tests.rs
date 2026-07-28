use std::{
    collections::{HashMap, VecDeque},
    error::Error as StdError,
    io,
    sync::{Arc, Mutex},
    time::Duration,
};

use crossterm::event::{
    Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers,
};
use ratatui::{
    Terminal,
    backend::{Backend, ClearType, WindowSize},
    buffer::Cell,
    layout::{Position, Size},
};

use super::*;
use crate::{
    tui::{
        DRAW_TERMINAL, EventSource, POLL_TERMINAL_EVENTS, READ_TERMINAL_EVENT,
        run_with_environment,
    },
    tui_config::TuiConfig,
    utils::LogBuffer,
};

const DRAW: &str = "backend draw";
const FLUSH: &str = "backend flush";
const GET_CURSOR: &str = "get cursor";
const SET_CURSOR: &str = "set cursor";
const WINDOW_SIZE: &str = "window size";

#[derive(Default)]
struct FakeState {
    operations: Vec<&'static str>,
    call_counts: HashMap<&'static str, usize>,
    failures: Vec<(&'static str, usize)>,
}

impl FakeState {
    fn record(&mut self, operation: &'static str) -> io::Result<()> {
        self.operations.push(operation);
        let count = self.call_counts.entry(operation).or_default();
        *count += 1;
        if let Some(index) = self
            .failures
            .iter()
            .position(|failure| *failure == (operation, *count))
        {
            self.failures.remove(index);
            Err(io::Error::other(format!("injected {operation} failure")))
        } else {
            Ok(())
        }
    }
}

type SharedState = Arc<Mutex<FakeState>>;

fn with_state<T>(
    state: &SharedState, action: impl FnOnce(&mut FakeState) -> T,
) -> io::Result<T> {
    let mut state = state
        .lock()
        .map_err(|_| io::Error::other("fake terminal state poisoned"))?;
    Ok(action(&mut state))
}

fn fail_on(
    state: &SharedState, operation: &'static str, call: usize,
) -> io::Result<()> {
    with_state(state, |state| state.failures.push((operation, call)))
}

fn fail_next(state: &SharedState, operation: &'static str) -> io::Result<()> {
    with_state(state, |state| {
        let next = state.call_counts.get(operation).copied().unwrap_or(0) + 1;
        state.failures.push((operation, next));
    })
}

fn operations(state: &SharedState) -> io::Result<Vec<&'static str>> {
    with_state(state, |state| state.operations.clone())
}

struct FakeBackend {
    state: SharedState,
    cursor: Position,
}

impl Backend for FakeBackend {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        content.for_each(drop);
        with_state(&self.state, |state| state.record(DRAW))?
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        with_state(&self.state, |state| state.record(HIDE_CURSOR))?
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        with_state(&self.state, |state| state.record(SHOW_CURSOR))?
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        with_state(&self.state, |state| state.record(GET_CURSOR))??;
        Ok(self.cursor)
    }

    fn set_cursor_position<P: Into<Position>>(
        &mut self, position: P,
    ) -> io::Result<()> {
        with_state(&self.state, |state| state.record(SET_CURSOR))??;
        self.cursor = position.into();
        Ok(())
    }

    fn clear(&mut self) -> io::Result<()> {
        with_state(&self.state, |state| state.record(CLEAR_TERMINAL))?
    }

    fn clear_region(&mut self, _clear_type: ClearType) -> io::Result<()> {
        self.clear()
    }

    fn size(&self) -> io::Result<Size> {
        with_state(&self.state, |state| state.record(CREATE_TERMINAL))??;
        Ok(Size::new(120, 40))
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        with_state(&self.state, |state| state.record(WINDOW_SIZE))??;
        Ok(WindowSize {
            columns_rows: Size::new(120, 40),
            pixels: Size::new(0, 0),
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        with_state(&self.state, |state| state.record(FLUSH))?
    }
}

struct FakeEnvironment {
    state: SharedState,
}

impl FakeEnvironment {
    fn new(state: SharedState) -> Self {
        Self { state }
    }

    fn record(&self, operation: &'static str) -> io::Result<()> {
        with_state(&self.state, |state| state.record(operation))?
    }
}

impl TerminalEnvironment for FakeEnvironment {
    type Backend = FakeBackend;

    fn enable_raw_mode(&mut self) -> io::Result<()> {
        self.record(ENABLE_RAW_MODE)
    }

    fn disable_raw_mode(&mut self) -> io::Result<()> {
        self.record(DISABLE_RAW_MODE)
    }

    fn enter_alternate_screen(&mut self) -> io::Result<()> {
        self.record(ENTER_ALTERNATE_SCREEN)
    }

    fn leave_alternate_screen(&mut self) -> io::Result<()> {
        self.record(LEAVE_ALTERNATE_SCREEN)
    }

    fn create_terminal(&mut self) -> io::Result<Terminal<Self::Backend>> {
        Terminal::new(FakeBackend {
            state: self.state.clone(),
            cursor: Position::ORIGIN,
        })
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

struct FakeEventSource {
    polls: VecDeque<io::Result<bool>>,
    events: VecDeque<io::Result<CrosstermEvent>>,
}

impl FakeEventSource {
    fn new(
        polls: impl IntoIterator<Item = io::Result<bool>>,
        events: impl IntoIterator<Item = io::Result<CrosstermEvent>>,
    ) -> Self {
        Self {
            polls: polls.into_iter().collect(),
            events: events.into_iter().collect(),
        }
    }
}

impl EventSource for FakeEventSource {
    fn poll(&mut self, _timeout: Duration) -> io::Result<bool> {
        self.polls.pop_front().unwrap_or_else(|| {
            Err(io::Error::other("missing fake poll result"))
        })
    }

    fn read(&mut self) -> io::Result<CrosstermEvent> {
        self.events.pop_front().unwrap_or_else(|| {
            Err(io::Error::other("missing fake terminal event"))
        })
    }
}

fn operation_label(error: &Error) -> Option<&'static str> {
    match error {
        Error::TerminalOperation { operation, .. } => Some(*operation),
        _ => None,
    }
}

#[test]
fn acquisition_failures_restore_every_attempted_transition() -> Result<(), Error>
{
    let cases = [
        (ENABLE_RAW_MODE, vec![ENABLE_RAW_MODE, DISABLE_RAW_MODE]),
        (ENTER_ALTERNATE_SCREEN, vec![
            ENABLE_RAW_MODE,
            ENTER_ALTERNATE_SCREEN,
            LEAVE_ALTERNATE_SCREEN,
            DISABLE_RAW_MODE,
        ]),
        (CREATE_TERMINAL, vec![
            ENABLE_RAW_MODE,
            ENTER_ALTERNATE_SCREEN,
            CREATE_TERMINAL,
            LEAVE_ALTERNATE_SCREEN,
            DISABLE_RAW_MODE,
        ]),
        (CLEAR_TERMINAL, vec![
            ENABLE_RAW_MODE,
            ENTER_ALTERNATE_SCREEN,
            CREATE_TERMINAL,
            CLEAR_TERMINAL,
            CLEAR_TERMINAL,
            LEAVE_ALTERNATE_SCREEN,
            DISABLE_RAW_MODE,
        ]),
        (HIDE_CURSOR, vec![
            ENABLE_RAW_MODE,
            ENTER_ALTERNATE_SCREEN,
            CREATE_TERMINAL,
            CLEAR_TERMINAL,
            HIDE_CURSOR,
            SHOW_CURSOR,
            CLEAR_TERMINAL,
            LEAVE_ALTERNATE_SCREEN,
            DISABLE_RAW_MODE,
        ]),
    ];

    for (failure, expected) in cases {
        let state = SharedState::default();
        fail_on(&state, failure, 1)?;
        let Err(error) =
            TerminalGuard::acquire(FakeEnvironment::new(state.clone()))
        else {
            return Err(Error::msg(format!(
                "{failure} failure was not propagated"
            )));
        };
        assert_eq!(operation_label(&error), Some(failure));
        assert_eq!(operations(&state)?, expected);
    }
    Ok(())
}

#[test]
fn restoration_is_reverse_ordered_exhaustive_and_retried_by_drop()
-> Result<(), Error> {
    let state = SharedState::default();
    let mut guard =
        TerminalGuard::acquire(FakeEnvironment::new(state.clone()))?;
    for operation in [
        SHOW_CURSOR,
        CLEAR_TERMINAL,
        LEAVE_ALTERNATE_SCREEN,
        DISABLE_RAW_MODE,
    ] {
        fail_next(&state, operation)?;
    }

    let Err(error) = guard.restore() else {
        return Err(Error::msg("cleanup failures were discarded"));
    };
    let Error::MultipleTerminalCleanupFailures { first, additional } = error
    else {
        return Err(Error::msg("expected ordered cleanup aggregate"));
    };
    assert_eq!(operation_label(&first), Some(SHOW_CURSOR));
    assert_eq!(
        additional
            .iter()
            .filter_map(operation_label)
            .collect::<Vec<_>>(),
        vec![CLEAR_TERMINAL, LEAVE_ALTERNATE_SCREEN, DISABLE_RAW_MODE]
    );
    let before_drop = operations(&state)?;
    assert_eq!(&before_drop[before_drop.len().saturating_sub(4)..], [
        SHOW_CURSOR,
        CLEAR_TERMINAL,
        LEAVE_ALTERNATE_SCREEN,
        DISABLE_RAW_MODE
    ]);

    drop(guard);
    let after_drop = operations(&state)?;
    assert_eq!(&after_drop[after_drop.len().saturating_sub(4)..], [
        SHOW_CURSOR,
        CLEAR_TERMINAL,
        LEAVE_ALTERNATE_SCREEN,
        DISABLE_RAW_MODE
    ]);
    Ok(())
}

#[test]
fn drop_retries_only_cleanup_that_remains_armed() -> Result<(), Error> {
    let state = SharedState::default();
    let mut guard =
        TerminalGuard::acquire(FakeEnvironment::new(state.clone()))?;
    fail_next(&state, SHOW_CURSOR)?;
    assert!(guard.restore().is_err());
    let before_drop = operations(&state)?.len();

    drop(guard);

    assert_eq!(&operations(&state)?[before_drop..], [SHOW_CURSOR]);
    Ok(())
}

#[test]
fn initialization_error_remains_primary_when_cleanup_fails() -> Result<(), Error>
{
    let state = SharedState::default();
    fail_on(&state, CREATE_TERMINAL, 1)?;
    fail_on(&state, LEAVE_ALTERNATE_SCREEN, 1)?;

    let Err(error) = TerminalGuard::acquire(FakeEnvironment::new(state)) else {
        return Err(Error::msg("initialization failure was discarded"));
    };
    let Some(source) = StdError::source(&error) else {
        return Err(Error::msg("combined error has no primary source"));
    };
    assert!(source.to_string().contains(CREATE_TERMINAL));
    let Error::TuiAndTerminalCleanupFailed { primary, cleanup } = error else {
        return Err(Error::msg("expected typed primary/cleanup error"));
    };
    assert_eq!(operation_label(&primary), Some(CREATE_TERMINAL));
    assert_eq!(operation_label(&cleanup), Some(LEAVE_ALTERNATE_SCREEN));
    Ok(())
}

#[test]
fn successful_event_loop_restores_terminal_without_mouse_capture()
-> Result<(), Error> {
    let state = SharedState::default();
    let event_source =
        FakeEventSource::new([Ok(true), Ok(false)], [Ok(CrosstermEvent::Key(
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
        ))]);

    run_with_environment(
        TuiConfig::default(),
        LogBuffer::new(8),
        FakeEnvironment::new(state.clone()),
        event_source,
    )?;

    let operations = operations(&state)?;
    assert_eq!(&operations[operations.len().saturating_sub(4)..], [
        SHOW_CURSOR,
        CLEAR_TERMINAL,
        LEAVE_ALTERNATE_SCREEN,
        DISABLE_RAW_MODE
    ]);
    assert!(
        operations
            .iter()
            .all(|operation| !operation.contains("mouse"))
    );
    Ok(())
}

fn assert_event_loop_primary_and_cleanup(
    expected_primary: &'static str, state: SharedState,
    event_source: FakeEventSource,
) -> Result<(), Error> {
    fail_on(&state, SHOW_CURSOR, 1)?;
    fail_on(&state, CLEAR_TERMINAL, 2)?;
    fail_on(&state, LEAVE_ALTERNATE_SCREEN, 1)?;
    fail_on(&state, DISABLE_RAW_MODE, 1)?;

    let Err(error) = run_with_environment(
        TuiConfig::default(),
        LogBuffer::new(8),
        FakeEnvironment::new(state),
        event_source,
    ) else {
        return Err(Error::msg("event loop failure was discarded"));
    };
    let Error::TuiAndTerminalCleanupFailed { primary, cleanup } = error else {
        return Err(Error::msg("expected typed TUI/cleanup error"));
    };
    assert_eq!(operation_label(&primary), Some(expected_primary));
    let Error::MultipleTerminalCleanupFailures { first, additional } = *cleanup
    else {
        return Err(Error::msg("expected all cleanup failures"));
    };
    assert_eq!(operation_label(&first), Some(SHOW_CURSOR));
    assert_eq!(additional.len(), 3);
    Ok(())
}

#[test]
fn draw_poll_and_read_errors_reach_explicit_restoration() -> Result<(), Error> {
    let draw_state = SharedState::default();
    fail_on(&draw_state, DRAW, 1)?;
    assert_event_loop_primary_and_cleanup(
        DRAW_TERMINAL,
        draw_state,
        FakeEventSource::new([], []),
    )?;

    assert_event_loop_primary_and_cleanup(
        POLL_TERMINAL_EVENTS,
        SharedState::default(),
        FakeEventSource::new(
            [Err(io::Error::other("injected poll failure"))],
            [],
        ),
    )?;

    assert_event_loop_primary_and_cleanup(
        POLL_TERMINAL_EVENTS,
        SharedState::default(),
        FakeEventSource::new(
            [
                Ok(true),
                Err(io::Error::other("injected drain poll failure")),
            ],
            [Ok(CrosstermEvent::FocusGained)],
        ),
    )?;

    assert_event_loop_primary_and_cleanup(
        READ_TERMINAL_EVENT,
        SharedState::default(),
        FakeEventSource::new([Ok(true)], [Err(io::Error::other(
            "injected read failure",
        ))]),
    )?;
    Ok(())
}
