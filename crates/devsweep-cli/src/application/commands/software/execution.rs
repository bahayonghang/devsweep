//! Frozen-grammar Software preview and current-user MSIX uninstall handlers.

use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use devsweep_core::software::{
    SoftwareAuditError, SoftwareExecutionError, SoftwareExecutionOutcome,
    SoftwareExecutionReportV1, SoftwareExecutionRequest, SoftwareExecutor, SoftwareInventorySource,
    SoftwarePreviewV1, SoftwareSelectionPlanV1, inventory_software, preview_selection_plan_live,
};

use super::super::super::{
    cli::{Cli, PlanExecutionCommand, PlanPresentationCommand},
    output::{ApplicationError, ExitClass, OutputSink, ResultFormat, write_json},
};
use crate::i18n::Locale;

#[cfg(test)]
pub(super) struct Route;

pub(super) fn run_preview(
    command: &PlanPresentationCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    let plan = load_plan(&command.plan)?;
    let live = inventory_software(SoftwareInventorySource::Msix, None).map_err(|error| {
        ApplicationError::new(
            ExitClass::Unavailable,
            "software_revalidation_unavailable",
            format!("Software live revalidation failed: {error:#}"),
        )
    })?;
    let preview = preview_selection_plan_live(&plan, &live, unix_ms(SystemTime::now()))
        .map_err(super::inventory::map_plan_error)?;
    emit_preview(cli, locale, &preview)
}

pub(super) fn run_uninstall(
    command: &PlanExecutionCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    if !command.confirm {
        return Err(ApplicationError::usage(
            "confirmation_required",
            "Software uninstall requires --confirm",
        ));
    }
    let plan = load_plan(&command.plan)?;
    let report = SoftwareExecutor::default()
        .execute(SoftwareExecutionRequest {
            plan: &plan,
            expected_preview_digest: &command.preview_digest,
            confirmed: command.confirm,
            cancel: None,
        })
        .map_err(map_execution_error)?;
    emit_execution(cli, locale, &report)?;

    if report
        .outcomes
        .iter()
        .all(|outcome| outcome.outcome == SoftwareExecutionOutcome::CanceledBeforeStart)
    {
        return Err(ApplicationError::after_output(
            ExitClass::CanceledBeforeDispatch,
            "software_canceled_before_start",
            "Software uninstall was canceled before dispatch",
        ));
    }
    if !report.succeeded() {
        return Err(ApplicationError::after_output(
            ExitClass::Failed,
            "software_uninstall_not_removed",
            "Software uninstall did not prove removal",
        ));
    }
    Ok(())
}

