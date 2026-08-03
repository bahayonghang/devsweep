use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "devsweep")]
#[command(about = "Plan and execute safety-first developer disk cleanup.")]
#[command(version)]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Command,
}

#[derive(Debug, Subcommand)]
pub(super) enum Command {
    /// Open the interactive terminal UI.
    Tui,
    /// Scan roots and emit a cleanup report.
    Scan(ScanCommand),
    /// Inspect capacity without creating cleanup targets.
    Inventory(InventoryCommand),
    /// Dry-run or execute an existing cleanup plan.
    Clean(CleanCommand),
    /// Manage the persistent user protection list.
    #[command(subcommand)]
    Protect(ProtectCommand),
    /// Inspect cleanup rules.
    Rules,
}

#[derive(Debug, Subcommand)]
pub(super) enum ProtectCommand {
    /// Add an existing path that must never be cleaned.
    Add {
        /// Existing path to protect.
        path: PathBuf,
    },
    /// Remove a path from the protection list.
    Remove {
        /// Path to remove from the protection list.
        path: PathBuf,
    },
    /// List protected paths.
    List,
}

#[derive(Debug, Args)]
pub(super) struct ScanCommand {
    /// Roots to scan.
    #[arg(value_name = "ROOT", default_value = ".")]
    pub(super) roots: Vec<PathBuf>,
    /// Emit the scan report as JSON.
    #[arg(long)]
    pub(super) json: bool,
    /// Include global cache providers.
    #[arg(long)]
    pub(super) global: bool,
    /// Include project-level cleanup targets.
    #[arg(long)]
    pub(super) projects: bool,
    /// Re-estimate one target ID from this scan with a higher bounded budget.
    #[arg(long, value_name = "TARGET_ID")]
    pub(super) rescan_target: Option<String>,
}

impl ScanCommand {
    pub(super) fn scope_label(&self) -> &'static str {
        match (self.global, self.projects) {
            (true, true) | (false, false) => "global+projects",
            (true, false) => "global",
            (false, true) => "projects",
        }
    }
}

#[derive(Debug, Args)]
pub(super) struct InventoryCommand {
    /// Root whose immediate contents should be inventoried.
    #[arg(value_name = "ROOT", default_value = ".")]
    pub(super) root: PathBuf,
    /// Emit the read-only inventory report as JSON.
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Args)]
pub(super) struct CleanCommand {
    /// Cleanup plan or scan report to dry-run or execute.
    #[arg(long, value_name = "PATH")]
    pub(super) plan: Option<PathBuf>,
    /// Execute the plan. Omit this flag for dry-run behavior.
    #[arg(long)]
    pub(super) execute: bool,
    /// Append execution audit records to this JSONL file. Defaults to devsweep-audit.jsonl when executing.
    #[arg(long, value_name = "PATH")]
    pub(super) audit_log: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::{Cli, Command};

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn clean_help_does_not_expose_permanent_delete() {
        let mut command = Cli::command();
        let clean = command
            .find_subcommand_mut("clean")
            .expect("clean subcommand exists");
        let mut help = Vec::new();

        clean.write_long_help(&mut help).expect("help renders");

        assert!(
            !String::from_utf8(help)
                .expect("help is UTF-8")
                .contains("allow-permanent-delete")
        );
    }

    #[test]
    fn scan_rescan_target_parses_an_exact_target_id() {
        let cli = Cli::try_parse_from([
            "devsweep",
            "scan",
            "--projects",
            "--rescan-target",
            "python.pycache:C:/code/app/__pycache__",
            "C:/code",
        ])
        .expect("scan command parses");

        let Command::Scan(command) = cli.command else {
            panic!("expected scan command");
        };
        assert_eq!(
            command.rescan_target.as_deref(),
            Some("python.pycache:C:/code/app/__pycache__")
        );
        assert_eq!(command.roots, vec![std::path::PathBuf::from("C:/code")]);
        assert!(command.projects);
    }
}
