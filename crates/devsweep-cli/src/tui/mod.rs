mod app;
mod display;
mod render;
mod runtime;
mod shell;
mod terminal;
#[cfg(test)]
mod test_support;

use anyhow::Result;

use self::runtime::{ExecutorCleanService, LocalInventoryService, SweepScanService};
use crate::i18n::Locale;

pub(crate) fn run(explicit_locale: Option<Locale>) -> Result<()> {
    let shell = shell::TuiShell::load(explicit_locale)?;
    let composition = shell.compose()?;
    terminal::install_panic_hook();
    let mut session = terminal::TerminalSession::enter()?;
    let terminal = session
        .terminal_mut()
        .expect("live terminal session owns a backend");
    let loop_result = runtime::run_event_loop(
        terminal,
        SweepScanService,
        LocalInventoryService,
        ExecutorCleanService,
        composition,
    );
    session.restore_best_effort();
    loop_result
}
