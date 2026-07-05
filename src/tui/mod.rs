mod app;
mod render;
mod runtime;
mod terminal;
#[cfg(test)]
mod test_support;

use anyhow::Result;

use self::runtime::{ExecutorCleanService, SweepScanService};

pub fn run() -> Result<()> {
    let mut terminal = terminal::enter()?;
    let loop_result =
        runtime::run_event_loop(&mut terminal, SweepScanService, ExecutorCleanService);
    let restore_result = terminal::restore_terminal(&mut terminal);

    restore_result?;
    loop_result
}
