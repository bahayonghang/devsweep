use std::path::Path;

use anyhow::{Context, Result};
use clap::Parser;
use devsweep::{
    cli::{CleanCommand, Cli, Command, InventoryCommand, ProtectCommand, ScanCommand},
    config,
    executor::{ExecutionReport, ExecutionRequest, Executor},
    inventory::inventory_root,
    model::{
        LEGACY_CLEANUP_PLAN_VERSION, SCAN_REPORT_VERSION, ScanReport, TargetId, UntrustedPlan,
    },
    plan::{V1_RESCAN_MESSAGE, validate_plan},
    rules::RuleScope,
    safety::UserProtectionList,
    sweep::{ScanOptions, Sweeper},
};

fn main() -> Result<()> {
    config::init_tracing();

    match Cli::parse().command {
        Command::Tui => devsweep::tui::run(),
        Command::Scan(command) => run_scan(command),
        Command::Inventory(command) => run_inventory(command),
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
    let report = match command.rescan_target.as_deref() {
        Some(target_id) => Sweeper::default().full_scan_report_rescanning_target(
            &options,
            &TargetId::new(target_id),
            &mut |_| {},
        )?,
        None => Sweeper::default().full_scan_report(&options, &mut |_| {})?,
    };

    if command.json {
        serde_json::to_writer_pretty(std::io::stdout(), &report)?;
        println!();
        return Ok(());
    }

    if options.include_projects {
        println!(
            "Scanner found {} cleanup target(s) from {} root(s), scope {}.",
            report.plan.targets.len(),
            command.roots.len(),
            command.scope_label()
        );
    } else {
        println!(
            "Scanner found {} global cleanup target(s), scope {}.",
            report.plan.targets.len(),
            command.scope_label()
        );
    }
    println!(
        "Scan health: {} ({} diagnostic(s)).",
        report.health.completeness.label(),
        report.health.diagnostics.len()
    );
    println!(
        "Capacity: {} verified; {} partial lower bound; {} unknown target(s).",
        format_bytes(report.health.totals.verified_bytes),
        format_bytes(report.health.totals.partial_lower_bound_bytes),
        report.health.totals.unknown_target_count
    );
    if let Some(target_id) = &command.rescan_target {
        println!("Re-estimated target {target_id} with the review size budget.");
    }
    println!("Run with --json to emit the scan report.");

    Ok(())
}

fn run_inventory(command: InventoryCommand) -> Result<()> {
    let report = inventory_root(&command.root)?;
    if command.json {
        serde_json::to_writer_pretty(std::io::stdout(), &report)?;
        println!();
        return Ok(());
    }

    println!("Inventory root: {}", report.root.display());
    for observation in &report.observations {
        let size = if observation.size_complete {
            format_bytes(observation.estimated_bytes)
        } else if observation.estimated_bytes == 0 {
            "unknown".to_string()
        } else {
            format!(">= {}", format_bytes(observation.estimated_bytes))
        };
        println!("  {size:>12}  {}", observation.path.display());
    }
    println!(
        "Inventory health: {} ({} diagnostic(s)); {} verified, {} partial lower bound, {} unknown.",
        report.health.completeness.label(),
        report.health.diagnostics.len(),
        format_bytes(report.health.totals.verified_bytes),
        format_bytes(report.health.totals.partial_lower_bound_bytes),
        report.health.totals.unknown_target_count,
    );
    if let Some(finding) = &report.orphan_pnpm_store {
        println!(
            "Inspect-only pnpm store: {} (configured store: {}; {} project reference file(s)).",
            finding.candidate_path.display(),
            finding.configured_store.display(),
            finding.project_references.len(),
        );
    }
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
        anyhow::bail!(execution_failure_message(&report));
    }
    Ok(())
}

fn execution_failure_message(report: &ExecutionReport) -> String {
    let failures = report
        .failures
        .iter()
        .map(|failure| format!("{}: {}", failure.target_id.as_str(), failure.message))
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        "cleanup finished with {} failure(s): {failures}",
        report.failed
    )
}

