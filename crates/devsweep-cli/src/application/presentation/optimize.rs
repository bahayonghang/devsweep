//! Bilingual Optimize V1 human renderers.

use devsweep_core::optimize::{
    MaintenanceActionClass, MaintenanceCatalogueEntryV1, MaintenanceExecutionOutcome,
    MaintenanceExecutionReportV1, MaintenancePreviewV1, OPTIMIZE_CATALOGUE_VERSION,
};

use super::render;
use crate::i18n::CatalogueError;

#[cfg(test)]
pub(super) struct Route;

/// Renders the exhaustive closed catalogue. Guidance rows are display-only by
/// construction: this is the only surface they ever reach.
pub(crate) fn list(
    locale: crate::i18n::Locale,
    entries: &[MaintenanceCatalogueEntryV1],
) -> Result<String, CatalogueError> {
    let count = entries.len().to_string();
    let version = OPTIMIZE_CATALOGUE_VERSION.to_string();
    let mut lines = vec![render(
        locale,
        "optimize.v1.list.summary",
        &[("count", &count), ("version", &version)],
        None,
    )?];
    for entry in entries {
        let action = static_text(locale, action_key(entry.action_class))?;
        let predicate = match entry.action_class {
            MaintenanceActionClass::SettingsHandoff => {
                let floor = entry.build_floor.unwrap_or_default().to_string();
                render(
                    locale,
                    "optimize.v1.predicate.settings",
                    &[("floor", &floor)],
                    None,
                )?
            }
            MaintenanceActionClass::Execute => {
                static_text(locale, "optimize.v1.predicate.dns_flush")?
            }
            MaintenanceActionClass::Guidance => {
                static_text(locale, "optimize.v1.predicate.guidance")?
            }
        };
        lines.push(render(
            locale,
            "optimize.v1.list.entry",
            &[
                ("id", entry.id),
                ("action", &action),
                ("predicate", &predicate),
            ],
            None,
        )?);
    }
    Ok(lines.join("\n"))
}

pub(crate) fn preview(
    locale: crate::i18n::Locale,
    preview: &MaintenancePreviewV1,
) -> Result<String, CatalogueError> {
    let action = static_text(locale, action_key(preview.action_class))?;
    let mut lines = vec![render(
        locale,
        "optimize.v1.preview.summary",
        &[
            ("id", preview.operation_id.as_str()),
            ("action", &action),
            ("digest", preview.digest.as_str()),
        ],
        None,
    )?];
    lines.push(static_text(locale, "optimize.v1.preview.note")?);
    Ok(lines.join("\n"))
}

pub(crate) fn execution(
    locale: crate::i18n::Locale,
    report: &MaintenanceExecutionReportV1,
) -> Result<String, CatalogueError> {
    let count = |outcome: MaintenanceExecutionOutcome| {
        report
            .outcomes
            .iter()
            .filter(|item| item.outcome == outcome)
            .count()
            .to_string()
    };
    let succeeded = count(MaintenanceExecutionOutcome::Succeeded);
    let launched = count(MaintenanceExecutionOutcome::Launched);
    let failed = count(MaintenanceExecutionOutcome::Failed);
    let unknown = count(MaintenanceExecutionOutcome::UnknownAfterDispatch);
    let canceled = count(MaintenanceExecutionOutcome::CanceledBeforeStart);
    let mut lines = vec![render(
        locale,
        "optimize.v1.execution.summary",
        &[
            ("succeeded", &succeeded),
            ("launched", &launched),
            ("failed", &failed),
            ("unknown", &unknown),
            ("canceled", &canceled),
        ],
        None,
    )?];
    for item in &report.outcomes {
        let outcome = static_text(locale, outcome_key(item.outcome))?;
        lines.push(render(
            locale,
            "optimize.v1.execution.item",
            &[
                ("id", item.catalogue_id.as_str()),
                ("outcome", &outcome),
                ("operation", item.operation_id.as_str()),
            ],
            None,
        )?);
    }
    Ok(lines.join("\n"))
}

fn action_key(action_class: MaintenanceActionClass) -> &'static str {
    match action_class {
        MaintenanceActionClass::Execute => "optimize.v1.action.execute",
        MaintenanceActionClass::SettingsHandoff => "optimize.v1.action.settings",
        MaintenanceActionClass::Guidance => "optimize.v1.action.guidance",
    }
}

fn outcome_key(outcome: MaintenanceExecutionOutcome) -> &'static str {
    match outcome {
        MaintenanceExecutionOutcome::CanceledBeforeStart => {
            "optimize.v1.outcome.canceled_before_start"
        }
        MaintenanceExecutionOutcome::Succeeded => "optimize.v1.outcome.succeeded",
        MaintenanceExecutionOutcome::Launched => "optimize.v1.outcome.launched",
        MaintenanceExecutionOutcome::Failed => "optimize.v1.outcome.failed",
        MaintenanceExecutionOutcome::UnknownAfterDispatch => {
            "optimize.v1.outcome.unknown_after_dispatch"
        }
    }
}

fn static_text(locale: crate::i18n::Locale, key: &'static str) -> Result<String, CatalogueError> {
    render(locale, key, &[], None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Locale;
    use devsweep_core::optimize::{MaintenanceActionOutcomeV1, catalogue_entries};

    #[test]
    fn catalogue_is_bilingual_and_lists_all_eight_rows() {
        let english = list(Locale::En, catalogue_entries()).unwrap();
        let chinese = list(Locale::ZhCn, catalogue_entries()).unwrap();
        for id in [
            "dns.flush",
            "settings.storage_recommendations",
            "settings.search",
            "settings.energy_recommendations",
            "guidance.drive_optimize",
            "guidance.system_integrity",
            "guidance.filesystem_check",
            "guidance.network_reset",
        ] {
            assert!(english.contains(id), "english list omits {id}");
            assert!(chinese.contains(id), "chinese list omits {id}");
        }
        assert!(english.contains("catalogue version 1"));
        assert!(chinese.contains("目录版本 1"));
    }

    #[test]
    fn preview_and_execution_state_launched_is_not_completion() {
        let fixture = MaintenancePreviewV1 {
            version: 1,
            catalogue_version: 1,
            operation_id: "settings.search".to_string(),
            action_class: MaintenanceActionClass::SettingsHandoff,
            digest: format!("sha256:{}", "4".repeat(64)),
        };
        let english = preview(Locale::En, &fixture).unwrap();
        assert!(english.contains("never maintenance completion"));
        let chinese = preview(Locale::ZhCn, &fixture).unwrap();
        assert!(chinese.contains("不是已完成维护"));

        let report = MaintenanceExecutionReportV1 {
            version: 1,
            catalogue_version: 1,
            outcomes: vec![MaintenanceActionOutcomeV1 {
                operation_id: "op-1".to_string(),
                catalogue_id: "settings.search".to_string(),
                action_class: MaintenanceActionClass::SettingsHandoff,
                outcome: MaintenanceExecutionOutcome::Launched,
                error_code: None,
            }],
        };
        let english = execution(Locale::En, &report).unwrap();
        assert!(english.contains("is not maintenance completion"));
        assert!(english.contains("Launched"));
        let chinese = execution(Locale::ZhCn, &report).unwrap();
        assert!(chinese.contains("不是已完成维护"));
    }
}
