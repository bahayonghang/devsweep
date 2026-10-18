//! Software update check and one-package plan, preview, and execute.

use std::{fs, path::Path};

use devsweep_core::software::{
    SoftwareUpdateError, SoftwareUpdateOutcome, SoftwareUpdatePlanV1, SoftwareUpdatePreviewV1,
    SoftwareUpdateReportV1, SoftwareUpdatesV1, check_software_updates, execute_software_update,
    plan_software_update, preview_software_update,
};

use super::super::super::{
    cli::{
        Cli, SoftwareUpdatesCheckCommand, SoftwareUpdatesExecuteCommand,
        SoftwareUpdatesPlanCommand, SoftwareUpdatesPreviewCommand,
    },
    output::{
        ApplicationError, ExitClass, OutputSink, ResultFormat, ensure_output_absent,
        from_domain_code, write_json,
    },
    presentation,
};
use crate::i18n::Locale;

#[cfg(test)]
pub(super) struct Route;

pub(super) fn run_check(
    command: &SoftwareUpdatesCheckCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    let inventory = match &command.inventory {
        Some(path) => Some(super::inventory::load_inventory(path)?),
        None => None,
    };
    let report = check_software_updates(inventory.as_ref(), None);
    emit(cli, locale, &report)
}

pub(super) fn run_plan(command: &SoftwareUpdatesPlanCommand) -> Result<(), ApplicationError> {
    let plan = plan_software_update(
        command.provider.as_os_str(),
        &command.package_id,
        &command.source,
        &command.version,
    )
    .map_err(map_update)?;
    write_plan(&command.output, &plan)
}

pub(super) fn run_preview(
    command: &SoftwareUpdatesPreviewCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    let plan = load_plan(&command.plan)?;
    let preview =
        preview_software_update(command.provider.as_os_str(), &plan).map_err(map_update)?;
    emit_preview(cli, locale, &preview)
}

pub(super) fn run_execute(
    command: &SoftwareUpdatesExecuteCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    if !command.confirm {
        return Err(ApplicationError::usage(
            "confirmation_required",
            "package update requires explicit confirmation",
        ));
    }
    let plan = load_plan(&command.plan)?;
    ensure_output_absent(cli.output_path())?;
    let report = execute_software_update(
        command.provider.as_os_str(),
        &plan,
        &command.preview_digest,
        command.confirm,
    )
    .map_err(map_update)?;
    emit_report(cli, locale, &report)?;
    match report.outcome {
        SoftwareUpdateOutcome::Updated | SoftwareUpdateOutcome::RestartRequired => Ok(()),
        SoftwareUpdateOutcome::Failed => Err(ApplicationError::after_output(
            ExitClass::Failed,
            "update_failed",
            "package update failed",
        )),
        SoftwareUpdateOutcome::Unknown => Err(ApplicationError::after_output(
            ExitClass::Failed,
            "update_result_unknown",
            "package update result is unknown",
        )),
    }
}

