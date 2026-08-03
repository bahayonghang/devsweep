#![warn(unreachable_pub)]

//! Command-line and terminal user interfaces for devsweep.

use devsweep_core::{execution, inventory, model, plan, process, rules, scan};

mod application;
mod tui;

/// Runs the command-line application using process arguments.
pub fn run() -> anyhow::Result<()> {
    application::run()
}
