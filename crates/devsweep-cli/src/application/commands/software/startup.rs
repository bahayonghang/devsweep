//! Startup list, plan, preview, and strict execute.

use std::{fs, path::Path};

use devsweep_core::software::{
    SoftwareSourceState, SoftwareStartupError, SoftwareStartupListV1, StartupHiveExecuteReportV1,
    StartupRestorePlanV1, StartupRestorePreviewV1, StartupStrictOutcome, StartupTogglePlanV1,
    StartupTogglePreviewV1, StartupToggleStrictReportV1, execute_startup_hive,
    execute_startup_restore, execute_startup_toggle, list_startup_entries, plan_startup_hive,
    plan_startup_restore, plan_startup_toggle, preview_startup_hive, preview_startup_restore,
    preview_startup_toggle,
};

use super::super::super::{
    cli::{
        Cli, PlanExecutionCommand, PlanPresentationCommand, SoftwareStartupHiveExecuteCommand,
        SoftwareStartupHivePlanCommand, SoftwareStartupHivePreviewCommand,
        SoftwareStartupPlanCommand, SoftwareStartupRestoreExecuteCommand,
        SoftwareStartupRestorePlanCommand, SoftwareStartupRestorePreviewCommand,
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

pub(super) fn run_list(locale: Locale, cli: &Cli) -> Result<(), ApplicationError> {
    let list = list_startup_entries();
    emit_list(cli, locale, &list)
}

pub(super) fn run_plan(command: &SoftwareStartupPlanCommand) -> Result<(), ApplicationError> {
    let plan = plan_startup_toggle(&command.entry, command.enabled).map_err(map_startup)?;
    write_plan(&command.output, &plan)
}

pub(super) fn run_preview(
    command: &PlanPresentationCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    let plan = load_plan(&command.plan)?;
    let preview = preview_startup_toggle(&plan).map_err(map_startup)?;
    emit_preview(cli, locale, "software.startup.preview", &preview)
}

pub(super) fn run_execute(
    command: &PlanExecutionCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    if !command.confirm {
        return Err(ApplicationError::usage(
            "confirmation_required",
            "startup toggle requires explicit confirmation",
        ));
    }
    let plan = load_plan(&command.plan)?;
    ensure_output_absent(cli.output_path())?;
    let report = execute_startup_toggle(&plan, &command.preview_digest, command.confirm)
        .map_err(map_startup)?;
    emit_execute(cli, locale, &report)?;
    match report.outcome {
        StartupStrictOutcome::Succeeded | StartupStrictOutcome::AlreadyInState => Ok(()),
        StartupStrictOutcome::Failed => Err(ApplicationError::after_output(
            ExitClass::Failed,
            "startup_toggle_failed",
            "startup toggle failed after dispatch",
        )),
        StartupStrictOutcome::Unknown => Err(ApplicationError::after_output(
            ExitClass::Failed,
            "startup_result_unknown",
            "startup toggle result is unknown",
        )),
    }
}

pub(super) fn run_hive_plan(
    command: &SoftwareStartupHivePlanCommand,
) -> Result<(), ApplicationError> {
    let plan =
        plan_startup_hive(&command.hive, &command.entry, command.enabled).map_err(map_startup)?;
    write_plan(&command.output, &plan)
}

pub(super) fn run_hive_preview(
    command: &SoftwareStartupHivePreviewCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    let plan = load_plan(&command.plan)?;
    let preview = preview_startup_hive(&command.hive, &plan).map_err(map_startup)?;
    emit_preview(cli, locale, "software.startup.hive-preview", &preview)
}

pub(super) fn run_hive_execute(
    command: &SoftwareStartupHiveExecuteCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    if !command.confirm {
        return Err(ApplicationError::usage(
            "confirmation_required",
            "startup toggle requires explicit confirmation",
        ));
    }
    let plan = load_plan(&command.plan)?;
    ensure_output_absent(cli.output_path())?;
    let report = execute_startup_hive(
        &command.hive,
        &plan,
        &command.preview_digest,
        command.confirm,
    )
    .map_err(map_startup)?;
    emit_hive(cli, locale, &report)?;
    finish_toggle(report.toggle.outcome)
}

pub(super) fn run_restore_plan(
    command: &SoftwareStartupRestorePlanCommand,
) -> Result<(), ApplicationError> {
    let report = load_hive_report(&command.report)?;
    let Some(backup) = report.backup else {
        return Err(ApplicationError::usage(
            "invalid_plan",
            "hive report has no backup",
        ));
    };
    let plan = plan_startup_restore(&backup).map_err(map_startup)?;
    write_restore_plan(&command.output, &plan)
}

pub(super) fn run_restore_preview(
    command: &SoftwareStartupRestorePreviewCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    let plan = load_restore_plan(&command.plan)?;
    let preview = preview_startup_restore(&command.hive, &plan).map_err(map_startup)?;
    emit_restore_preview(cli, locale, &preview)
}

pub(super) fn run_restore_execute(
    command: &SoftwareStartupRestoreExecuteCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    if !command.confirm {
        return Err(ApplicationError::usage(
            "confirmation_required",
            "startup restore requires explicit confirmation",
        ));
    }
    let plan = load_restore_plan(&command.plan)?;
    ensure_output_absent(cli.output_path())?;
    execute_startup_restore(
        &command.hive,
        &plan,
        &command.preview_digest,
        command.confirm,
    )
    .map_err(map_startup)?;
    emit_restore_succeeded(cli, locale)
}

fn finish_toggle(outcome: StartupStrictOutcome) -> Result<(), ApplicationError> {
    match outcome {
        StartupStrictOutcome::Succeeded | StartupStrictOutcome::AlreadyInState => Ok(()),
        StartupStrictOutcome::Failed => Err(ApplicationError::after_output(
            ExitClass::Failed,
            "startup_toggle_failed",
            "startup toggle failed after dispatch",
        )),
        StartupStrictOutcome::Unknown => Err(ApplicationError::after_output(
            ExitClass::Failed,
            "startup_result_unknown",
            "startup toggle result is unknown",
        )),
    }
}

fn load_plan(path: &Path) -> Result<StartupTogglePlanV1, ApplicationError> {
    let bytes = fs::read(path).map_err(|error| {
        ApplicationError::failed(
            "plan_read_failed",
            format!("failed to read startup plan {}: {error}", path.display()),
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|_| ApplicationError::usage("invalid_plan", "startup plan is invalid"))
}

fn map_startup(error: SoftwareStartupError) -> ApplicationError {
    from_domain_code(error.code(), error.to_string())
}

pub(super) fn list_terminal(
    list: &SoftwareStartupListV1,
) -> (
    &'static str,
    Option<(ExitClass, &'static str, &'static str)>,
) {
    if !list.sources.is_empty()
        && list
            .sources
            .iter()
            .all(|source| source.state == SoftwareSourceState::Unsupported)
    {
        return (
            "unavailable",
            Some((
                ExitClass::Unavailable,
                "startup_source_unavailable",
                "every startup source is unsupported",
            )),
        );
    }
    if list
        .sources
        .iter()
        .any(|source| source.state != SoftwareSourceState::Available)
    {
        return (
            "partial",
            Some((
                ExitClass::Partial,
                "startup_partial",
                "startup list is partial",
            )),
        );
    }
    ("succeeded", None)
}

fn emit_list(
    cli: &Cli,
    locale: Locale,
    list: &SoftwareStartupListV1,
) -> Result<(), ApplicationError> {
    let (outcome, terminal) = list_terminal(list);
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::software::startup_list(locale, list).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)?;
        }
        _ => write_envelope(
            cli,
            "software.startup.list",
            outcome,
            serde_json::to_value(list).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        )?,
    }
    finish(terminal)
}

fn emit_preview(
    cli: &Cli,
    locale: Locale,
    command: &str,
    preview: &StartupTogglePreviewV1,
) -> Result<(), ApplicationError> {
    match cli.result_format() {
        ResultFormat::Human => {
            let text =
                presentation::software::startup_preview(locale, preview).map_err(|error| {
                    ApplicationError::failed("catalogue_render_failed", error.to_string())
                })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            command,
            "succeeded",
            serde_json::to_value(preview).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
}

fn emit_hive(
    cli: &Cli,
    locale: Locale,
    report: &StartupHiveExecuteReportV1,
) -> Result<(), ApplicationError> {
    let outcome = match report.toggle.outcome {
        StartupStrictOutcome::Succeeded | StartupStrictOutcome::AlreadyInState => "succeeded",
        StartupStrictOutcome::Failed => "failed",
        StartupStrictOutcome::Unknown => "unknown",
    };
    match cli.result_format() {
        ResultFormat::Human => {
            let text =
                presentation::software::startup_hive_execute(locale, report).map_err(|error| {
                    ApplicationError::failed("catalogue_render_failed", error.to_string())
                })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "software.startup.hive-execute",
            outcome,
            serde_json::to_value(report).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
}

fn emit_restore_preview(
    cli: &Cli,
    locale: Locale,
    preview: &StartupRestorePreviewV1,
) -> Result<(), ApplicationError> {
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::software::startup_restore_preview(locale, preview).map_err(
                |error| ApplicationError::failed("catalogue_render_failed", error.to_string()),
            )?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "software.startup.restore-preview",
            "succeeded",
            serde_json::to_value(preview).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
}

fn emit_restore_succeeded(cli: &Cli, locale: Locale) -> Result<(), ApplicationError> {
    match cli.result_format() {
        ResultFormat::Human => {
            let text =
                presentation::software::startup_restore_succeeded(locale).map_err(|error| {
                    ApplicationError::failed("catalogue_render_failed", error.to_string())
                })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "software.startup.restore-execute",
            "succeeded",
            serde_json::Value::Null,
        ),
    }
}

fn load_hive_report(path: &Path) -> Result<StartupHiveExecuteReportV1, ApplicationError> {
    let invalid = "hive report is invalid";
    let bytes = fs::read(path).map_err(|error| {
        ApplicationError::failed(
            "plan_read_failed",
            format!("failed to read hive report {}: {error}", path.display()),
        )
    })?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| ApplicationError::usage("invalid_plan", invalid))?;
    let payload = value
        .get("schema_version")
        .and_then(|_| value.get("data"))
        .cloned()
        .unwrap_or(value);
    serde_json::from_value(payload).map_err(|_| ApplicationError::usage("invalid_plan", invalid))
}

fn load_restore_plan(path: &Path) -> Result<StartupRestorePlanV1, ApplicationError> {
    let bytes = fs::read(path).map_err(|error| {
        ApplicationError::failed(
            "plan_read_failed",
            format!("failed to read restore plan {}: {error}", path.display()),
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|_| ApplicationError::usage("invalid_plan", "restore plan is invalid"))
}

fn write_restore_plan(path: &Path, plan: &StartupRestorePlanV1) -> Result<(), ApplicationError> {
    let mut sink = OutputSink::open(Some(path))?;
    let mut bytes = serde_json::to_vec_pretty(plan)
        .map_err(|error| ApplicationError::failed("output_serialize_failed", error.to_string()))?;
    bytes.push(b'\n');
    sink.write_all(&bytes)
        .map_err(|error| ApplicationError::failed("output_write_failed", error.to_string()))
}

fn emit_execute(
    cli: &Cli,
    locale: Locale,
    report: &StartupToggleStrictReportV1,
) -> Result<(), ApplicationError> {
    let outcome = match report.outcome {
        StartupStrictOutcome::Succeeded => "succeeded",
        StartupStrictOutcome::Failed => "failed",
        StartupStrictOutcome::Unknown => "unknown",
        StartupStrictOutcome::AlreadyInState => "already_in_state",
    };
    match cli.result_format() {
        ResultFormat::Human => {
            let text =
                presentation::software::startup_execute(locale, report).map_err(|error| {
                    ApplicationError::failed("catalogue_render_failed", error.to_string())
                })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "software.startup.execute",
            outcome,
            serde_json::to_value(report).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
}

fn finish(
    terminal: Option<(ExitClass, &'static str, &'static str)>,
) -> Result<(), ApplicationError> {
    if let Some((exit, code, message)) = terminal {
        Err(ApplicationError::after_output(exit, code, message))
    } else {
        Ok(())
    }
}

fn write_plan(path: &Path, plan: &StartupTogglePlanV1) -> Result<(), ApplicationError> {
    let mut sink = OutputSink::open(Some(path))?;
    let mut bytes = serde_json::to_vec_pretty(plan)
        .map_err(|error| ApplicationError::failed("output_serialize_failed", error.to_string()))?;
    bytes.push(b'\n');
    sink.write_all(&bytes)
        .map_err(|error| ApplicationError::failed("output_write_failed", error.to_string()))
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
    use devsweep_core::software::{SoftwareStartupLocation, SoftwareStartupSourceV1};
    use tempfile::TempDir;

    fn parse(args: &[&str]) -> Cli {
        let cli = Cli::try_parse_from(args).expect("startup command parses");
        cli.validate().expect("startup command validates");
        cli
    }

    fn execute_command(cli: &Cli) -> &PlanExecutionCommand {
        let super::super::super::super::cli::Command::Software(group) =
            cli.command.as_ref().unwrap()
        else {
            panic!("software");
        };
        let super::super::super::super::cli::SoftwareCommand::Startup(group) = &group.command
        else {
            panic!("startup");
        };
        let super::super::super::super::cli::SoftwareStartupCommand::Execute(command) =
            &group.command
        else {
            panic!("execute");
        };
        command
    }

    #[test]
    fn malformed_plan_is_usage_and_leaves_an_existing_output_untouched() {
        let fixture = TempDir::new().unwrap();
        let plan = fixture.path().join("plan.json");
        let output = fixture.path().join("out.json");
        fs::write(&plan, b"{").unwrap();
        fs::write(&output, b"keep").unwrap();
        let digest = format!("sha256:{}", "a".repeat(64));
        let cli = parse(&[
            "devsweep",
            "software",
            "startup",
            "execute",
            "--plan",
            plan.to_str().unwrap(),
            "--preview-digest",
            &digest,
            "--confirm",
            "--format",
            "json",
            "--output",
            output.to_str().unwrap(),
        ]);
        let error = run_execute(execute_command(&cli), Locale::En, &cli).expect_err("malformed");
        assert_eq!(error.exit, ExitClass::Usage);
        assert_eq!(error.code, "invalid_plan");
        assert_eq!(fs::read(&output).unwrap(), b"keep");
    }

    #[test]
    fn existing_output_is_rejected_before_startup_dispatch() {
        let fixture = TempDir::new().unwrap();
        let plan = fixture.path().join("plan.json");
        let output = fixture.path().join("out.json");
        fs::write(
            &plan,
            br#"{"version":1,"entry_id":"startup:v1:not-real","requested_enabled":false,"expected_entry_identity_sha256":"ab","expected_approval_identity_sha256":"cd"}"#,
        )
        .unwrap();
        fs::write(&output, b"keep").unwrap();
        let digest = format!("sha256:{}", "b".repeat(64));
        let cli = parse(&[
            "devsweep",
            "software",
            "startup",
            "execute",
            "--plan",
            plan.to_str().unwrap(),
            "--preview-digest",
            &digest,
            "--confirm",
            "--output",
            output.to_str().unwrap(),
        ]);
        let error = run_execute(execute_command(&cli), Locale::En, &cli).expect_err("collision");
        assert_eq!(error.code, "output_exists");
        assert_eq!(error.exit, ExitClass::Failed);
        assert_eq!(fs::read(&output).unwrap(), b"keep");
    }

    #[test]
    fn partial_startup_list_exits_after_the_report() {
        let fixture = TempDir::new().unwrap();
        let output = fixture.path().join("list.json");
        let cli = parse(&[
            "devsweep",
            "software",
            "startup",
            "list",
            "--format",
            "json",
            "--output",
            output.to_str().unwrap(),
        ]);
        let list = SoftwareStartupListV1 {
            version: 1,
            observed_at_unix_ms: 1,
            sources: vec![SoftwareStartupSourceV1 {
                location: SoftwareStartupLocation::CurrentUserRun,
                state: SoftwareSourceState::Partial,
                reason_code: Some("fixture".to_string()),
            }],
            entries: Vec::new(),
        };
        let error = emit_list(&cli, Locale::En, &list).expect_err("partial");
        assert_eq!(error.exit, ExitClass::Partial);
        assert!(error.output_already_emitted);
        let document: serde_json::Value =
            serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
        assert_eq!(document["outcome"], "partial");
        assert_eq!(document["data"], serde_json::to_value(&list).unwrap());
    }

    #[test]
    fn unsupported_startup_sources_are_unavailable_after_the_report() {
        let list = SoftwareStartupListV1 {
            version: 1,
            observed_at_unix_ms: 1,
            sources: vec![SoftwareStartupSourceV1 {
                location: SoftwareStartupLocation::MachineRun,
                state: SoftwareSourceState::Unsupported,
                reason_code: None,
            }],
            entries: Vec::new(),
        };
        let (outcome, terminal) = list_terminal(&list);
        assert_eq!(outcome, "unavailable");
        assert_eq!(terminal.unwrap().0, ExitClass::Unavailable);
    }
}
