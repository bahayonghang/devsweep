mod app;
mod display;
mod render;
mod runtime;
mod terminal;
#[cfg(test)]
mod test_support;

use anyhow::Result;

use self::runtime::{ExecutorCleanService, LocalInventoryService, SweepScanService};

pub(crate) fn run() -> Result<()> {
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
    );
    session.restore_best_effort();
    loop_result
}
