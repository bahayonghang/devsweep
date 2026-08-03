use anyhow::Result;
use clap::Parser;

use self::cli::{Cli, Command};

mod cli;
mod commands;

pub(crate) fn run() -> Result<()> {
    init_tracing();

    match Cli::parse().command {
        Command::Tui => crate::tui::run(),
        Command::Scan(command) => commands::run_scan(command),
        Command::Inventory(command) => commands::run_inventory(command),
        Command::Clean(command) => commands::run_clean(command),
        Command::Protect(command) => commands::run_protect(command),
        Command::Rules => commands::run_rules(),
    }
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .try_init();
}
