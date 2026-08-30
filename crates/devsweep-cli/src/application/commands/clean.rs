//! Frozen-grammar Clean handler.

use std::{fs, path::Path};

use devsweep_core::{
    execution::{ConfirmationDigest, ExecutionError, ExecutionReport, ExecutionRequest, Executor},
    model::{CleanupIntent, ScanReport, TargetId, UntrustedPlan},
    plan::validate_plan,
    scan::{ScanOptions, Sweeper},
};

use super::super::{
    cli::{
        CleanCommand, CleanPlanCommand, CleanScanCommand, CleanScope, Cli, Command,
        PlanExecutionCommand, PlanPresentationCommand,
    },
    output::{ApplicationError, ExitClass, OutputSink, ResultFormat, write_json},
    presentation,
};
use crate::i18n::{Locale, catalogue};

#[cfg(test)]
pub(super) struct Route;

pub(super) fn run(cli: &Cli, locale: Locale) -> Result<(), ApplicationError> {
    match cli.command.as_ref() {
        Some(Command::Clean(group)) => match &group.command {
            CleanCommand::Scan(command) => run_scan(command, locale, cli),
            CleanCommand::Plan(command) => run_plan(command, locale),
            CleanCommand::Preview(command) => run_preview(command, locale, cli),
            CleanCommand::Execute(command) => run_execute(command, locale, cli),
            CleanCommand::Protect(_) | CleanCommand::Rules(_) => Err(mode_unavailable(cli, locale)),
        },
        _ => Err(mode_unavailable(cli, locale)),
    }
}

fn mode_unavailable(cli: &Cli, locale: Locale) -> ApplicationError {
    let command = cli.command_name().unwrap_or("clean");
    let message = catalogue(locale)
        .render("error.mode_unavailable", &[("command", command)], None)
        .unwrap_or_else(|_| format!("unavailable: {command}"));
    ApplicationError::unavailable(message)
}

fn run_scan(command: &CleanScanCommand, locale: Locale, cli: &Cli) -> Result<(), ApplicationError> {
    let options = ScanOptions {
        include_projects: matches!(command.scope, CleanScope::Projects | CleanScope::All),
        include_global: matches!(command.scope, CleanScope::Global | CleanScope::All),
        roots: command.roots.clone(),
    };
    let mut progress = |_event| {};
    let report = if let Some(target_id) = &command.rescan_target {
        Sweeper::default()
            .full_scan_report_rescanning_target(
                &options,
                &TargetId::new(target_id.clone()),
                &mut progress,
            )
            .map_err(|error| map_anyhow("scan_failed", error))?
    } else {
        Sweeper::default()
            .full_scan_report(&options, &mut progress)
            .map_err(|error| map_anyhow("scan_failed", error))?
    };
    emit_scan(cli, locale, &report)
}

fn run_plan(command: &CleanPlanCommand, locale: Locale) -> Result<(), ApplicationError> {
    let observation = load_scan_report(&command.observation)?;
    let mut selected = Vec::new();
    for id in &command.select {
        let target = observation
            .plan
            .targets
            .iter()
            .find(|target| target.id.as_str() == id)
            .ok_or_else(|| {
                ApplicationError::failed("unknown_target", format!("unknown cleanup target: {id}"))
            })?;
        if matches!(target.intent, CleanupIntent::InspectOnly { .. }) {
            let message = catalogue(locale)
                .render("clean.v1.error.inspect_only", &[], None)
                .unwrap_or_else(|_| "inspect-only".to_string());
            return Err(ApplicationError::failed("inspect_only_target", message));
        }
        let mut selected_target = target.clone();
        selected_target.selected_by_default = true;
        selected.push(selected_target);
    }
    let plan = UntrustedPlan {
        version: observation.plan.version,
        targets: selected,
    };
    let _ = validate_plan(&plan).map_err(|error| map_anyhow("invalid_plan", error))?;
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
    let (validated, selected) = load_validated_selection(&command.plan)?;
    let report = Executor::default()
        .run_plan(
            &validated,
            ExecutionRequest {
                execute: false,
                audit_log: None,
                selected,
                expected_digest: None,
                cancel: None,
            },
        )
        .map_err(|error| map_anyhow("preview_failed", error))?;
    let digest = prefixed_digest(&report);
    emit_preview(cli, locale, &report, &digest)
}

