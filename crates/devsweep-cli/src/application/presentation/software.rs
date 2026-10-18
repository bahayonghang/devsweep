//! Bilingual Software V1 human renderers.

use devsweep_core::software::{
    SoftwareActionOutcomeV1, SoftwareEligibilityReason, SoftwareEligibilityState,
    SoftwareExecutionOutcome, SoftwareExecutionReportV1, SoftwareIdentity, SoftwareInventoryV1,
    SoftwareLastUsedEvidence, SoftwareLeftoverCertainty, SoftwareLeftoverOrigin,
    SoftwareLeftoverPreviewV1, SoftwareLeftoverStrictOutcome, SoftwareLeftoverStrictPreviewV1,
    SoftwareLeftoverStrictReportV1, SoftwarePreviewV1, SoftwareRegistryResiduePreviewV1,
    SoftwareRegistryResidueReportV1, SoftwareScope, SoftwareSizeBasis, SoftwareSizeEvidence,
    SoftwareSourceState, SoftwareStartupListV1, SoftwareStartupLocation, SoftwareStartupState,
    SoftwareUpdateOutcome, SoftwareUpdatePreviewV1, SoftwareUpdateReportV1, SoftwareUpdatesReason,
    SoftwareUpdatesV1, StartupHiveExecuteReportV1, StartupRestorePreviewV1, StartupStrictOutcome,
    StartupTogglePreviewV1, StartupToggleStrictReportV1,
};

use super::render;
use crate::i18n::{CatalogueError, Locale, format_binary_bytes};

#[cfg(test)]
pub(super) struct Route;

pub(crate) fn inventory(
    locale: Locale,
    inventory: &SoftwareInventoryV1,
) -> Result<String, CatalogueError> {
    let selectable = inventory
        .entries
        .iter()
        .filter(|entry| entry.eligibility.state == SoftwareEligibilityState::Selectable)
        .count();
    let manual = inventory.entries.len().saturating_sub(selectable);
    let source_count = |state| {
        inventory
            .sources
            .iter()
            .filter(|evidence| evidence.state == state)
            .count()
            .to_string()
    };
    let count = inventory.entries.len().to_string();
    let selectable = selectable.to_string();
    let manual = manual.to_string();
    let available = source_count(SoftwareSourceState::Available);
    let partial = source_count(SoftwareSourceState::Partial);
    let permission = source_count(SoftwareSourceState::Permission);
    let unsupported = source_count(SoftwareSourceState::Unsupported);
    let mut lines = vec![
        render(
            locale,
            "software.v1.inventory.summary",
            &[
                ("count", &count),
                ("selectable", &selectable),
                ("manual", &manual),
            ],
            None,
        )?,
        render(
            locale,
            "software.v1.inventory.sources",
            &[
                ("available", &available),
                ("partial", &partial),
                ("permission", &permission),
                ("unsupported", &unsupported),
            ],
            None,
        )?,
    ];
    for entry in &inventory.entries {
        let name = entry.display_name.as_deref().unwrap_or("<unnamed>");
        let identity = identity(&entry.identity);
        let scope = static_text(locale, scope_key(entry.scope))?;
        let eligibility = static_text(locale, eligibility_key(entry.eligibility.reason))?;
        let size = size(locale, &entry.size)?;
        let last_used = last_used(locale, entry.last_used)?;
        lines.push(render(
            locale,
            "software.v1.inventory.entry",
            &[
                ("name", name),
                ("id", entry.id.as_str()),
                ("identity", &identity),
                ("scope", &scope),
                ("eligibility", &eligibility),
                ("size", &size),
                ("last_used", &last_used),
            ],
            None,
        )?);
    }
    Ok(lines.join("\n"))
}

