//! Compiler-registered Software handler hierarchy.

use super::super::{
    cli::{Cli, Command, SoftwareCommand},
    output::ApplicationError,
};
use crate::i18n::{Locale, catalogue};

mod execution;
mod inventory;

#[cfg(test)]
pub(super) struct Route;

pub(super) fn run(cli: &Cli, locale: Locale) -> Result<(), ApplicationError> {
    match cli.command.as_ref() {
        Some(Command::Software(group)) => match &group.command {
            SoftwareCommand::Inventory(command) => inventory::run_inventory(command, locale, cli),
            SoftwareCommand::Plan(command) => inventory::run_plan(command),
            SoftwareCommand::Preview(_) | SoftwareCommand::Uninstall(_) => {
                Err(mode_unavailable(cli, locale))
            }
        },
        _ => Err(mode_unavailable(cli, locale)),
    }
}

fn mode_unavailable(cli: &Cli, locale: Locale) -> ApplicationError {
    let command = cli.command_name().unwrap_or("software");
    let message = catalogue(locale)
        .render("error.mode_unavailable", &[("command", command)], None)
        .unwrap_or_else(|_| format!("unavailable: {command}"));
    ApplicationError::unavailable(message)
}

#[cfg(test)]
pub(super) fn assert_leaf_routes_registered() {
    let _ = execution::Route;
    let _ = inventory::Route;
}
