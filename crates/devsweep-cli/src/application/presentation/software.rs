//! Bilingual Software V1 human renderers.

use devsweep_core::software::{
    SoftwareActionOutcomeV1, SoftwareEligibilityReason, SoftwareEligibilityState,
    SoftwareExecutionOutcome, SoftwareExecutionReportV1, SoftwareIdentity, SoftwareInventoryV1,
    SoftwareLastUsedEvidence, SoftwarePreviewV1, SoftwareScope, SoftwareSizeBasis,
    SoftwareSizeEvidence, SoftwareSourceState,
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