fn run_execute(
    command: &PlanExecutionCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    let (validated, selected) = load_validated_selection(&command.plan)?;
    let digest = command
        .preview_digest
        .strip_prefix("sha256:")
        .unwrap_or(&command.preview_digest);
    let report = match Executor::default().run_plan(
        &validated,
        ExecutionRequest {
            execute: true,
            audit_log: None,
            selected,
            expected_digest: Some(ConfirmationDigest::new(digest)),
            cancel: None,
        },
    ) {
        Ok(report) => report,
        Err(error) => {
            if let Some(ExecutionError::StaleConfirmation { .. }) = error.downcast_ref() {
                let message = catalogue(locale)
                    .render("clean.v1.error.stale", &[], None)
                    .unwrap_or_else(|_| "stale preview digest".to_string());
                return Err(ApplicationError::new(
                    ExitClass::InvalidAuthority,
                    "stale_confirmation",
                    message,
                ));
            }
            return Err(map_anyhow("execute_failed", error));
        }
    };
    emit_execution(cli, locale, &report)?;
    if report.failed > 0 && report.succeeded > 0 {
        return Err(ApplicationError::new(
            ExitClass::Partial,
            "partial_execution",
            catalogue(locale)
                .render("clean.v1.execute.partial", &[], None)
                .unwrap_or_else(|_| "partial".to_string()),
        ));
    }
    if report.failed > 0 {
        return Err(ApplicationError::failed(
            "execution_failed",
            catalogue(locale)
                .render("clean.v1.execute.failed", &[], None)
                .unwrap_or_else(|_| "failed".to_string()),
        ));
    }
    Ok(())
}

fn load_scan_report(path: &Path) -> Result<ScanReport, ApplicationError> {
    let bytes = fs::read(path).map_err(|error| {
        ApplicationError::failed(
            "observation_read_failed",
            format!("failed to read {}: {error}", path.display()),
        )
    })?;
    if let Ok(envelope) = serde_json::from_slice::<serde_json::Value>(&bytes)
        && envelope.get("schema_version") == Some(&serde_json::json!(1))
        && let Some(data) = envelope.get("data")
    {
        return serde_json::from_value(data.clone()).map_err(|error| {
            ApplicationError::failed(
                "invalid_observation",
                format!("failed to decode observation {}: {error}", path.display()),
            )
        });
    }
    serde_json::from_slice(&bytes).map_err(|error| {
        ApplicationError::failed(
            "invalid_observation",
            format!("failed to decode observation {}: {error}", path.display()),
        )
    })
}

fn load_validated_selection(
    path: &Path,
) -> Result<(devsweep_core::plan::ValidatedPlan, Vec<TargetId>), ApplicationError> {
    let bytes = fs::read(path).map_err(|error| {
        ApplicationError::failed(
            "plan_read_failed",
            format!("failed to read {}: {error}", path.display()),
        )
    })?;
    let plan: UntrustedPlan = serde_json::from_slice(&bytes).map_err(|error| {
        ApplicationError::failed(
            "invalid_plan",
            format!("failed to decode plan {}: {error}", path.display()),
        )
    })?;
    let validated = validate_plan(&plan).map_err(|error| map_anyhow("invalid_plan", error))?;
    let selected = validated.default_selected_ids();
    Ok((validated, selected))
}

fn prefixed_digest(report: &ExecutionReport) -> String {
    format!("sha256:{}", report.confirmation_digest.as_str())
}
fn emit_scan(cli: &Cli, locale: Locale, report: &ScanReport) -> Result<(), ApplicationError> {
    match cli.result_format() {
        ResultFormat::Human => {
            let key = if report.plan.targets.is_empty() {
                "clean.v1.scan.empty"
            } else {
                "clean.v1.scan.complete"
            };
            let count = report.plan.targets.len().to_string();
            let text =
                presentation::render(locale, key, &[("count", &count)], None).map_err(|error| {
                    ApplicationError::failed("catalogue_render_failed", error.to_string())
                })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "clean.scan",
            "succeeded",
            serde_json::to_value(report).unwrap(),
        ),
    }
}

fn emit_preview(
    cli: &Cli,
    locale: Locale,
    report: &ExecutionReport,
    digest: &str,
) -> Result<(), ApplicationError> {
    match cli.result_format() {
        ResultFormat::Human => {
            let count = report.selected.to_string();
            let text = presentation::render(
                locale,
                "clean.v1.preview.digest",
                &[("digest", digest), ("count", &count)],
                None,
            )
            .or_else(|_| {
                presentation::render(
                    locale,
                    "clean.v1.preview.digest",
                    &[("digest", digest)],
                    None,
                )
            })
            .map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "clean.preview",
            "succeeded",
            serde_json::json!({ "digest": digest, "report": report }),
        ),
    }
}