pub(crate) fn preview(
    locale: Locale,
    preview: &SoftwarePreviewV1,
) -> Result<String, CatalogueError> {
    let count = preview.selected.len().to_string();
    let mut lines = vec![render(
        locale,
        "software.v1.preview.summary",
        &[("count", &count), ("digest", preview.digest.as_str())],
        None,
    )?];
    for item in &preview.selected {
        let identity = identity(&item.identity);
        lines.push(render(
            locale,
            "software.v1.preview.item",
            &[("id", item.id.as_str()), ("identity", &identity)],
            None,
        )?);
    }
    Ok(lines.join("\n"))
}

pub(crate) fn execution(
    locale: Locale,
    report: &SoftwareExecutionReportV1,
) -> Result<String, CatalogueError> {
    let count = |outcome| {
        report
            .outcomes
            .iter()
            .filter(|item| item.outcome == outcome)
            .count()
            .to_string()
    };
    let removed = count(SoftwareExecutionOutcome::Removed);
    let reboot = count(SoftwareExecutionOutcome::RebootRequired);
    let present = count(SoftwareExecutionOutcome::StillPresent);
    let failed = count(SoftwareExecutionOutcome::Failed);
    let unknown = count(SoftwareExecutionOutcome::UnknownAfterDispatch);
    let canceled = count(SoftwareExecutionOutcome::CanceledBeforeStart);
    let mut lines = vec![render(
        locale,
        "software.v1.execution.summary",
        &[
            ("removed", &removed),
            ("reboot", &reboot),
            ("present", &present),
            ("failed", &failed),
            ("unknown", &unknown),
            ("canceled", &canceled),
        ],
        None,
    )?];
    for item in &report.outcomes {
        lines.push(execution_item(locale, item)?);
    }
    Ok(lines.join("\n"))
}

fn execution_item(
    locale: Locale,
    item: &SoftwareActionOutcomeV1,
) -> Result<String, CatalogueError> {
    let outcome = static_text(locale, outcome_key(item.outcome))?;
    render(
        locale,
        "software.v1.execution.item",
        &[
            ("id", item.software_id.as_str()),
            ("outcome", &outcome),
            ("operation", item.operation_id.as_str()),
        ],
        None,
    )
}

fn identity(identity: &SoftwareIdentity) -> String {
    match identity {
        SoftwareIdentity::Arp { hive, view, subkey } => {
            format!("arp:{hive:?}:{view:?}:{subkey}")
        }
        SoftwareIdentity::Msi {
            product_code,
            context,
        } => {
            format!("msi:{context:?}:{product_code}")
        }
        SoftwareIdentity::Msix { package_full_name } => {
            format!("msix:{package_full_name}")
        }
    }
}

fn static_text(locale: Locale, key: &'static str) -> Result<String, CatalogueError> {
    render(locale, key, &[], None)
}

fn size(locale: Locale, size: &SoftwareSizeEvidence) -> Result<String, CatalogueError> {
    match size {
        SoftwareSizeEvidence::Available {
            value_bytes, basis, ..
        } => {
            let bytes = format_binary_bytes(*value_bytes);
            let key = match basis {
                SoftwareSizeBasis::ReportedEstimate => "software.v1.size.estimated",
                SoftwareSizeBasis::MeasuredInstalledLocation => "software.v1.size.measured",
                SoftwareSizeBasis::MeasuredDirectory => "software.v1.size.measured_directory",
            };
            render(locale, key, &[("bytes", &bytes)], None)
        }
        SoftwareSizeEvidence::Partial {
            lower_bound_bytes, ..
        } => {
            let bytes = format_binary_bytes(*lower_bound_bytes);
            render(
                locale,
                "software.v1.size.lower_bound",
                &[("bytes", &bytes)],
                None,
            )
        }
        SoftwareSizeEvidence::Unknown { .. } => static_text(locale, "software.v1.size.unknown"),
    }
}

fn last_used(locale: Locale, evidence: SoftwareLastUsedEvidence) -> Result<String, CatalogueError> {
    match evidence {
        SoftwareLastUsedEvidence::Unknown { .. } => {
            static_text(locale, "software.v1.last_used.unknown")
        }
    }
}

