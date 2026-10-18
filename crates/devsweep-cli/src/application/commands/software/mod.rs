//! Compiler-registered Software handler hierarchy.

use super::super::{
    cli::{
        Cli, Command, SoftwareCommand, SoftwareLeftoversCommand, SoftwareStartupCommand,
        SoftwareUpdatesCommand,
    },
    output::ApplicationError,
};
use crate::i18n::{Locale, catalogue};

mod execution;
mod inventory;
mod leftovers;
mod startup;
mod updates;

#[cfg(test)]
pub(super) struct Route;

pub(super) fn run(cli: &Cli, locale: Locale) -> Result<(), ApplicationError> {
    match cli.command.as_ref() {
        Some(Command::Software(group)) => match &group.command {
            SoftwareCommand::Inventory(command) => inventory::run_inventory(command, locale, cli),
            SoftwareCommand::Plan(command) => inventory::run_plan(command),
            SoftwareCommand::Preview(command) => execution::run_preview(command, locale, cli),
            SoftwareCommand::Uninstall(command) => execution::run_uninstall(command, locale, cli),
            SoftwareCommand::Updates(group) => match &group.command {
                SoftwareUpdatesCommand::Check(command) => updates::run_check(command, locale, cli),
                SoftwareUpdatesCommand::Plan(command) => updates::run_plan(command),
                SoftwareUpdatesCommand::Preview(command) => {
                    updates::run_preview(command, locale, cli)
                }
                SoftwareUpdatesCommand::Execute(command) => {
                    updates::run_execute(command, locale, cli)
                }
            },
            SoftwareCommand::Startup(group) => match &group.command {
                SoftwareStartupCommand::List(_) => startup::run_list(locale, cli),
                SoftwareStartupCommand::Plan(command) => startup::run_plan(command),
                SoftwareStartupCommand::Preview(command) => {
                    startup::run_preview(command, locale, cli)
                }
                SoftwareStartupCommand::Execute(command) => {
                    startup::run_execute(command, locale, cli)
                }
                SoftwareStartupCommand::HivePlan(command) => startup::run_hive_plan(command),
                SoftwareStartupCommand::HivePreview(command) => {
                    startup::run_hive_preview(command, locale, cli)
                }
                SoftwareStartupCommand::HiveExecute(command) => {
                    startup::run_hive_execute(command, locale, cli)
                }
                SoftwareStartupCommand::RestorePlan(command) => startup::run_restore_plan(command),
                SoftwareStartupCommand::RestorePreview(command) => {
                    startup::run_restore_preview(command, locale, cli)
                }
                SoftwareStartupCommand::RestoreExecute(command) => {
                    startup::run_restore_execute(command, locale, cli)
                }
            },
            SoftwareCommand::Leftovers(group) => match &group.command {
                SoftwareLeftoversCommand::Scan(command) => {
                    leftovers::run_scan(command, locale, cli)
                }
                SoftwareLeftoversCommand::Plan(command) => leftovers::run_plan(command),
                SoftwareLeftoversCommand::Preview(command) => {
                    leftovers::run_preview(command, locale, cli)
                }
                SoftwareLeftoversCommand::Execute(command) => {
                    leftovers::run_execute(command, locale, cli)
                }
                SoftwareLeftoversCommand::RegistryPlan(command) => {
                    leftovers::run_registry_plan(command)
                }
                SoftwareLeftoversCommand::RegistryPreview(command) => {
                    leftovers::run_registry_preview(command, locale, cli)
                }
                SoftwareLeftoversCommand::RegistryExecute(command) => {
                    leftovers::run_registry_execute(command, locale, cli)
                }
                SoftwareLeftoversCommand::RegistryRestorePlan(command) => {
                    leftovers::run_registry_restore_plan(command)
                }
                SoftwareLeftoversCommand::RegistryRestorePreview(command) => {
                    leftovers::run_registry_restore_preview(command, locale, cli)
                }
                SoftwareLeftoversCommand::RegistryRestoreExecute(command) => {
                    leftovers::run_registry_restore_execute(command, locale, cli)
                }
            },
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
    let _ = leftovers::Route;
    let _ = startup::Route;
    let _ = updates::Route;
}
