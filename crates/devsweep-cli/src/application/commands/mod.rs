//! Frozen command-dispatch root.
//!
//! Downstream mode tasks fill these compiler-registered modules. Until a
//! handler is wired, dispatch fails explicitly and cannot fall through to one
//! of the removed legacy commands.

use super::{cli::Cli, output::ApplicationError};
use crate::i18n::{Locale, catalogue};

mod analyze;
mod clean;
mod history;
mod optimize;
mod protect;
mod rules;
mod software;
mod status;

pub(super) fn dispatch(cli: &Cli, locale: Locale) -> Result<(), ApplicationError> {
    let command = cli
        .command_name()
        .expect("bare invocation is routed before command dispatch");
    if command.starts_with("clean.") {
        return clean::run(cli, locale);
    }
    if command.starts_with("analyze.") {
        return analyze::run(cli, locale);
    }
    if command.starts_with("software.") {
        return software::run(cli, locale);
    }
    if command.starts_with("optimize.") {
        return optimize::run(cli, locale);
    }
    let message = catalogue(locale)
        .render("error.mode_unavailable", &[("command", command)], None)
        .map_err(|error| {
            ApplicationError::failed(
                "catalogue_render_failed",
                format!("failed to render unavailable-mode message: {error}"),
            )
        })?;
    Err(ApplicationError::unavailable(message))
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;
    use crate::application::cli::Cli;

    #[test]
    fn frozen_module_tree_is_compiler_registered() {
        let _ = analyze::Route;
        let _ = clean::Route;
        let _ = history::Route;
        let _ = optimize::Route;
        let _ = protect::Route;
        let _ = rules::Route;
        let _ = software::Route;
        let _ = status::Route;
        software::assert_leaf_routes_registered();
        let _ = software::run;
    }

    #[test]
    fn staged_commands_fail_closed_instead_of_running_legacy_handlers() {
        let cli =
            Cli::try_parse_from(["devsweep", "status", "snapshot"]).expect("frozen command parses");
        let error = dispatch(&cli, Locale::En).expect_err("handler is not yet wired");
        assert_eq!(error.code, "mode_unavailable");
        assert!(error.message.contains("status.snapshot"));
    }

    #[test]
    fn optimize_commands_reach_the_owned_handler() {
        let temp = tempfile::TempDir::new().unwrap();
        let output = temp.path().join("plan.json");
        let cli = Cli::try_parse_from([
            "devsweep",
            "optimize",
            "plan",
            "--operation",
            "cmd.exe /c calc",
            "--output",
            output.to_str().unwrap(),
        ])
        .expect("frozen command parses");
        let error = dispatch(&cli, Locale::En).expect_err("hostile selection fails closed");
        assert_eq!(error.code, "invalid_optimize_selection");
    }

    #[test]
    fn software_commands_reach_the_owned_handler() {
        let cli = Cli::try_parse_from([
            "devsweep",
            "software",
            "plan",
            "--inventory",
            "missing-inventory.json",
            "--select",
            "software:v1:msix:missing",
            "--output",
            "unused-plan.json",
        ])
        .expect("frozen command parses");
        let error = dispatch(&cli, Locale::En).expect_err("missing inventory fails");
        assert_eq!(error.code, "inventory_read_failed");
    }
}
