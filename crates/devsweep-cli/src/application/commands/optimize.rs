//! Frozen-grammar Optimize list, plan, preview, and run handlers.

use std::{fs, path::Path};

use devsweep_core::optimize::{
    MaintenanceExecutionError, MaintenanceExecutionOutcome, MaintenanceExecutor, MaintenancePlanV1,
    OPTIMIZE_CATALOGUE_VERSION, OptimizePlanError, catalogue_entries, plan_operation,
    preview_maintenance_plan_live,
};

use super::super::{
    cli::{Cli, Command, OptimizeCommand, PlanExecutionCommand, PlanPresentationCommand},
    output::{ApplicationError, ExitClass, OutputSink, ResultFormat, write_json},
    presentation,
};
use crate::i18n::Locale;

#[cfg(test)]
pub(super) struct Route;

pub(super) fn run(cli: &Cli, locale: Locale) -> Result<(), ApplicationError> {
    match cli.command.as_ref() {
        Some(Command::Optimize(group)) => match &group.command {
            OptimizeCommand::List(_) => emit_list(cli, locale),
            OptimizeCommand::Plan(command) => run_plan(command),
            OptimizeCommand::Preview(command) => run_preview(command, locale, cli),
            OptimizeCommand::Run(command) => run_run(command, locale, cli),
        },
        _ => Err(mode_unavailable(cli, locale)),
    }
}

fn mode_unavailable(cli: &Cli, locale: Locale) -> ApplicationError {
    let command = cli.command_name().unwrap_or("optimize");
    let message = crate::i18n::catalogue(locale)
        .render("error.mode_unavailable", &[("command", command)], None)
        .unwrap_or_else(|_| format!("unavailable: {command}"));
    ApplicationError::unavailable(message)
}

fn emit_list(cli: &Cli, locale: Locale) -> Result<(), ApplicationError> {
    let entries = catalogue_entries();
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::optimize::list(locale, entries).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => {
            let mut sink = OutputSink::open(cli.output_path())?;
            write_json(
                &mut sink,
                &serde_json::json!({
                    "schema_version": 1,
                    "command": "optimize.list",
                    "outcome": "succeeded",
                    "data": {
                        "catalogue_version": OPTIMIZE_CATALOGUE_VERSION,
                        "entries": entries,
                    },
                    "warnings": [],
                    "error": null
                }),
            )
        }
    }
}

fn run_plan(command: &super::super::cli::OptimizePlanCommand) -> Result<(), ApplicationError> {
    let plan = plan_operation(&command.operation).map_err(map_plan_error)?;
    let mut sink = OutputSink::open(Some(&command.output))?;
    let mut bytes = serde_json::to_vec_pretty(&plan)
        .map_err(|error| ApplicationError::failed("output_serialize_failed", error.to_string()))?;
    bytes.push(b'\n');
    sink.write_all(&bytes)
        .map_err(|error| ApplicationError::failed("output_write_failed", error.to_string()))
}

fn run_preview(
    command: &PlanPresentationCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    let plan = load_plan(&command.plan)?;
    let preview = preview_maintenance_plan_live(&plan).map_err(map_plan_error)?;
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::optimize::preview(locale, &preview).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)?;
        }
        _ => {
            write_envelope(
                cli,
                "optimize.preview",
                "succeeded",
                serde_json::to_value(&preview).map_err(|error| {
                    ApplicationError::failed("output_serialize_failed", error.to_string())
                })?,
            )?;
        }
    }
    Ok(())
}