fn scope_key(scope: SoftwareScope) -> &'static str {
    match scope {
        SoftwareScope::CurrentUser => "software.v1.scope.current_user",
        SoftwareScope::Machine => "software.v1.scope.machine",
    }
}

fn eligibility_key(reason: SoftwareEligibilityReason) -> &'static str {
    match reason {
        SoftwareEligibilityReason::ProtectedProduct => "software.v1.eligibility.protected_product",
        SoftwareEligibilityReason::SourceIncomplete => "software.v1.eligibility.source_incomplete",
        SoftwareEligibilityReason::ConflictingIdentity => {
            "software.v1.eligibility.conflicting_identity"
        }
        SoftwareEligibilityReason::NoRemove => "software.v1.eligibility.no_remove",
        SoftwareEligibilityReason::HiddenEntry => "software.v1.eligibility.hidden_entry",
        SoftwareEligibilityReason::SystemOrUpdate => "software.v1.eligibility.system_or_update",
        SoftwareEligibilityReason::DependencyPackage => {
            "software.v1.eligibility.dependency_package"
        }
        SoftwareEligibilityReason::StubPackage => "software.v1.eligibility.stub_package",
        SoftwareEligibilityReason::UnhealthyPackage => "software.v1.eligibility.unhealthy_package",
        SoftwareEligibilityReason::MsiExecutionNotSupportedV1 => {
            "software.v1.eligibility.msi_execution_not_supported_v1"
        }
        SoftwareEligibilityReason::RegistryOnlyManual => {
            "software.v1.eligibility.registry_only_manual"
        }
        SoftwareEligibilityReason::UnsupportedSource => {
            "software.v1.eligibility.unsupported_source"
        }
        SoftwareEligibilityReason::EligibleCurrentUserMsix => {
            "software.v1.eligibility.eligible_current_user_msix"
        }
        SoftwareEligibilityReason::EligibleMsiQuiet => "software.v1.eligibility.eligible_msi_quiet",
        SoftwareEligibilityReason::EligibleVendorUninstaller => {
            "software.v1.eligibility.eligible_vendor_uninstaller"
        }
        SoftwareEligibilityReason::EligibleScoopApp => "software.v1.eligibility.eligible_scoop_app",
    }
}

pub(crate) fn updates(
    locale: Locale,
    report: &SoftwareUpdatesV1,
) -> Result<String, CatalogueError> {
    match report {
        SoftwareUpdatesV1::Available { rows, .. } => {
            let count = rows.len().to_string();
            let mut lines = vec![render(
                locale,
                "software.v1.updates.summary",
                &[("count", &count)],
                None,
            )?];
            for row in rows {
                let installed = row.installed_version.as_str();
                let available = row.available_version.as_str();
                lines.push(render(
                    locale,
                    "software.cli.updates.row",
                    &[
                        ("name", row.name.as_str()),
                        ("id", row.id.as_str()),
                        ("installed", installed),
                        ("available", available),
                        ("source", row.source.as_str()),
                    ],
                    None,
                )?);
            }
            Ok(lines.join("\n"))
        }
        SoftwareUpdatesV1::Unavailable { reason_code, .. } => Ok([
            render(locale, "software.v1.updates.unavailable", &[], None)?,
            render(locale, updates_reason_key(*reason_code), &[], None)?,
        ]
        .join("\n")),
    }
}

pub(crate) fn startup_list(
    locale: Locale,
    list: &SoftwareStartupListV1,
) -> Result<String, CatalogueError> {
    let count = list.entries.len().to_string();
    let mut lines = vec![render(
        locale,
        "software.cli.startup.summary",
        &[("count", &count)],
        None,
    )?];
    for entry in &list.entries {
        let location = static_text(locale, startup_location_key(entry.location))?;
        let state = static_text(locale, startup_state_key(entry.state))?;
        lines.push(render(
            locale,
            "software.cli.startup.entry",
            &[
                ("name", entry.name.as_str()),
                ("location", &location),
                ("state", &state),
                ("id", entry.id.as_str()),
            ],
            None,
        )?);
    }
    Ok(lines.join("\n"))
}