fn load_plan(path: &Path) -> Result<SoftwareUpdatePlanV1, ApplicationError> {
    let bytes = fs::read(path).map_err(|error| {
        ApplicationError::failed(
            "plan_read_failed",
            format!("failed to read update plan {}: {error}", path.display()),
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|_| ApplicationError::usage("invalid_plan", "update plan is invalid"))
}

fn map_update(error: SoftwareUpdateError) -> ApplicationError {
    from_domain_code(error.code(), error.to_string())
}

fn write_plan(path: &Path, plan: &SoftwareUpdatePlanV1) -> Result<(), ApplicationError> {
    let mut sink = OutputSink::open(Some(path))?;
    let mut bytes = serde_json::to_vec_pretty(plan)
        .map_err(|error| ApplicationError::failed("output_serialize_failed", error.to_string()))?;
    bytes.push(b'\n');
    sink.write_all(&bytes)
        .map_err(|error| ApplicationError::failed("output_write_failed", error.to_string()))
}

fn emit_preview(
    cli: &Cli,
    locale: Locale,
    preview: &SoftwareUpdatePreviewV1,
) -> Result<(), ApplicationError> {
    match cli.result_format() {
        ResultFormat::Human => {
            let text =
                presentation::software::update_preview(locale, preview).map_err(|error| {
                    ApplicationError::failed("catalogue_render_failed", error.to_string())
                })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "software.updates.preview",
            "succeeded",
            serde_json::to_value(preview).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
}

fn emit_report(
    cli: &Cli,
    locale: Locale,
    report: &SoftwareUpdateReportV1,
) -> Result<(), ApplicationError> {
    let outcome = match report.outcome {
        SoftwareUpdateOutcome::Updated => "succeeded",
        SoftwareUpdateOutcome::RestartRequired => "restart_required",
        SoftwareUpdateOutcome::Failed => "failed",
        SoftwareUpdateOutcome::Unknown => "unknown",
    };
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::software::update_report(locale, report).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "software.updates.execute",
            outcome,
            serde_json::to_value(report).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
}

fn emit(cli: &Cli, locale: Locale, report: &SoftwareUpdatesV1) -> Result<(), ApplicationError> {
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::software::updates(locale, report).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "software.updates.check",
            "succeeded",
            serde_json::to_value(report).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
}

fn write_human(cli: &Cli, text: &str) -> Result<(), ApplicationError> {
    let mut sink = OutputSink::open(cli.output_path())?;
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(b'\n');
    sink.write_all(&bytes)
        .map_err(|error| ApplicationError::failed("output_write_failed", error.to_string()))
}

fn write_envelope(
    cli: &Cli,
    command: &str,
    outcome: &str,
    data: serde_json::Value,
) -> Result<(), ApplicationError> {
    let mut sink = OutputSink::open(cli.output_path())?;
    write_json(
        &mut sink,
        &serde_json::json!({
            "schema_version": 1,
            "command": command,
            "outcome": outcome,
            "data": data,
            "warnings": [],
            "error": null
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use devsweep_core::software::SoftwareUpdatesReason;
    use std::fs;
    use tempfile::TempDir;

    fn parse(args: &[&str]) -> Cli {
        let cli = Cli::try_parse_from(args).expect("updates command parses");
        cli.validate().expect("updates command validates");
        cli
    }

    #[test]
    fn missing_inventory_fails_before_a_winget_probe() {
        let fixture = TempDir::new().unwrap();
        let missing = fixture.path().join("missing.json");
        let output = fixture.path().join("out.json");
        let cli = parse(&[
            "devsweep",
            "software",
            "updates",
            "check",
            "--inventory",
            missing.to_str().unwrap(),
            "--format",
            "json",
            "--output",
            output.to_str().unwrap(),
        ]);
        let super::super::super::super::cli::Command::Software(group) =
            cli.command.as_ref().unwrap()
        else {
            panic!("software");
        };
        let super::super::super::super::cli::SoftwareCommand::Updates(group) = &group.command
        else {
            panic!("updates");
        };
        let super::super::super::super::cli::SoftwareUpdatesCommand::Check(command) =
            &group.command
        else {
            panic!("check");
        };
        let error = run_check(command, Locale::En, &cli).expect_err("missing inventory");
        assert_eq!(error.code, "inventory_read_failed");
        assert_eq!(
            error.exit,
            super::super::super::super::output::ExitClass::Failed
        );
        assert!(!output.exists());
    }

    #[test]
    fn unavailable_winget_shape_stays_in_the_json_data() {
        let fixture = TempDir::new().unwrap();
        let output = fixture.path().join("out.json");
        let cli = parse(&[
            "devsweep",
            "software",
            "updates",
            "check",
            "--format",
            "json",
            "--output",
            output.to_str().unwrap(),
        ]);
        let report = SoftwareUpdatesV1::Unavailable {
            version: 1,
            observed_at_unix_ms: 7,
            reason_code: SoftwareUpdatesReason::WingetMissing,
        };
        emit(&cli, Locale::En, &report).unwrap();
        emit(&cli, Locale::ZhCn, &report).expect_err("second write collides");
        let document: serde_json::Value =
            serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
        assert_eq!(document["command"], "software.updates.check");
        assert_eq!(document["outcome"], "succeeded");
        assert_eq!(document["data"], serde_json::to_value(&report).unwrap());
    }
}
