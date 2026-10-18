//! Leftover scan, plan, strict preview, and strict execute.

use std::{fs, path::Path};

use devsweep_core::software::{
    SoftwareLeftoverError, SoftwareLeftoverPlanV1, SoftwareLeftoverPreviewV1,
    SoftwareLeftoverSelectionV1, SoftwareLeftoverStrictOutcome, SoftwareLeftoverStrictPreviewV1,
    SoftwareLeftoverStrictReportV1, SoftwareRegistryResiduePlanV1,
    SoftwareRegistryResiduePreviewV1, SoftwareRegistryResidueReportV1,
    SoftwareRegistryResidueRestorePlanV1, SoftwareRegistryResidueRestorePreviewV1,
    discover_software_leftovers, execute_registry_residue, execute_registry_residue_restore,
    execute_software_leftover_plan, plan_registry_residue, plan_registry_residue_restore,
    plan_software_leftovers, preview_registry_residue, preview_registry_residue_restore,
    preview_software_leftover_plan,
};

use super::super::super::{
    cli::{
        Cli, SoftwareLeftoverRegistryExecuteCommand, SoftwareLeftoverRegistryPlanCommand,
        SoftwareLeftoverRegistryPreviewCommand, SoftwareLeftoverRegistryRestoreExecuteCommand,
        SoftwareLeftoverRegistryRestorePlanCommand, SoftwareLeftoverRegistryRestorePreviewCommand,
        SoftwareLeftoversExecuteCommand, SoftwareLeftoversPlanCommand,
        SoftwareLeftoversPreviewCommand, SoftwareLeftoversScanCommand,
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

pub(super) fn run_scan(
    command: &SoftwareLeftoversScanCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    let inventory = super::inventory::load_inventory(&command.inventory)?;
    let preview =
        discover_software_leftovers(&inventory, &command.select, None).map_err(map_error)?;
    emit_scan(cli, locale, &preview)
}

pub(super) fn run_plan(command: &SoftwareLeftoversPlanCommand) -> Result<(), ApplicationError> {
    let inventory = super::inventory::load_inventory(&command.inventory)?;
    let selection = SoftwareLeftoverSelectionV1 {
        software_id: command.software_id.clone(),
        uninstall_operation_id: command.uninstall_operation.clone(),
        selected_candidate_ids: command.select.clone(),
    };
    let (plan, _) = plan_software_leftovers(&inventory, &selection, None).map_err(map_error)?;
    write_plan(&command.output, &plan)
}

pub(super) fn run_preview(
    command: &SoftwareLeftoversPreviewCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    let plan = load_plan(&command.plan)?;
    let inventory = super::inventory::load_inventory(&command.inventory)?;
    let preview = preview_software_leftover_plan(&inventory, &plan, None).map_err(map_error)?;
    emit_preview(cli, locale, &preview)
}

pub(super) fn run_execute(
    command: &SoftwareLeftoversExecuteCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    if !command.confirm {
        return Err(ApplicationError::usage(
            "confirmation_required",
            "leftover removal requires explicit confirmation",
        ));
    }
    let plan = load_plan(&command.plan)?;
    ensure_output_absent(cli.output_path())?;
    let inventory = super::inventory::load_inventory(&command.inventory)?;
    let report = execute_software_leftover_plan(
        &inventory,
        &plan,
        &command.preview_digest,
        command.confirm,
        None,
    )
    .map_err(map_error)?;
    emit_execute(cli, locale, &report)?;
    match report.outcome {
        SoftwareLeftoverStrictOutcome::Succeeded => Ok(()),
        SoftwareLeftoverStrictOutcome::Failed => Err(ApplicationError::after_output(
            ExitClass::Failed,
            "leftover_move_failed",
            "leftover removal failed after dispatch",
        )),
        SoftwareLeftoverStrictOutcome::Unknown => Err(ApplicationError::after_output(
            ExitClass::Failed,
            "leftover_result_unknown",
            "leftover removal result is unknown",
        )),
        SoftwareLeftoverStrictOutcome::Refused => Err(ApplicationError::after_output(
            ExitClass::InvalidAuthority,
            "candidate_changed",
            "leftover removal was refused",
        )),
    }
}

pub(super) fn run_registry_plan(
    command: &SoftwareLeftoverRegistryPlanCommand,
) -> Result<(), ApplicationError> {
    let plan = plan_registry_residue(&command.key).map_err(map_error)?;
    write_registry_plan(&command.output, &plan)
}

pub(super) fn run_registry_preview(
    command: &SoftwareLeftoverRegistryPreviewCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    let plan = load_registry_plan(&command.plan)?;
    let preview = preview_registry_residue(&command.key, &plan).map_err(map_error)?;
    emit_residue_preview(cli, locale, &preview)
}

pub(super) fn run_registry_execute(
    command: &SoftwareLeftoverRegistryExecuteCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    if !command.confirm {
        return Err(ApplicationError::usage(
            "confirmation_required",
            "registry residue change requires explicit confirmation",
        ));
    }
    let plan = load_registry_plan(&command.plan)?;
    ensure_output_absent(cli.output_path())?;
    let report = execute_registry_residue(
        &command.key,
        &command.backup,
        &plan,
        &command.preview_digest,
        command.confirm,
    )
    .map_err(map_error)?;
    emit_residue_report(cli, locale, &report)
}

pub(super) fn run_registry_restore_plan(
    command: &SoftwareLeftoverRegistryRestorePlanCommand,
) -> Result<(), ApplicationError> {
    let report = load_residue_report(&command.report)?;
    let plan = plan_registry_residue_restore(&report).map_err(map_error)?;
    write_residue_restore_plan(&command.output, &plan)
}

pub(super) fn run_registry_restore_preview(
    command: &SoftwareLeftoverRegistryRestorePreviewCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    let plan = load_residue_restore_plan(&command.plan)?;
    let preview = preview_registry_residue_restore(&command.key, &command.backup, &plan)
        .map_err(map_error)?;
    emit_residue_restore_preview(cli, locale, &preview)
}

pub(super) fn run_registry_restore_execute(
    command: &SoftwareLeftoverRegistryRestoreExecuteCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    if !command.confirm {
        return Err(ApplicationError::usage(
            "confirmation_required",
            "registry residue restore requires explicit confirmation",
        ));
    }
    let plan = load_residue_restore_plan(&command.plan)?;
    ensure_output_absent(cli.output_path())?;
    execute_registry_residue_restore(
        &command.key,
        &command.backup,
        &plan,
        &command.preview_digest,
        command.confirm,
    )
    .map_err(map_error)?;
    emit_residue_restored(cli, locale)
}

fn load_plan(path: &Path) -> Result<SoftwareLeftoverPlanV1, ApplicationError> {
    let bytes = fs::read(path).map_err(|error| {
        ApplicationError::failed(
            "plan_read_failed",
            format!("failed to read leftover plan {}: {error}", path.display()),
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|_| ApplicationError::usage("invalid_plan", "leftover plan is invalid"))
}

fn map_error(error: SoftwareLeftoverError) -> ApplicationError {
    from_domain_code(error.code(), error.to_string())
}

fn emit_scan(
    cli: &Cli,
    locale: Locale,
    preview: &SoftwareLeftoverPreviewV1,
) -> Result<(), ApplicationError> {
    match cli.result_format() {
        ResultFormat::Human => {
            let text =
                presentation::software::leftovers_scan(locale, preview).map_err(|error| {
                    ApplicationError::failed("catalogue_render_failed", error.to_string())
                })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "software.leftovers.scan",
            "succeeded",
            serde_json::to_value(preview).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
}

fn emit_preview(
    cli: &Cli,
    locale: Locale,
    preview: &SoftwareLeftoverStrictPreviewV1,
) -> Result<(), ApplicationError> {
    match cli.result_format() {
        ResultFormat::Human => {
            let text =
                presentation::software::leftovers_preview(locale, preview).map_err(|error| {
                    ApplicationError::failed("catalogue_render_failed", error.to_string())
                })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "software.leftovers.preview",
            "succeeded",
            serde_json::to_value(preview).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
}

fn emit_execute(
    cli: &Cli,
    locale: Locale,
    report: &SoftwareLeftoverStrictReportV1,
) -> Result<(), ApplicationError> {
    let outcome = match report.outcome {
        SoftwareLeftoverStrictOutcome::Succeeded => "succeeded",
        SoftwareLeftoverStrictOutcome::Failed => "failed",
        SoftwareLeftoverStrictOutcome::Unknown => "unknown",
        SoftwareLeftoverStrictOutcome::Refused => "refused",
    };
    match cli.result_format() {
        ResultFormat::Human => {
            let text =
                presentation::software::leftovers_execute(locale, report).map_err(|error| {
                    ApplicationError::failed("catalogue_render_failed", error.to_string())
                })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "software.leftovers.execute",
            outcome,
            serde_json::to_value(report).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
}

fn load_registry_plan(path: &Path) -> Result<SoftwareRegistryResiduePlanV1, ApplicationError> {
    let invalid = "registry residue plan is invalid";
    serde_json::from_value(read_value(path, invalid)?)
        .map_err(|_| ApplicationError::usage("invalid_plan", invalid))
}

fn load_residue_report(path: &Path) -> Result<SoftwareRegistryResidueReportV1, ApplicationError> {
    let invalid = "registry residue report is invalid";
    serde_json::from_value(read_report_value(path, invalid)?)
        .map_err(|_| ApplicationError::usage("invalid_plan", invalid))
}

fn load_residue_restore_plan(
    path: &Path,
) -> Result<SoftwareRegistryResidueRestorePlanV1, ApplicationError> {
    let invalid = "registry residue restore plan is invalid";
    serde_json::from_value(read_value(path, invalid)?)
        .map_err(|_| ApplicationError::usage("invalid_plan", invalid))
}

fn read_value(path: &Path, invalid: &'static str) -> Result<serde_json::Value, ApplicationError> {
    let bytes = fs::read(path).map_err(|error| {
        ApplicationError::failed(
            "plan_read_failed",
            format!("failed to read {}: {error}", path.display()),
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|_| ApplicationError::usage("invalid_plan", invalid))
}

fn read_report_value(
    path: &Path,
    invalid: &'static str,
) -> Result<serde_json::Value, ApplicationError> {
    let value = read_value(path, invalid)?;
    Ok(value
        .get("schema_version")
        .and_then(|_| value.get("data"))
        .cloned()
        .unwrap_or(value))
}

fn write_registry_plan(
    path: &Path,
    plan: &SoftwareRegistryResiduePlanV1,
) -> Result<(), ApplicationError> {
    let value = serde_json::to_value(plan)
        .map_err(|error| ApplicationError::failed("output_serialize_failed", error.to_string()))?;
    write_pretty(path, &value)
}

fn write_residue_restore_plan(
    path: &Path,
    plan: &SoftwareRegistryResidueRestorePlanV1,
) -> Result<(), ApplicationError> {
    let value = serde_json::to_value(plan)
        .map_err(|error| ApplicationError::failed("output_serialize_failed", error.to_string()))?;
    write_pretty(path, &value)
}

fn write_pretty(path: &Path, value: &serde_json::Value) -> Result<(), ApplicationError> {
    let mut sink = OutputSink::open(Some(path))?;
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| ApplicationError::failed("output_serialize_failed", error.to_string()))?;
    bytes.push(b'\n');
    sink.write_all(&bytes)
        .map_err(|error| ApplicationError::failed("output_write_failed", error.to_string()))
}

fn emit_residue_preview(
    cli: &Cli,
    locale: Locale,
    preview: &SoftwareRegistryResiduePreviewV1,
) -> Result<(), ApplicationError> {
    match cli.result_format() {
        ResultFormat::Human => {
            let text =
                presentation::software::residue_preview(locale, preview).map_err(|error| {
                    ApplicationError::failed("catalogue_render_failed", error.to_string())
                })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "software.leftovers.registry-preview",
            "succeeded",
            serde_json::to_value(preview).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
}

fn emit_residue_report(
    cli: &Cli,
    locale: Locale,
    report: &SoftwareRegistryResidueReportV1,
) -> Result<(), ApplicationError> {
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::software::residue_report(locale, report).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "software.leftovers.registry-execute",
            "succeeded",
            serde_json::to_value(report).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
}

fn emit_residue_restore_preview(
    cli: &Cli,
    locale: Locale,
    preview: &SoftwareRegistryResidueRestorePreviewV1,
) -> Result<(), ApplicationError> {
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::software::residue_preview(
                locale,
                &SoftwareRegistryResiduePreviewV1 {
                    version: preview.version,
                    key_name: "DevSweepResidue".to_string(),
                    before_sha256: String::new(),
                    digest: preview.digest.clone(),
                },
            )
            .map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "software.leftovers.registry-restore-preview",
            "succeeded",
            serde_json::to_value(preview).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
}

fn emit_residue_restored(cli: &Cli, locale: Locale) -> Result<(), ApplicationError> {
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::software::residue_restored(locale).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "software.leftovers.registry-restore-execute",
            "succeeded",
            serde_json::Value::Null,
        ),
    }
}

fn write_plan(path: &Path, plan: &SoftwareLeftoverPlanV1) -> Result<(), ApplicationError> {
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
    use devsweep_core::software::{RegistryHive, RegistryView, SoftwareIdentity};
    use tempfile::TempDir;

    fn parse(args: &[&str]) -> Cli {
        let cli = Cli::try_parse_from(args).expect("leftover command parses");
        cli.validate().expect("leftover command validates");
        cli
    }

    fn execute(cli: &Cli) -> &SoftwareLeftoversExecuteCommand {
        let super::super::super::super::cli::Command::Software(group) =
            cli.command.as_ref().unwrap()
        else {
            panic!("software");
        };
        let super::super::super::super::cli::SoftwareCommand::Leftovers(group) = &group.command
        else {
            panic!("leftovers");
        };
        let super::super::super::super::cli::SoftwareLeftoversCommand::Execute(command) =
            &group.command
        else {
            panic!("execute");
        };
        command
    }

    fn sample_plan() -> SoftwareLeftoverPlanV1 {
        SoftwareLeftoverPlanV1 {
            version: 1,
            software_id: "software:v1:not-real".to_string(),
            identity: SoftwareIdentity::Arp {
                hive: RegistryHive::CurrentUser,
                view: RegistryView::Registry64,
                subkey: "not-real".to_string(),
            },
            display_name: None,
            publisher: None,
            uninstall_operation_id: "op-1".to_string(),
            selected_candidate_ids: vec!["candidate-1".to_string()],
        }
    }

    #[test]
    fn malformed_plan_is_usage_before_any_removal() {
        let fixture = TempDir::new().unwrap();
        let plan = fixture.path().join("plan.json");
        let output = fixture.path().join("out.json");
        fs::write(&plan, b"{").unwrap();
        fs::write(&output, b"keep").unwrap();
        let digest = format!("sha256:{}", "c".repeat(64));
        let cli = parse(&[
            "devsweep",
            "software",
            "leftovers",
            "execute",
            "--inventory",
            fixture
                .path()
                .join("missing-inventory.json")
                .to_str()
                .unwrap(),
            "--plan",
            plan.to_str().unwrap(),
            "--preview-digest",
            &digest,
            "--confirm",
            "--output",
            output.to_str().unwrap(),
        ]);
        let error = run_execute(execute(&cli), Locale::En, &cli).expect_err("malformed");
        assert_eq!(error.code, "invalid_plan");
        assert_eq!(error.exit, ExitClass::Usage);
        assert_eq!(fs::read(&output).unwrap(), b"keep");
    }

    #[test]
    fn existing_output_is_rejected_before_leftover_dispatch() {
        let fixture = TempDir::new().unwrap();
        let plan = fixture.path().join("plan.json");
        let output = fixture.path().join("out.json");
        fs::write(&plan, serde_json::to_vec(&sample_plan()).unwrap()).unwrap();
        fs::write(&output, b"keep").unwrap();
        let digest = format!("sha256:{}", "d".repeat(64));
        let cli = parse(&[
            "devsweep",
            "software",
            "leftovers",
            "execute",
            "--inventory",
            fixture
                .path()
                .join("missing-inventory.json")
                .to_str()
                .unwrap(),
            "--plan",
            plan.to_str().unwrap(),
            "--preview-digest",
            &digest,
            "--confirm",
            "--output",
            output.to_str().unwrap(),
        ]);
        let error = run_execute(execute(&cli), Locale::En, &cli).expect_err("collision");
        assert_eq!(error.code, "output_exists");
        assert_eq!(fs::read(&output).unwrap(), b"keep");
    }
}
