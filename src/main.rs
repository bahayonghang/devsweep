use std::path::Path;

use anyhow::{Context, Result};
use clap::Parser;
use devsweep::{
    cli::{CleanCommand, Cli, Command, ProtectCommand, ScanCommand},
    config,
    executor::{ExecutionRequest, Executor},
    model::{LEGACY_CLEANUP_PLAN_VERSION, UntrustedPlan},
    plan_validation::{V1_RESCAN_MESSAGE, untrusted_plan_from_scan, validate_plan},
    rules::RuleScope,
    safety::UserProtectionList,
    sweep::{ScanOptions, Sweeper},
};

fn main() -> Result<()> {
    config::init_tracing();

    match Cli::parse().command {
        Command::Tui => devsweep::tui::run(),
        Command::Scan(command) => run_scan(command),
        Command::Clean(command) => run_clean(command),
        Command::Protect(command) => run_protect(command),
        Command::Rules => run_rules(),
    }
}

fn run_scan(command: ScanCommand) -> Result<()> {
    let options = ScanOptions {
        include_projects: command.projects || !command.global,
        include_global: command.global || !command.projects,
        roots: command.roots.clone(),
    };
    let plan = Sweeper::default().full_scan(&options, &mut |_| {})?;

    if command.json {
        let untrusted = untrusted_plan_from_scan(&plan)?;
        serde_json::to_writer_pretty(std::io::stdout(), &untrusted)?;
        println!();
        return Ok(());
    }

    if options.include_projects {
        println!(
            "Scanner found {} cleanup target(s) from {} root(s), scope {}.",
            plan.targets.len(),
            command.roots.len(),
            command.scope_label()
        );
    } else {
        println!(
            "Scanner found {} global cleanup target(s), scope {}.",
            plan.targets.len(),
            command.scope_label()
        );
    }
    println!("Run with --json to emit the cleanup plan.");

    Ok(())
}

fn run_clean(command: CleanCommand) -> Result<()> {
    if command.execute && command.plan.is_none() {
        anyhow::bail!("cleanup execution requires --plan PATH");
    }

    let untrusted = match command.plan.as_deref() {
        Some(path) => read_untrusted_plan(path)?,
        None => UntrustedPlan::empty(),
    };
    let plan = validate_plan(&untrusted)?;
    let report = Executor::default().run_plan(
        &plan,
        ExecutionRequest {
            execute: command.execute,
            audit_log: command.audit_log,
            selected: plan.default_selected_ids(),
            cancel: None,
        },
    )?;

    if report.dry_run {
        println!(
            "Clean dry-run: {} selected target(s). No cleanup actions were executed.",
            report.selected
        );
        return Ok(());
    }

    println!(
        "Clean execution finished: {} succeeded, {} failed, {} skipped.",
        report.succeeded, report.failed, report.skipped
    );
    if let Some(path) = &report.audit_log {
        println!("Audit log: {}", path.display());
    }
    if report.has_failures() {
        anyhow::bail!("cleanup finished with {} failure(s)", report.failed);
    }
    Ok(())
}

fn read_untrusted_plan(path: &Path) -> Result<UntrustedPlan> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("failed to open cleanup plan {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_reader(file)
        .with_context(|| format!("failed to parse cleanup plan {}", path.display()))?;

    if value.get("version").and_then(serde_json::Value::as_u64)
        == Some(u64::from(LEGACY_CLEANUP_PLAN_VERSION))
    {
        anyhow::bail!(V1_RESCAN_MESSAGE);
    }

    serde_json::from_value(value)
        .with_context(|| format!("failed to parse cleanup plan {}", path.display()))
}

fn run_protect(command: ProtectCommand) -> Result<()> {
    match command {
        ProtectCommand::Add { path } => {
            let mut list = UserProtectionList::load()?;
            list.add(&path)?;
            println!("Protected {}", path.display());
        }
        ProtectCommand::Remove { path } => {
            let mut list = UserProtectionList::load()?;
            if list.remove(&path)? {
                println!("Removed protection for {}", path.display());
            } else {
                println!("No protection entry matched {}", path.display());
            }
        }
        ProtectCommand::List => {
            let list = UserProtectionList::load()?;
            if list.list().is_empty() {
                println!("No protected paths.");
            } else {
                for path in list.list() {
                    println!("{}", path.display());
                }
            }
        }
    }
    Ok(())
}

fn run_rules() -> Result<()> {
    let catalogue = devsweep::rules::rule_catalogue();
    println!("Project rules");
    for doc in catalogue
        .iter()
        .filter(|doc| doc.scope == RuleScope::Project)
    {
        println!("  {}", devsweep::rules::rule_row(doc));
    }
    println!();
    println!("Global providers & caches");
    for doc in catalogue
        .iter()
        .filter(|doc| doc.scope == RuleScope::Global)
    {
        println!("  {}", devsweep::rules::rule_row(doc));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn legacy_saved_plan_reports_the_rescan_guidance_before_v2_decoding() {
        let file = NamedTempFile::new().expect("temporary plan file");
        fs::write(
            file.path(),
            r#"{
                "version": 1,
                "targets": [{
                    "id": "node.node_modules:C:/code/app/node_modules",
                    "scope": { "type": "project", "root": "C:/code/app" },
                    "ecosystem": "node",
                    "kind": "dependency_directory",
                    "path": "C:/code/app/node_modules",
                    "estimated_bytes": 1,
                    "last_modified": null,
                    "risk": "medium",
                    "reversible": true,
                    "selected_by_default": false,
                    "evidence": [],
                    "action": {
                        "type": "move_to_trash",
                        "path": "C:/code/app/node_modules"
                    }
                }]
            }"#,
        )
        .expect("legacy plan writes");

        assert_eq!(
            read_untrusted_plan(file.path())
                .expect_err("legacy plan rejects")
                .to_string(),
            V1_RESCAN_MESSAGE
        );
    }
}