pub(crate) fn startup_preview(
    locale: Locale,
    preview: &StartupTogglePreviewV1,
) -> Result<String, CatalogueError> {
    let before = static_text(locale, startup_state_key(preview.before_state))?;
    let after = static_text(locale, startup_state_key(preview.after_state))?;
    render(
        locale,
        "software.cli.startup.preview",
        &[
            ("digest", preview.digest.as_str()),
            ("before", &before),
            ("after", &after),
        ],
        None,
    )
}

pub(crate) fn update_preview(
    locale: Locale,
    preview: &SoftwareUpdatePreviewV1,
) -> Result<String, CatalogueError> {
    render(
        locale,
        "software.cli.updates.preview",
        &[
            ("package_id", preview.package_id.as_str()),
            ("source", preview.source.as_str()),
            ("installed_version", preview.installed_version.as_str()),
            ("available_version", preview.available_version.as_str()),
            ("digest", preview.digest.as_str()),
        ],
        None,
    )
}

pub(crate) fn update_report(
    locale: Locale,
    report: &SoftwareUpdateReportV1,
) -> Result<String, CatalogueError> {
    let outcome = match report.outcome {
        SoftwareUpdateOutcome::Updated => "updated",
        SoftwareUpdateOutcome::RestartRequired => "restart_required",
        SoftwareUpdateOutcome::Failed => "failed",
        SoftwareUpdateOutcome::Unknown => "unknown",
    };
    let updated = if report.updated { "true" } else { "false" };
    render(
        locale,
        "software.cli.updates.execute",
        &[
            ("package_id", report.package_id.as_str()),
            ("outcome", outcome),
            ("updated", updated),
        ],
        None,
    )
}

pub(crate) fn startup_hive_execute(
    locale: Locale,
    report: &StartupHiveExecuteReportV1,
) -> Result<String, CatalogueError> {
    let toggle = startup_execute(locale, &report.toggle)?;
    let outcome = match report.toggle.outcome {
        StartupStrictOutcome::Succeeded => "succeeded",
        StartupStrictOutcome::Failed => "failed",
        StartupStrictOutcome::Unknown => "unknown",
        StartupStrictOutcome::AlreadyInState => "already_in_state",
    };
    let line = render(
        locale,
        "software.cli.startup.hive",
        &[
            ("entry_id", report.toggle.entry.id.as_str()),
            ("outcome", outcome),
        ],
        None,
    )?;
    Ok(format!("{toggle}\n{line}"))
}

pub(crate) fn startup_restore_preview(
    locale: Locale,
    preview: &StartupRestorePreviewV1,
) -> Result<String, CatalogueError> {
    render(
        locale,
        "software.cli.startup.restore",
        &[("digest", preview.digest.as_str())],
        None,
    )
}

pub(crate) fn startup_restore_succeeded(locale: Locale) -> Result<String, CatalogueError> {
    static_text(locale, "software.cli.startup.restore.succeeded")
}

pub(crate) fn residue_preview(
    locale: Locale,
    preview: &SoftwareRegistryResiduePreviewV1,
) -> Result<String, CatalogueError> {
    render(
        locale,
        "software.cli.leftovers.residue",
        &[
            ("key_name", preview.key_name.as_str()),
            ("detail", preview.digest.as_str()),
        ],
        None,
    )
}

pub(crate) fn residue_report(
    locale: Locale,
    report: &SoftwareRegistryResidueReportV1,
) -> Result<String, CatalogueError> {
    render(
        locale,
        "software.cli.leftovers.residue",
        &[
            ("key_name", report.key_name.as_str()),
            ("detail", report.backup_sha256.as_str()),
        ],
        None,
    )
}