fn load_plan(path: &Path) -> Result<SoftwareSelectionPlanV1, ApplicationError> {
    let bytes = fs::read(path).map_err(|error| {
        ApplicationError::failed(
            "software_plan_read_failed",
            format!("failed to read Software plan {}: {error}", path.display()),
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        ApplicationError::failed(
            "invalid_software_plan",
            format!("failed to decode Software plan {}: {error}", path.display()),
        )
    })
}

fn emit_preview(
    cli: &Cli,
    locale: Locale,
    preview: &SoftwarePreviewV1,
) -> Result<(), ApplicationError> {
    match cli.result_format() {
        ResultFormat::Human => {
            let mut text = match locale {
                Locale::En => format!(
                    "Software preview: {} exact current-user MSIX package(s). Digest: {}. Uninstall is irreversible from DevSweep's perspective.",
                    preview.selected.len(),
                    preview.digest
                ),
                Locale::ZhCn => format!(
                    "软件预览：{} 个精确的当前用户 MSIX 包。摘要：{}。从 DevSweep 的角度看，卸载不可逆。",
                    preview.selected.len(),
                    preview.digest
                ),
            };
            for item in &preview.selected {
                let identity = serde_json::to_string(&item.identity).map_err(|error| {
                    ApplicationError::failed("output_serialize_failed", error.to_string())
                })?;
                text.push('\n');
                text.push_str(&identity);
            }
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "software.preview",
            "succeeded",
            serde_json::to_value(preview).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
}

fn emit_execution(
    cli: &Cli,
    locale: Locale,
    report: &SoftwareExecutionReportV1,
) -> Result<(), ApplicationError> {
    let outcome = if report.succeeded() {
        "succeeded"
    } else if report
        .outcomes
        .iter()
        .all(|item| item.outcome == SoftwareExecutionOutcome::CanceledBeforeStart)
    {
        "canceled_before_start"
    } else {
        "failed"
    };
    match cli.result_format() {
        ResultFormat::Human => {
            let removed = report
                .outcomes
                .iter()
                .filter(|item| item.outcome == SoftwareExecutionOutcome::Removed)
                .count();
            let reboot = report
                .outcomes
                .iter()
                .filter(|item| item.outcome == SoftwareExecutionOutcome::RebootRequired)
                .count();
            let text = match locale {
                Locale::En => format!(
                    "Software uninstall finished: {removed} removed, {reboot} require reboot, {} other terminal result(s). Every action is irreversible from DevSweep's perspective.",
                    report.outcomes.len().saturating_sub(removed + reboot)
                ),
                Locale::ZhCn => format!(
                    "软件卸载已结束：{removed} 个已移除，{reboot} 个需要重启，{} 个为其他终态。从 DevSweep 的角度看，每项操作均不可逆。",
                    report.outcomes.len().saturating_sub(removed + reboot)
                ),
            };
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "software.uninstall",
            outcome,
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

fn map_execution_error(error: SoftwareExecutionError) -> ApplicationError {
    match error {
        SoftwareExecutionError::Plan(error) => super::inventory::map_plan_error(error),
        SoftwareExecutionError::ConfirmationRequired => ApplicationError::usage(
            "confirmation_required",
            "Software uninstall requires --confirm",
        ),
        SoftwareExecutionError::LiveRevalidationUnavailable => ApplicationError::new(
            ExitClass::Unavailable,
            "software_revalidation_unavailable",
            error.to_string(),
        ),
        SoftwareExecutionError::Audit(SoftwareAuditError::LockUnavailable(_))
        | SoftwareExecutionError::Audit(SoftwareAuditError::LocalAppDataUnavailable) => {
            ApplicationError::new(
                ExitClass::Unavailable,
                "software_audit_unavailable",
                error.to_string(),
            )
        }
        SoftwareExecutionError::Audit(_)
        | SoftwareExecutionError::PermitPoisoned
        | SoftwareExecutionError::CrashInjected(_) => {
            ApplicationError::failed("software_execution_failed", error.to_string())
        }
    }
}

fn unix_ms(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |duration| {
        u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use devsweep_core::software::{
        SoftwareActionClass, SoftwareEligibilityReason, SoftwareIdentity, SoftwarePreviewItemV1,
        SoftwareRebootEvidence, SoftwareScope,
    };
    use tempfile::TempDir;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("software CLI parses")
    }

    fn preview() -> SoftwarePreviewV1 {
        SoftwarePreviewV1 {
            version: 1,
            inventory_fingerprint: format!("sha256:{}", "1".repeat(64)),
            irreversible: true,
            selected: vec![SoftwarePreviewItemV1 {
                id: "software:v1:msix:fixture".to_string(),
                identity: SoftwareIdentity::Msix {
                    package_full_name: "Fixture_1.0.0.0_x64__publisher".to_string(),
                },
                action_class: SoftwareActionClass::RemoveCurrentUserMsix,
                scope: SoftwareScope::CurrentUser,
                eligibility: SoftwareEligibilityReason::EligibleCurrentUserMsix,
                strategy_token: format!("software-token:sha256:{}", "2".repeat(64)),
            }],
            digest: format!("sha256:{}", "3".repeat(64)),
        }
    }

    #[test]
    fn preview_machine_document_is_locale_invariant_and_irreversible() {
        let temp = TempDir::new().unwrap();
        let en_path = temp.path().join("en.json");
        let zh_path = temp.path().join("zh.json");
        let en = parse(&[
            "devsweep",
            "software",
            "preview",
            "--plan",
            "fixture.json",
            "--format",
            "json",
            "--output",
            en_path.to_str().unwrap(),
        ]);
        let zh = parse(&[
            "devsweep",
            "software",
            "preview",
            "--plan",
            "fixture.json",
            "--format",
            "json",
            "--output",
            zh_path.to_str().unwrap(),
        ]);
        emit_preview(&en, Locale::En, &preview()).unwrap();
        emit_preview(&zh, Locale::ZhCn, &preview()).unwrap();
        let en_bytes = fs::read(&en_path).unwrap();
        assert_eq!(en_bytes, fs::read(&zh_path).unwrap());
        let document: serde_json::Value = serde_json::from_slice(&en_bytes).unwrap();
        assert_eq!(document["data"]["irreversible"], true);
        assert_eq!(
            document["data"]["selected"][0]["identity"]["source"],
            "msix"
        );
        assert!(
            !String::from_utf8(en_bytes)
                .unwrap()
                .contains("UninstallString")
        );
    }

    #[test]
    fn hostile_plan_fields_fail_closed_before_any_live_or_adapter_call() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("hostile.json");
        fs::write(
            &path,
            format!(
                "{{\"version\":1,\"inventory_fingerprint\":\"sha256:{}\",\"inventory_observed_at_unix_ms\":1,\"expires_at_unix_ms\":2,\"selected_ids\":[\"x\"],\"UninstallString\":\"cmd.exe /c calc\",\"argv\":[\"hostile\"]}}",
                "1".repeat(64)
            ),
        )
        .unwrap();
        let error = load_plan(&path).expect_err("unknown executable fields fail closed");
        assert_eq!(error.code, "invalid_software_plan");
        assert!(!error.message.contains("cmd.exe"));
        assert!(!error.message.contains("Remove-Item"));
    }

    #[test]
    fn execution_machine_document_never_uses_partial_or_reversible() {
        let temp = TempDir::new().unwrap();
        let output = temp.path().join("result.json");
        let cli = parse(&[
            "devsweep",
            "software",
            "uninstall",
            "--plan",
            "fixture.json",
            "--preview-digest",
            &format!("sha256:{}", "4".repeat(64)),
            "--confirm",
            "--format",
            "json",
            "--output",
            output.to_str().unwrap(),
        ]);
        let report = SoftwareExecutionReportV1 {
            version: 1,
            irreversible: true,
            outcomes: vec![devsweep_core::software::SoftwareActionOutcomeV1 {
                operation_id: "op".to_string(),
                software_id: "software:v1:msix:fixture".to_string(),
                outcome: SoftwareExecutionOutcome::UnknownAfterDispatch,
                installed_state: None,
                reboot_evidence: SoftwareRebootEvidence::None,
                error_code: None,
                irreversible: true,
            }],
        };
        emit_execution(&cli, Locale::En, &report).unwrap();
        let text = fs::read_to_string(output).unwrap();
        assert!(!text.contains("partial"));
        assert!(!text.contains("\"reversible\":"));
        assert!(text.contains("unknown_after_dispatch"));
    }

    #[test]
    fn emitted_failure_uses_one_machine_document_and_preserves_failed_exit() {
        let error = ApplicationError::after_output(
            ExitClass::Failed,
            "software_uninstall_not_removed",
            "not removed",
        );
        assert!(error.output_already_emitted);
        assert_eq!(error.exit, ExitClass::Failed);
    }
}
