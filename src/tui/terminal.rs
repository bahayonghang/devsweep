use std::{
    io::{self, Write},
    sync::OnceLock,
};

use anyhow::{Context, Result};
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

pub(super) type Tui = Terminal<CrosstermBackend<io::Stdout>>;

/// Injectable terminal control surface used by unit tests.
pub(super) trait TerminalControl: Send {
    fn enable_raw_mode(&mut self) -> Result<()>;
    fn enter_alternate_screen(&mut self) -> Result<()>;
    fn disable_raw_mode(&mut self) -> Result<()>;
    fn leave_alternate_screen(&mut self) -> Result<()>;
    fn show_cursor(&mut self) -> Result<()>;
}

#[derive(Debug, Default)]
struct CrosstermControl;

impl TerminalControl for CrosstermControl {
    fn enable_raw_mode(&mut self) -> Result<()> {
        enable_raw_mode().context("failed to enable raw mode")
    }

    fn enter_alternate_screen(&mut self) -> Result<()> {
        execute!(io::stdout(), EnterAlternateScreen).context("failed to enter alternate screen")
    }

    fn disable_raw_mode(&mut self) -> Result<()> {
        disable_raw_mode().context("failed to disable raw mode")
    }

    fn leave_alternate_screen(&mut self) -> Result<()> {
        execute!(io::stdout(), LeaveAlternateScreen).context("failed to leave alternate screen")
    }

    fn show_cursor(&mut self) -> Result<()> {
        execute!(io::stdout(), crossterm::cursor::Show).context("failed to show cursor")
    }
}

#[derive(Debug)]
struct SessionFlags {
    raw_mode: bool,
    alternate_screen: bool,
    hide_cursor: bool,
}

/// RAII terminal owner. Construction enters; Drop restores best-effort.
pub(super) struct TerminalSession {
    terminal: Option<Tui>,
    control: Box<dyn TerminalControl>,
    flags: SessionFlags,
    restored: bool,
}

impl TerminalSession {
    pub(super) fn enter() -> Result<Self> {
        Self::enter_with(Box::new(CrosstermControl), true)
    }

    fn enter_with(mut control: Box<dyn TerminalControl>, build_terminal: bool) -> Result<Self> {
        let mut flags = SessionFlags {
            raw_mode: false,
            alternate_screen: false,
            hide_cursor: false,
        };

        if let Err(error) = control.enable_raw_mode() {
            let _ = control.disable_raw_mode();
            return Err(error);
        }
        flags.raw_mode = true;

        if let Err(error) = control.enter_alternate_screen() {
            let _ = control.disable_raw_mode();
            return Err(error);
        }
        flags.alternate_screen = true;

        let terminal = if build_terminal {
            let backend = CrosstermBackend::new(io::stdout());
            match Terminal::new(backend) {
                Ok(mut terminal) => {
                    if terminal.hide_cursor().is_ok() {
                        flags.hide_cursor = true;
                    }
                    Some(terminal)
                }
                Err(error) => {
                    let mut session = Self {
                        terminal: None,
                        control,
                        flags,
                        restored: false,
                    };
                    session.restore_best_effort();
                    return Err(error).context("failed to initialize terminal");
                }
            }
        } else {
            None
        };

        Ok(Self {
            terminal,
            control,
            flags,
            restored: false,
        })
    }

    pub(super) fn terminal_mut(&mut self) -> Option<&mut Tui> {
        self.terminal.as_mut()
    }

    pub(super) fn restore_best_effort(&mut self) {
        if self.restored {
            return;
        }
        self.restored = true;

        // Each step is independent; earlier failures do not skip later ones.
        if self.flags.raw_mode {
            let _ = self.control.disable_raw_mode();
            self.flags.raw_mode = false;
        }
        if self.flags.alternate_screen {
            let _ = self.control.leave_alternate_screen();
            self.flags.alternate_screen = false;
        }
        if self.flags.hide_cursor || self.terminal.is_some() {
            if let Some(terminal) = self.terminal.as_mut() {
                let _ = terminal.show_cursor();
            }
            let _ = self.control.show_cursor();
            self.flags.hide_cursor = false;
        }
        let _ = io::stdout().flush();
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        self.restore_best_effort();
    }
}

static PANIC_HOOK_INSTALLED: OnceLock<()> = OnceLock::new();

/// Install a panic hook that best-effort restores the terminal before the
/// previous hook runs. Idempotent across process lifetime.
pub(super) fn install_panic_hook() {
    PANIC_HOOK_INSTALLED.get_or_init(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // Always try crossterm restore as a last resort.
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen, crossterm::cursor::Show);
            previous(info);
        }));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    #[derive(Clone, Default)]
    struct FakeControl {
        enable_raw_calls: Arc<AtomicUsize>,
        enter_alt_calls: Arc<AtomicUsize>,
        disable_raw_calls: Arc<AtomicUsize>,
        leave_alt_calls: Arc<AtomicUsize>,
        show_cursor_calls: Arc<AtomicUsize>,
        fail_enable_raw: Arc<AtomicBool>,
        fail_enter_alt: Arc<AtomicBool>,
        fail_disable_raw: Arc<AtomicBool>,
        fail_leave_alt: Arc<AtomicBool>,
    }

    impl TerminalControl for FakeControl {
        fn enable_raw_mode(&mut self) -> Result<()> {
            self.enable_raw_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_enable_raw.load(Ordering::SeqCst) {
                anyhow::bail!("enable raw failed");
            }
            Ok(())
        }

        fn enter_alternate_screen(&mut self) -> Result<()> {
            self.enter_alt_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_enter_alt.load(Ordering::SeqCst) {
                anyhow::bail!("enter alt failed");
            }
            Ok(())
        }

        fn disable_raw_mode(&mut self) -> Result<()> {
            self.disable_raw_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_disable_raw.load(Ordering::SeqCst) {
                anyhow::bail!("disable raw failed");
            }
            Ok(())
        }

        fn leave_alternate_screen(&mut self) -> Result<()> {
            self.leave_alt_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_leave_alt.load(Ordering::SeqCst) {
                anyhow::bail!("leave alt failed");
            }
            Ok(())
        }

        fn show_cursor(&mut self) -> Result<()> {
            self.show_cursor_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn partial_enter_failure_restores_completed_steps() {
        let control = FakeControl {
            fail_enter_alt: Arc::new(AtomicBool::new(true)),
            ..FakeControl::default()
        };
        let disable = Arc::clone(&control.disable_raw_calls);
        let result = TerminalSession::enter_with(Box::new(control), false);
        assert!(result.is_err());
        assert_eq!(disable.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn restore_continues_after_first_failure() {
        let control = FakeControl {
            fail_disable_raw: Arc::new(AtomicBool::new(true)),
            ..FakeControl::default()
        };
        let leave = Arc::clone(&control.leave_alt_calls);
        let show = Arc::clone(&control.show_cursor_calls);
        let mut session = TerminalSession::enter_with(Box::new(control), false).expect("enter");
        session.flags.hide_cursor = true;
        session.restore_best_effort();
        assert_eq!(leave.load(Ordering::SeqCst), 1);
        assert_eq!(show.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn drop_restores_session() {
        let control = FakeControl::default();
        let disable = Arc::clone(&control.disable_raw_calls);
        let leave = Arc::clone(&control.leave_alt_calls);
        {
            let _session = TerminalSession::enter_with(Box::new(control), false).expect("enter");
        }
        assert_eq!(disable.load(Ordering::SeqCst), 1);
        assert_eq!(leave.load(Ordering::SeqCst), 1);
    }
}
