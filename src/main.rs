use anyhow::{Context, Result};
use clap::Parser;
use devsweep::{
    cli::{CleanCommand, Cli, Command, ScanCommand},
    config,
    executor::{ExecutionRequest, Executor},
    model::CleanupPlan,
    sweep::{ScanOptions, Sweeper},
};

fn main() -> Result<()> {
    config::init_tracing();

    match Cli::parse().command {
        Command::Tui => devsweep::tui::run(),
        Command::Scan(command) => run_scan(command),
        Command::Clean(command) => run_clean(command),
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
        serde_json::to_writer_pretty(std::io::stdout(), &plan)?;
        println!();
        return Ok(());
    }

    println!(
        "Scanner found {} cleanup target(s) from {} root(s), scope {}.",
        plan.targets.len(),
        command.roots.len(),
        command.scope_label()
    );
    println!("Run with --json to emit the cleanup plan.");

    Ok(())
}

fn run_clean(command: CleanCommand) -> Result<()> {
    if command.execute && command.plan.is_none() {
        anyhow::bail!("cleanup execution requires --plan PATH");
    }

    let plan = match command.plan.as_deref() {
        Some(path) => {
            let file = std::fs::File::open(path)
                .with_context(|| format!("failed to open cleanup plan {}", path.display()))?;
            serde_json::from_reader(file)
                .with_context(|| format!("failed to parse cleanup plan {}", path.display()))?
        }
        None => CleanupPlan::empty(),
    };
    let report = Executor::default().run_plan(
        &plan,
        ExecutionRequest {
            execute: command.execute,
            allow_permanent_delete: command.allow_permanent_delete,
            audit_log: command.audit_log,
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

fn run_rules() -> Result<()> {
    println!(
        "{:<30} {:<8} {:<10} {:<18} SUMMARY",
        "ID", "SCOPE", "RISK", "ACTION"
    );
    for doc in devsweep::rules::rule_catalogue() {
        println!(
            "{:<30} {:<8} {:<10} {:<18} {}",
            doc.id,
            format!("{:?}", doc.scope).to_lowercase(),
            format!("{:?}", doc.risk).to_lowercase(),
            doc.action,
            doc.summary,
        );
    }
    Ok(())
}
