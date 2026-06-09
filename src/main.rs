use anyhow::Result;
use clap::Parser;
use devsweep::{
    cli::{CleanCommand, Cli, Command, ScanCommand},
    config,
    model::CleanupPlan,
    scanner::ProjectScanner,
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
    let include_projects = command.projects || !command.global;
    let plan = if include_projects {
        ProjectScanner::new().scan_roots(&command.roots)?
    } else {
        CleanupPlan::empty()
    };

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
    if command.execute {
        anyhow::bail!("cleanup execution is not implemented in the foundation build");
    }

    let plan = command
        .plan
        .as_deref()
        .map_or("no plan file provided".to_string(), |path| {
            format!("plan file: {}", path.display())
        });

    println!("Clean dry-run placeholder: {plan}. No cleanup actions were executed.");
    Ok(())
}

fn run_rules() -> Result<()> {
    println!("Rules placeholder: built-in cleanup rules are not implemented yet.");
    Ok(())
}