fn run_run(
    command: &PlanExecutionCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    if !command.confirm {
        return Err(ApplicationError::usage(
            "confirmation_required",
            "Optimize run requires --confirm",
        ));
    }
    let plan = load_plan(&command.plan)?;
    let report = MaintenanceExecutor::default()
        .execute(devsweep_core::optimize::MaintenanceExecutionRequest {
            plan: &plan,
            expected_preview_digest: &command.preview_digest,
            confirmed: command.confirm,
            cancel: None,
        })
        .map_err(map_execution_error)?;
    let canceled = report
        .outcomes
        .iter()
        .all(|outcome| outcome.outcome == MaintenanceExecutionOutcome::CanceledBeforeStart);
    let succeeded = report.succeeded();
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::optimize::execution(locale, &report).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)?;
        }
        _ => {
            let outcome = if succeeded {
                "succeeded"
            } else if canceled {
                "canceled_before_start"
            } else {
                "failed"
            };
            write_envelope(
                cli,
                "optimize.run",
                outcome,
                serde_json::to_value(&report).map_err(|error| {
                    ApplicationError::failed("output_serialize_failed", error.to_string())
                })?,
            )?;
        }
    }

    if canceled {
        return Err(ApplicationError::after_output(
            ExitClass::CanceledBeforeDispatch,
            "optimize_canceled_before_start",
            "Optimize run was canceled before dispatch",
        ));
    }
    if !succeeded {
        return Err(ApplicationError::after_output(
            ExitClass::Failed,
            "optimize_run_not_successful",
            "Optimize run did not reach a successful terminal outcome",
        ));
    }
    Ok(())
}

