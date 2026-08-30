//! Bilingual Software V1 inventory summary renderer.

use devsweep_core::software::{SoftwareEligibilityState, SoftwareInventoryV1, SoftwareSourceState};

use super::render;
use crate::i18n::{CatalogueError, Locale};

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
    Ok([
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
    ]
    .join("\n"))
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
