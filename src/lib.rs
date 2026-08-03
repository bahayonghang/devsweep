#![warn(unreachable_pub)]

//! Binary-oriented interface for the devsweep application.

mod application;
mod cargo_metadata;
mod execution;
mod filesystem;
mod inventory;
mod model;
mod plan;
mod process;
mod rules;
mod scan;
mod tui;

/// Parses the process command line and runs the selected devsweep command.
pub fn run() -> anyhow::Result<()> {
    application::run()
}