fn load_plan(path: &Path) -> Result<MaintenancePlanV1, ApplicationError> {
    let bytes = fs::read(path).map_err(|error| {
        ApplicationError::failed(
            "optimize_plan_read_failed",
            format!("failed to read Optimize plan {}: {error}", path.display()),
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        ApplicationError::failed(
            "invalid_optimize_plan",
            format!("failed to decode Optimize plan {}: {error}", path.display()),
        )
    })
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

pub(super) fn map_plan_error(error: OptimizePlanError) -> ApplicationError {
    let (exit, code) = match &error {
        OptimizePlanError::UnsupportedPlanVersion(_)
        | OptimizePlanError::UnsupportedCatalogueVersion(_)
        | OptimizePlanError::DigestMismatch => {
            (ExitClass::InvalidAuthority, "invalid_optimize_authority")
        }
        OptimizePlanError::UnknownOperation(_) | OptimizePlanError::GuidanceNotExecutable(_) => {
            (ExitClass::Failed, "invalid_optimize_selection")
        }
        OptimizePlanError::PlatformUnsupported
        | OptimizePlanError::OsBuildUnavailable
        | OptimizePlanError::BuildUnsupported { .. }
        | OptimizePlanError::ResolverUnavailable => {
            (ExitClass::Unavailable, "optimize_preflight_unavailable")
        }
        OptimizePlanError::SerializationFailed => {
            (ExitClass::Failed, "optimize_plan_serialize_failed")
        }
    };
    ApplicationError::new(exit, code, error.to_string())
}

fn map_execution_error(error: MaintenanceExecutionError) -> ApplicationError {
    match error {
        MaintenanceExecutionError::Plan(error) => map_plan_error(error),
        MaintenanceExecutionError::ConfirmationRequired => {
            ApplicationError::usage("confirmation_required", "Optimize run requires --confirm")
        }
        MaintenanceExecutionError::Audit(_) => ApplicationError::new(
            ExitClass::Unavailable,
            "optimize_audit_unavailable",
            error.to_string(),
        ),
        MaintenanceExecutionError::PermitPoisoned => {
            ApplicationError::failed("optimize_execution_failed", error.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use devsweep_core::optimize::{
        MaintenanceActionClass, MaintenanceActionOutcomeV1, MaintenanceExecutionOutcome,
        MaintenanceExecutionReportV1,
    };
    use tempfile::TempDir;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("optimize CLI parses")
    }

    fn report(outcome: MaintenanceExecutionOutcome) -> MaintenanceExecutionReportV1 {
        MaintenanceExecutionReportV1 {
            version: 1,
            catalogue_version: 1,
            outcomes: vec![MaintenanceActionOutcomeV1 {
                operation_id: "optimize-op-fixture".to_string(),
                catalogue_id: "dns.flush".to_string(),
                action_class: MaintenanceActionClass::Execute,
                outcome,
                error_code: None,
            }],
        }
    }

    #[test]
    fn list_documents_are_locale_invariant_and_closed() {
        let temp = TempDir::new().unwrap();
        let en_path = temp.path().join("en.json");
        let zh_path = temp.path().join("zh.json");
        let en = parse(&[
            "devsweep",
            "optimize",
            "list",
            "--format",
            "json",
            "--output",
            en_path.to_str().unwrap(),
        ]);
        let zh = parse(&[
            "devsweep",
            "optimize",
            "list",
            "--format",
            "json",
            "--output",
            zh_path.to_str().unwrap(),
        ]);
        emit_list(&en, Locale::En).unwrap();
        emit_list(&zh, Locale::ZhCn).unwrap();
        let en_bytes = fs::read(&en_path).unwrap();
        assert_eq!(en_bytes, fs::read(&zh_path).unwrap());
        let document: serde_json::Value = serde_json::from_slice(&en_bytes).unwrap();
        assert_eq!(document["command"], "optimize.list");
        assert_eq!(document["data"]["catalogue_version"], 1);
        let entries = document["data"]["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 8);
        assert_eq!(entries[0]["id"], "dns.flush");
        assert_eq!(entries[0]["action_class"], "execute");
        let hostile = serde_json::to_string(&entries).unwrap();
        assert!(!hostile.contains("program"));
        assert!(!hostile.contains("argv"));
    }

    #[test]
    fn plan_writes_versioned_single_id_json_and_rejects_selections() {
        let temp = TempDir::new().unwrap();
        let plan_path = temp.path().join("plan.json");
        let command = super::super::super::cli::OptimizePlanCommand {
            operation: "dns.flush".to_string(),
            output: plan_path.clone(),
        };
        run_plan(&command).unwrap();
        let text = fs::read_to_string(&plan_path).unwrap();
        let document: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(document["version"], 1);
        assert_eq!(document["catalogue_version"], 1);
        assert_eq!(document["operation_id"], "dns.flush");
        assert!(!text.contains("program"));

        let hostile = super::super::super::cli::OptimizePlanCommand {
            operation: "cmd.exe /c calc".to_string(),
            output: temp.path().join("hostile.json"),
        };
        let error = run_plan(&hostile).expect_err("hostile operation fails closed");
        assert_eq!(error.code, "invalid_optimize_selection");
        assert!(!temp.path().join("hostile.json").exists());

        let guidance = super::super::super::cli::OptimizePlanCommand {
            operation: "guidance.drive_optimize".to_string(),
            output: temp.path().join("guidance.json"),
        };
        let error = run_plan(&guidance).expect_err("guidance never plans");
        assert_eq!(error.code, "invalid_optimize_selection");
        assert!(!temp.path().join("guidance.json").exists());
    }

    #[test]
    fn preview_is_locale_invariant_or_fails_closed_off_windows() {
        let temp = TempDir::new().unwrap();
        let plan_path = temp.path().join("plan.json");
        run_plan(&super::super::super::cli::OptimizePlanCommand {
            operation: "dns.flush".to_string(),
            output: plan_path.clone(),
        })
        .unwrap();
        let en_path = temp.path().join("en.json");
        let zh_path = temp.path().join("zh.json");
        let en = parse(&[
            "devsweep",
            "optimize",
            "preview",
            "--plan",
            plan_path.to_str().unwrap(),
            "--format",
            "json",
            "--output",
            en_path.to_str().unwrap(),
        ]);
        let zh = parse(&[
            "devsweep",
            "optimize",
            "preview",
            "--plan",
            plan_path.to_str().unwrap(),
            "--format",
            "json",
            "--output",
            zh_path.to_str().unwrap(),
        ]);
        let Some(Command::Optimize(group)) = en.command.as_ref() else {
            panic!("optimize command");
        };
        let OptimizeCommand::Preview(en_command) = &group.command else {
            panic!("preview command");
        };
        match run_preview(en_command, Locale::En, &en) {
            Ok(()) => {
                let Some(Command::Optimize(group)) = zh.command.as_ref() else {
                    panic!("optimize command");
                };
                let OptimizeCommand::Preview(zh_command) = &group.command else {
                    panic!("preview command");
                };
                run_preview(zh_command, Locale::ZhCn, &zh).unwrap();
                let en_bytes = fs::read(&en_path).unwrap();
                assert_eq!(en_bytes, fs::read(&zh_path).unwrap());
                let text = String::from_utf8(en_bytes).unwrap();
                assert!(text.contains("sha256:"));
                assert!(!text.contains("ipconfig"));
                assert!(!text.contains("ms-settings"));
            }
            Err(error) => {
                // Non-Windows host: the live preflight fails closed before any
                // document is emitted.
                assert_eq!(error.code, "optimize_preflight_unavailable");
                assert!(!en_path.exists());
            }
        }
    }

    #[test]
    fn execution_machine_document_never_claims_settings_completion() {
        let temp = TempDir::new().unwrap();
        let output = temp.path().join("result.json");
        let cli = parse(&[
            "devsweep",
            "optimize",
            "run",
            "--plan",
            "fixture.json",
            "--preview-digest",
            &format!("sha256:{}", "3".repeat(64)),
            "--confirm",
            "--format",
            "json",
            "--output",
            output.to_str().unwrap(),
        ]);
        write_envelope(
            &cli,
            "optimize.run",
            "succeeded",
            serde_json::to_value(report(MaintenanceExecutionOutcome::Launched)).unwrap(),
        )
        .unwrap();
        let text = fs::read_to_string(&output).unwrap();
        assert!(text.contains("launched"));
        assert!(!text.contains("completed"));
        assert!(!text.contains("\"reversible\":"));
    }

    #[test]
    fn plan_error_mapping_is_typed_by_exit_class() {
        let cases = [
            OptimizePlanError::UnsupportedPlanVersion(9),
            OptimizePlanError::UnsupportedCatalogueVersion(9),
            OptimizePlanError::DigestMismatch,
            OptimizePlanError::UnknownOperation("x".to_string()),
            OptimizePlanError::GuidanceNotExecutable("x".to_string()),
            OptimizePlanError::PlatformUnsupported,
            OptimizePlanError::OsBuildUnavailable,
            OptimizePlanError::BuildUnsupported { build: 1, floor: 2 },
            OptimizePlanError::ResolverUnavailable,
        ];
        for error in cases {
            let mapped = map_plan_error(error.clone());
            match error {
                OptimizePlanError::UnknownOperation(_)
                | OptimizePlanError::GuidanceNotExecutable(_) => {
                    assert_eq!(mapped.code, "invalid_optimize_selection")
                }
                OptimizePlanError::PlatformUnsupported
                | OptimizePlanError::OsBuildUnavailable
                | OptimizePlanError::BuildUnsupported { .. }
                | OptimizePlanError::ResolverUnavailable => {
                    assert_eq!(mapped.code, "optimize_preflight_unavailable")
                }
                _ => assert_eq!(mapped.code, "invalid_optimize_authority"),
            }
        }
    }

    #[test]
    fn hostile_plan_fields_fail_closed_without_echoing_payloads() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("hostile.json");
        fs::write(
            &path,
            "{\"version\":1,\"catalogue_version\":1,\"operation_id\":\"dns.flush\",\
             \"program\":\"cmd.exe\",\"argv\":[\"/c\",\"calc\"]}",
        )
        .unwrap();
        let error = load_plan(&path).expect_err("unknown executable fields fail closed");
        assert_eq!(error.code, "invalid_optimize_plan");
        assert!(!error.message.contains("cmd.exe"));
        assert!(!error.message.contains("calc"));
    }
}