fn emit_execution(
    cli: &Cli,
    locale: Locale,
    report: &ExecutionReport,
) -> Result<(), ApplicationError> {
    let outcome = if report.failed > 0 && report.succeeded > 0 {
        "partial"
    } else if report.failed > 0 {
        "failed"
    } else {
        "succeeded"
    };
    match cli.result_format() {
        ResultFormat::Human => {
            let key = match outcome {
                "partial" => "clean.v1.execute.partial",
                "failed" => "clean.v1.execute.failed",
                _ => "clean.v1.execute.completed",
            };
            let text = presentation::render(locale, key, &[], None).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "clean.execute",
            outcome,
            serde_json::to_value(report).unwrap(),
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

fn map_anyhow(code: &'static str, error: anyhow::Error) -> ApplicationError {
    ApplicationError::failed(code, format!("{error:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use devsweep_core::model::UntrustedTarget;
    use tempfile::TempDir;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("cli parses")
    }

    #[test]
    fn preview_empty_plan_is_dry_run() {
        let fixture = TempDir::new().unwrap();
        let plan_path = fixture.path().join("plan.json");
        fs::write(
            &plan_path,
            serde_json::to_vec(&UntrustedPlan::empty()).unwrap(),
        )
        .unwrap();
        let cli = parse(&[
            "devsweep",
            "clean",
            "preview",
            "--plan",
            plan_path.to_str().unwrap(),
            "--format",
            "json",
        ]);
        run(&cli, Locale::En).expect("preview succeeds");
    }

    #[test]
    fn plan_rejects_inspect_only_selection() {
        let fixture = TempDir::new().unwrap();
        let observation_path = fixture.path().join("observation.json");
        let output_path = fixture.path().join("plan.json");
        let target = UntrustedTarget {
            id: TargetId::new("cargo.home.inspect:x"),
            rule_id: "cargo.home.inspect".to_string(),
            scope: devsweep_core::model::Scope::Global,
            ecosystem: devsweep_core::model::Ecosystem::Rust,
            kind: devsweep_core::model::TargetKind::PackageCache,
            path: None,
            estimated_bytes: 0,
            size_complete: true,
            sizing_warnings: Vec::new(),
            last_modified: None,
            risk: devsweep_core::model::RiskLevel::Low,
            reversible: true,
            selected_by_default: false,
            evidence: vec![devsweep_core::model::Evidence::RuleMatched {
                rule_id: "cargo.home.inspect".to_string(),
            }],
            intent: CleanupIntent::InspectOnly {
                rule_id: "cargo.home.inspect".to_string(),
            },
        };
        let report = ScanReport::new(
            UntrustedPlan {
                version: devsweep_core::model::CLEANUP_PLAN_VERSION,
                targets: vec![target],
            },
            devsweep_core::model::ScanHealth::complete(),
        );
        fs::write(&observation_path, serde_json::to_vec(&report).unwrap()).unwrap();
        let cli = parse(&[
            "devsweep",
            "clean",
            "plan",
            "--observation",
            observation_path.to_str().unwrap(),
            "--select",
            "cargo.home.inspect:x",
            "--output",
            output_path.to_str().unwrap(),
        ]);
        let error = run(&cli, Locale::En).expect_err("inspect-only is refused");
        assert_eq!(error.code, "inspect_only_target");
        assert!(!output_path.exists());
    }

    #[test]
    fn stale_execute_is_invalid_authority() {
        let fixture = TempDir::new().unwrap();
        let plan_path = fixture.path().join("plan.json");
        fs::write(
            &plan_path,
            serde_json::to_vec(&UntrustedPlan::empty()).unwrap(),
        )
        .unwrap();
        let digest = format!("sha256:{}", "a".repeat(64));
        let cli = parse(&[
            "devsweep",
            "clean",
            "execute",
            "--plan",
            plan_path.to_str().unwrap(),
            "--preview-digest",
            &digest,
            "--confirm",
        ]);
        let error = run(&cli, Locale::En).expect_err("stale digest");
        assert_eq!(error.code, "stale_confirmation");
        assert_eq!(error.exit, ExitClass::InvalidAuthority);
    }
}
