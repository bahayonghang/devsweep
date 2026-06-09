use anyhow::Result;
use clap::Parser;
use devsweep::{
    cli::{CleanCommand, Cli, Command, ScanCommand},
    config,
    model::CleanupPlan,
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
    let plan = CleanupPlan::empty();

    if command.json {
        serde_json::to_writer_pretty(std::io::stdout(), &plan)?;
        println!();
        return Ok(());
    }

    println!(
        "Scanner placeholder: {} root(s), scope {}, {} cleanup target(s).",
        command.roots.len(),
        command.scope_label(),
        plan.targets.len()
    );
    println!("Run with --json to emit the current cleanup plan schema.");

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