pub(crate) fn residue_restored(locale: Locale) -> Result<String, CatalogueError> {
    static_text(locale, "software.cli.leftovers.residue.restored")
}

pub(crate) fn startup_execute(
    locale: Locale,
    report: &StartupToggleStrictReportV1,
) -> Result<String, CatalogueError> {
    let key = match report.outcome {
        StartupStrictOutcome::Succeeded => "software.cli.startup.execute.succeeded",
        StartupStrictOutcome::Failed => "software.cli.startup.execute.failed",
        StartupStrictOutcome::Unknown => "software.cli.startup.execute.unknown",
        StartupStrictOutcome::AlreadyInState => "software.cli.startup.execute.already_in_state",
    };
    with_code(locale, key, report.error_code.as_deref())
}

pub(crate) fn leftovers_scan(
    locale: Locale,
    preview: &SoftwareLeftoverPreviewV1,
) -> Result<String, CatalogueError> {
    let count = preview.apps.len().to_string();
    let mut lines = vec![
        render(
            locale,
            "software.cli.leftovers.summary",
            &[("count", &count)],
            None,
        )?,
        render(locale, "software.cli.leftovers.untrusted", &[], None)?,
    ];
    for app in &preview.apps {
        for candidate in &app.candidates {
            lines.push(leftover_candidate(locale, candidate)?);
        }
    }
    Ok(lines.join("\n"))
}

pub(crate) fn leftovers_preview(
    locale: Locale,
    preview: &SoftwareLeftoverStrictPreviewV1,
) -> Result<String, CatalogueError> {
    let count = preview.items.len().to_string();
    let mut lines = vec![
        render(locale, "software.cli.leftovers.untrusted", &[], None)?,
        render(
            locale,
            "software.cli.leftovers.preview",
            &[("digest", preview.digest.as_str()), ("count", &count)],
            None,
        )?,
    ];
    for candidate in &preview.items {
        lines.push(leftover_candidate(locale, candidate)?);
    }
    Ok(lines.join("\n"))
}

pub(crate) fn leftovers_execute(
    locale: Locale,
    report: &SoftwareLeftoverStrictReportV1,
) -> Result<String, CatalogueError> {
    let key = match report.outcome {
        SoftwareLeftoverStrictOutcome::Succeeded => "software.cli.leftovers.execute.succeeded",
        SoftwareLeftoverStrictOutcome::Failed => "software.cli.leftovers.execute.failed",
        SoftwareLeftoverStrictOutcome::Unknown => "software.cli.leftovers.execute.unknown",
        SoftwareLeftoverStrictOutcome::Refused => "software.cli.leftovers.execute.refused",
    };
    with_code(locale, key, report.error_code.as_deref())
}

fn leftover_candidate(
    locale: Locale,
    candidate: &devsweep_core::software::SoftwareLeftoverCandidateV1,
) -> Result<String, CatalogueError> {
    let certainty = static_text(locale, leftover_certainty_key(candidate.certainty))?;
    let origin = static_text(locale, leftover_origin_key(candidate.origin))?;
    render(
        locale,
        "software.cli.leftovers.candidate",
        &[
            ("id", candidate.id.as_str()),
            ("certainty", &certainty),
            ("origin", &origin),
        ],
        None,
    )
}

fn with_code(locale: Locale, key: &str, code: Option<&str>) -> Result<String, CatalogueError> {
    let mut lines = vec![render(locale, key, &[], None)?];
    if let Some(code) = code {
        lines.push(render(
            locale,
            "software.cli.result.code",
            &[("code", code)],
            None,
        )?);
    }
    Ok(lines.join("\n"))
}

