//! Exhaustive Optimize V1 catalogue: exactly eight closed maintenance ids.
//!
//! The catalogue is compile-time fact. Unknown, rejected, or hostile ids can
//! never resolve because resolution is a closed lookup against this registry,
//! and guidance rows have no dispatch representation anywhere in the crate.

use serde::{Deserialize, Serialize};

/// The compile-time catalogue generation bound by every plan and audit record.
pub const OPTIMIZE_CATALOGUE_VERSION: u32 = 1;

/// Stable identifier of the bounded DNS resolver cache refresh.
pub const DNS_FLUSH_ID: &str = "dns.flush";
/// Stable identifier of the storage-recommendations Settings handoff.
pub const SETTINGS_STORAGE_RECOMMENDATIONS_ID: &str = "settings.storage_recommendations";
/// Stable identifier of the search-indexing Settings handoff.
pub const SETTINGS_SEARCH_ID: &str = "settings.search";
/// Stable identifier of the energy-recommendations Settings handoff.
pub const SETTINGS_ENERGY_RECOMMENDATIONS_ID: &str = "settings.energy_recommendations";
/// Stable identifier of the read-only drive-optimization guidance entry.
pub const GUIDANCE_DRIVE_OPTIMIZE_ID: &str = "guidance.drive_optimize";
/// Stable identifier of the read-only system-integrity guidance entry.
pub const GUIDANCE_SYSTEM_INTEGRITY_ID: &str = "guidance.system_integrity";
/// Stable identifier of the read-only filesystem-check guidance entry.
pub const GUIDANCE_FILESYSTEM_CHECK_ID: &str = "guidance.filesystem_check";
/// Stable identifier of the read-only network-reset guidance entry.
pub const GUIDANCE_NETWORK_RESET_ID: &str = "guidance.network_reset";

/// The only three Settings URIs Optimize V1 can ever hand off. They are
/// literal catalogue facts and are never accepted from callers.
pub(super) const SETTINGS_STORAGE_RECOMMENDATIONS_URI: &str = "ms-settings:storagerecommendations";
pub(super) const SETTINGS_SEARCH_URI: &str = "ms-settings:search";
pub(super) const SETTINGS_ENERGY_RECOMMENDATIONS_URI: &str = "ms-settings:energyrecommendations";

/// Conservative Windows 11 project floors for the two review pages.
pub(super) const SETTINGS_REVIEW_BUILD_FLOOR: u32 = 22_000;
/// Microsoft-documented minimum build for energy recommendations.
pub(super) const SETTINGS_ENERGY_BUILD_FLOOR: u32 = 22_624;

/// Closed Optimize V1 action classes. [`MaintenanceActionClass::Guidance`]
/// entries never receive a dispatch variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceActionClass {
    /// One bounded native command execution.
    Execute,
    /// One allowlisted Windows Settings handoff that reports only `launched`.
    SettingsHandoff,
    /// Read-only guidance that is only ever displayed.
    Guidance,
}

/// One immutable catalogue row. The catalogue itself is not deserializable;
/// rows exist only as compile-time output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaintenanceCatalogueEntryV1 {
    /// Stable catalogue identity.
    pub id: &'static str,
    /// Closed action class of the row.
    pub action_class: MaintenanceActionClass,
    /// Minimum manifest-independent OS build, when the row has a floor.
    pub build_floor: Option<u32>,
}

/// The exhaustive V1 registry in frozen catalogue order.
static ENTRIES: [MaintenanceCatalogueEntryV1; 8] = [
    MaintenanceCatalogueEntryV1 {
        id: DNS_FLUSH_ID,
        action_class: MaintenanceActionClass::Execute,
        build_floor: None,
    },
    MaintenanceCatalogueEntryV1 {
        id: SETTINGS_STORAGE_RECOMMENDATIONS_ID,
        action_class: MaintenanceActionClass::SettingsHandoff,
        build_floor: Some(SETTINGS_REVIEW_BUILD_FLOOR),
    },
    MaintenanceCatalogueEntryV1 {
        id: SETTINGS_SEARCH_ID,
        action_class: MaintenanceActionClass::SettingsHandoff,
        build_floor: Some(SETTINGS_REVIEW_BUILD_FLOOR),
    },
    MaintenanceCatalogueEntryV1 {
        id: SETTINGS_ENERGY_RECOMMENDATIONS_ID,
        action_class: MaintenanceActionClass::SettingsHandoff,
        build_floor: Some(SETTINGS_ENERGY_BUILD_FLOOR),
    },
    MaintenanceCatalogueEntryV1 {
        id: GUIDANCE_DRIVE_OPTIMIZE_ID,
        action_class: MaintenanceActionClass::Guidance,
        build_floor: None,
    },
    MaintenanceCatalogueEntryV1 {
        id: GUIDANCE_SYSTEM_INTEGRITY_ID,
        action_class: MaintenanceActionClass::Guidance,
        build_floor: None,
    },
    MaintenanceCatalogueEntryV1 {
        id: GUIDANCE_FILESYSTEM_CHECK_ID,
        action_class: MaintenanceActionClass::Guidance,
        build_floor: None,
    },
    MaintenanceCatalogueEntryV1 {
        id: GUIDANCE_NETWORK_RESET_ID,
        action_class: MaintenanceActionClass::Guidance,
        build_floor: None,
    },
];

/// Returns the exhaustive catalogue in frozen order.
#[must_use]
pub fn catalogue_entries() -> &'static [MaintenanceCatalogueEntryV1] {
    &ENTRIES
}

/// Resolves one catalogue row, or `None` for anything outside the registry.
#[must_use]
pub fn catalogue_entry(id: &str) -> Option<&'static MaintenanceCatalogueEntryV1> {
    ENTRIES.iter().find(|entry| entry.id == id)
}

/// Maps the exactly three Settings ids to their literal URI and build floor.
/// Every other id, including execute and guidance rows, has no handoff.
pub(super) fn settings_handoff(id: &str) -> Option<(&'static str, u32)> {
    match id {
        SETTINGS_STORAGE_RECOMMENDATIONS_ID => Some((
            SETTINGS_STORAGE_RECOMMENDATIONS_URI,
            SETTINGS_REVIEW_BUILD_FLOOR,
        )),
        SETTINGS_SEARCH_ID => Some((SETTINGS_SEARCH_URI, SETTINGS_REVIEW_BUILD_FLOOR)),
        SETTINGS_ENERGY_RECOMMENDATIONS_ID => Some((
            SETTINGS_ENERGY_RECOMMENDATIONS_URI,
            SETTINGS_ENERGY_BUILD_FLOOR,
        )),
        _ => None,
    }
}
