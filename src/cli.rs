use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "devsweep")]
#[command(about = "Plan and execute safety-first developer disk cleanup.")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Open the interactive terminal UI.
    Tui,
    /// Scan roots and emit a cleanup plan.
    Scan(ScanCommand),
    /// Dry-run or execute an existing cleanup plan.
    Clean(CleanCommand),
    /// Inspect cleanup rules.
    Rules,
}

#[derive(Debug, Args)]
pub struct ScanCommand {
    /// Roots to scan.
    #[arg(value_name = "ROOT", default_value = ".")]
    pub roots: Vec<PathBuf>,
    /// Emit the cleanup plan as JSON.
    #[arg(long)]
    pub json: bool,
    /// Include global cache providers.
    #[arg(long)]
    pub global: bool,
    /// Include project-level cleanup targets.
    #[arg(long)]
    pub projects: bool,
}

impl ScanCommand {
    pub fn scope_label(&self) -> &'static str {
        match (self.global, self.projects) {
            (true, true) | (false, false) => "global+projects",
            (true, false) => "global",
            (false, true) => "projects",
        }
    }
}

#[derive(Debug, Args)]
pub struct CleanCommand {
    /// Cleanup plan to dry-run or execute.
    #[arg(long, value_name = "PATH")]
    pub plan: Option<PathBuf>,
    /// Execute the plan. Omit this flag for dry-run behavior.
    #[arg(long)]
    pub execute: bool,
    /// Future guard for permanent delete. Not used in the foundation build.
    #[arg(long)]
    pub allow_permanent_delete: bool,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