fn read_untrusted_plan(path: &Path) -> Result<UntrustedPlan> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("failed to open cleanup plan {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_reader(file)
        .with_context(|| format!("failed to parse cleanup plan {}", path.display()))?;

    if value.get("observations").is_some() {
        anyhow::bail!("inventory reports cannot be used as cleanup plans");
    }

    if value.get("plan").is_some() {
        let report: ScanReport = serde_json::from_value(value)
            .with_context(|| format!("failed to parse scan report {}", path.display()))?;
        if !report.has_supported_version() {
            anyhow::bail!(
                "unsupported scan report version {}; expected {}",
                report.version,
                SCAN_REPORT_VERSION
            );
        }
        return Ok(report.plan);
    }

    if value.get("version").and_then(serde_json::Value::as_u64)
        == Some(u64::from(LEGACY_CLEANUP_PLAN_VERSION))
    {
        anyhow::bail!(V1_RESCAN_MESSAGE);
    }

    serde_json::from_value(value)
        .with_context(|| format!("failed to parse cleanup plan {}", path.display()))
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }

    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
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

    use tempfile::{NamedTempFile, TempDir};

    use super::*;

    #[test]
    fn execution_failure_message_names_each_failed_target_and_reason() {
        let report = ExecutionReport {
            selected: 2,
            attempted: 2,
            succeeded: 0,
            failed: 2,
            skipped: 0,
            dry_run: false,
            audit_log: None,
            failures: vec![
                devsweep::executor::ActionFailure {
                    target_id: TargetId::new("rust.target:C:/repo/target"),
                    message: "marker CACHEDIR.TAG is missing".to_string(),
                },
                devsweep::executor::ActionFailure {
                    target_id: TargetId::new("node.node_modules:C:/repo/node_modules"),
                    message: "target path is a reparse point".to_string(),
                },
            ],
        };

        let message = execution_failure_message(&report);

        assert!(message.contains("2 failure(s)"));
        assert!(message.contains("rust.target:C:/repo/target: marker CACHEDIR.TAG is missing"));
        assert!(
            message
                .contains("node.node_modules:C:/repo/node_modules: target path is a reparse point")
        );
    }

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

    #[test]
    fn scan_report_saved_plan_decodes_to_its_embedded_untrusted_plan() {
        let file = NamedTempFile::new().expect("temporary report file");
        let report = ScanReport::new(
            UntrustedPlan::empty(),
            devsweep::model::ScanHealth::complete(),
        );
        fs::write(
            file.path(),
            serde_json::to_vec_pretty(&report).expect("report serializes"),
        )
        .expect("report writes");

        assert_eq!(
            read_untrusted_plan(file.path()).expect("report plan decodes"),
            report.plan
        );
    }

    #[test]
    fn scan_report_rejects_untrusted_executable_fields() {
        let file = NamedTempFile::new().expect("temporary report file");
        let report = ScanReport::new(
            UntrustedPlan::empty(),
            devsweep::model::ScanHealth::complete(),
        );
        let document = serde_json::to_value(report).expect("report serializes");

        for field in ["program", "action"] {
            let mut top_level = document.clone();
            top_level[field] = serde_json::Value::String("powershell.exe".to_string());
            fs::write(
                file.path(),
                serde_json::to_vec(&top_level).expect("invalid report serializes"),
            )
            .expect("invalid report writes");
            assert!(
                read_untrusted_plan(file.path()).is_err(),
                "top-level {field} must reject"
            );

            let mut embedded_plan = document.clone();
            embedded_plan["plan"][field] = serde_json::Value::String("powershell.exe".to_string());
            fs::write(
                file.path(),
                serde_json::to_vec(&embedded_plan).expect("invalid report serializes"),
            )
            .expect("invalid report writes");
            assert!(
                read_untrusted_plan(file.path()).is_err(),
                "embedded plan {field} must reject"
            );
        }
    }

    #[test]
    fn inventory_document_cannot_be_used_as_a_cleanup_plan() {
        let root = TempDir::new().expect("temporary inventory root");
        fs::write(root.path().join("observed.bin"), b"inventory only")
            .expect("inventory fixture writes");
        let report = inventory_root(root.path()).expect("inventory succeeds");
        let file = NamedTempFile::new().expect("temporary inventory report");
        fs::write(
            file.path(),
            serde_json::to_vec(&report).expect("inventory serializes"),
        )
        .expect("inventory report writes");

        assert_eq!(
            read_untrusted_plan(file.path())
                .expect_err("inventory report rejects")
                .to_string(),
            "inventory reports cannot be used as cleanup plans"
        );
    }
}