fn updates_reason_key(reason: SoftwareUpdatesReason) -> &'static str {
    match reason {
        SoftwareUpdatesReason::WingetMissing => "software.v1.updates.reason.winget_missing",
        SoftwareUpdatesReason::SourceAgreementPending => {
            "software.v1.updates.reason.source_agreement_pending"
        }
        SoftwareUpdatesReason::NonZeroExit => "software.v1.updates.reason.non_zero_exit",
        SoftwareUpdatesReason::TimedOut => "software.v1.updates.reason.timed_out",
        SoftwareUpdatesReason::Canceled => "software.v1.updates.reason.canceled",
        SoftwareUpdatesReason::OutputTruncated => "software.v1.updates.reason.output_truncated",
        SoftwareUpdatesReason::UnrecognizedOutput => {
            "software.v1.updates.reason.unrecognized_output"
        }
        SoftwareUpdatesReason::ProcessFailed => "software.v1.updates.reason.process_failed",
    }
}

fn startup_location_key(location: SoftwareStartupLocation) -> &'static str {
    match location {
        SoftwareStartupLocation::CurrentUserRun => "software.v1.startup.location.current_user_run",
        SoftwareStartupLocation::MachineRun => "software.v1.startup.location.machine_run",
        SoftwareStartupLocation::MachineRun32 => "software.v1.startup.location.machine_run32",
        SoftwareStartupLocation::CurrentUserStartupFolder => {
            "software.v1.startup.location.current_user_startup_folder"
        }
    }
}

fn startup_state_key(state: SoftwareStartupState) -> &'static str {
    match state {
        SoftwareStartupState::Enabled => "software.v1.startup.state.enabled",
        SoftwareStartupState::Disabled => "software.v1.startup.state.disabled",
        SoftwareStartupState::Unknown => "software.v1.startup.state.unknown",
    }
}

fn leftover_certainty_key(certainty: SoftwareLeftoverCertainty) -> &'static str {
    match certainty {
        SoftwareLeftoverCertainty::Certain => "software.v1.leftovers.certainty.certain",
        SoftwareLeftoverCertainty::Uncertain => "software.v1.leftovers.certainty.uncertain",
    }
}

fn leftover_origin_key(origin: SoftwareLeftoverOrigin) -> &'static str {
    match origin {
        SoftwareLeftoverOrigin::InstallLocation => "software.v1.leftovers.origin.install_location",
        SoftwareLeftoverOrigin::RoamingAppData => "software.v1.leftovers.origin.roaming_app_data",
        SoftwareLeftoverOrigin::LocalAppData => "software.v1.leftovers.origin.local_app_data",
        SoftwareLeftoverOrigin::ProgramData => "software.v1.leftovers.origin.program_data",
        SoftwareLeftoverOrigin::PersonalData => "software.v1.leftovers.origin.personal_data",
    }
}

fn outcome_key(outcome: SoftwareExecutionOutcome) -> &'static str {
    match outcome {
        SoftwareExecutionOutcome::CanceledBeforeStart => {
            "software.v1.outcome.canceled_before_start"
        }
        SoftwareExecutionOutcome::Removed => "software.v1.outcome.removed",
        SoftwareExecutionOutcome::RebootRequired => "software.v1.outcome.reboot_required",
        SoftwareExecutionOutcome::StillPresent => "software.v1.outcome.still_present",
        SoftwareExecutionOutcome::Failed => "software.v1.outcome.failed",
        SoftwareExecutionOutcome::UnknownAfterDispatch => {
            "software.v1.outcome.unknown_after_dispatch"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn software_inventory_summary_is_bilingual() {
        let inventory = SoftwareInventoryV1 {
            version: 1,
            observed_at_unix_ms: 0,
            sources: Vec::new(),
            entries: Vec::new(),
            fingerprint: "sha256:fixture".to_string(),
        };
        let english = super::inventory(Locale::En, &inventory).unwrap();
        let chinese = super::inventory(Locale::ZhCn, &inventory).unwrap();
        assert!(english.contains("Software inventory"));
        assert!(english.contains("0 entries"));
        assert!(chinese.contains("软件清单"));
        assert!(chinese.contains("0 个条目"));
    }
}
