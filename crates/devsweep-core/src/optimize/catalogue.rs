//! Closed Optimize catalogue.
//!
//! [`catalogue_entries`] remains the original eight V1 rows, in the original
//! order, so existing callers keep that slice. [`published_entries`] appends
//! the V2 rows from the C06 catalogue table. Unknown ids cannot resolve.
//! Guidance and observe rows have no dispatch representation.

use serde::{Deserialize, Serialize};

/// Catalogue generation of the original eight rows.
pub const OPTIMIZE_CATALOGUE_VERSION: u32 = 1;
/// Catalogue generation of the C06 maintenance rows.
pub(super) const OPTIMIZE_CATALOGUE_V2: u32 = 2;

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
/// SSD retrim. One plan binds one volume.
pub(super) const MAINTENANCE_RETRIM_ID: &str = "maintenance.retrim";
/// Read-only clock offset probe. It does not resync.
pub(super) const MAINTENANCE_CLOCK_PROBE_ID: &str = "maintenance.clock_probe";
/// Clock resync. It does not probe.
pub(super) const MAINTENANCE_CLOCK_SYNC_ID: &str = "maintenance.clock_sync";
/// DISM image scan. It does not repair.
pub(super) const MAINTENANCE_IMAGE_SCAN_ID: &str = "maintenance.image_scan";
/// DISM image repair. It does not scan as a second operation.
pub(super) const MAINTENANCE_IMAGE_REPAIR_ID: &str = "maintenance.image_repair";
/// Delete startup entries whose targets are already gone, after a `.reg` backup.
pub(super) const MAINTENANCE_DEAD_STARTUP_ID: &str = "maintenance.dead_startup";
/// Read-only disk health. `optimize plan` refuses this id.
pub(super) const MAINTENANCE_DISK_HEALTH_ID: &str = "maintenance.disk_health";
/// Guidance only. Update-cache deletion would be permanent delete.
pub(super) const MAINTENANCE_UPDATE_CACHE_ID: &str = "maintenance.update_cache";
/// Guidance only. Icon-cache recovery would kill shell processes.
pub(super) const MAINTENANCE_ICON_CACHE_ID: &str = "maintenance.icon_cache";
/// Guidance only. Search repair has no recovered safe argv.
pub(super) const MAINTENANCE_SEARCH_ID: &str = "maintenance.search";
/// Guidance only. Winget repair has no recovered argv.
pub(super) const MAINTENANCE_WINGET_REPAIR_ID: &str = "maintenance.winget_repair";

/// The only three Settings URIs Optimize V1 can ever hand off. They are
/// literal catalogue facts and are never accepted from callers.
pub(super) const SETTINGS_STORAGE_RECOMMENDATIONS_URI: &str = "ms-settings:storagerecommendations";
pub(super) const SETTINGS_SEARCH_URI: &str = "ms-settings:search";
pub(super) const SETTINGS_ENERGY_RECOMMENDATIONS_URI: &str = "ms-settings:energyrecommendations";

/// Conservative Windows 11 project floors for the two review pages.
pub(super) const SETTINGS_REVIEW_BUILD_FLOOR: u32 = 22_000;
/// Microsoft-documented minimum build for energy recommendations.
pub(super) const SETTINGS_ENERGY_BUILD_FLOOR: u32 = 22_624;

/// Closed Optimize action classes. [`MaintenanceActionClass::Guidance`] and
/// [`MaintenanceActionClass::Observe`] never receive a dispatch variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceActionClass {
    /// One bounded native command execution.
    Execute,
    /// One allowlisted Windows Settings handoff that reports only `launched`.
    SettingsHandoff,
    /// Read-only guidance that is only ever displayed.
    Guidance,
    /// Read-only observation. `optimize plan` refuses this class.
    Observe,
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

const V1_LEN: usize = 8;

/// V1 rows first, then the C06 rows, in catalogue-table order.
static PUBLISHED: [MaintenanceCatalogueEntryV1; 19] = [
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
    MaintenanceCatalogueEntryV1 {
        id: MAINTENANCE_RETRIM_ID,
        action_class: MaintenanceActionClass::Execute,
        build_floor: None,
    },
    MaintenanceCatalogueEntryV1 {
        id: MAINTENANCE_CLOCK_PROBE_ID,
        action_class: MaintenanceActionClass::Execute,
        build_floor: None,
    },
    MaintenanceCatalogueEntryV1 {
        id: MAINTENANCE_CLOCK_SYNC_ID,
        action_class: MaintenanceActionClass::Execute,
        build_floor: None,
    },
    MaintenanceCatalogueEntryV1 {
        id: MAINTENANCE_IMAGE_SCAN_ID,
        action_class: MaintenanceActionClass::Execute,
        build_floor: None,
    },
    MaintenanceCatalogueEntryV1 {
        id: MAINTENANCE_IMAGE_REPAIR_ID,
        action_class: MaintenanceActionClass::Execute,
        build_floor: None,
    },
    MaintenanceCatalogueEntryV1 {
        id: MAINTENANCE_DEAD_STARTUP_ID,
        action_class: MaintenanceActionClass::Execute,
        build_floor: None,
    },
    MaintenanceCatalogueEntryV1 {
        id: MAINTENANCE_DISK_HEALTH_ID,
        action_class: MaintenanceActionClass::Observe,
        build_floor: None,
    },
    MaintenanceCatalogueEntryV1 {
        id: MAINTENANCE_UPDATE_CACHE_ID,
        action_class: MaintenanceActionClass::Guidance,
        build_floor: None,
    },
    MaintenanceCatalogueEntryV1 {
        id: MAINTENANCE_ICON_CACHE_ID,
        action_class: MaintenanceActionClass::Guidance,
        build_floor: None,
    },
    MaintenanceCatalogueEntryV1 {
        id: MAINTENANCE_SEARCH_ID,
        action_class: MaintenanceActionClass::Guidance,
        build_floor: None,
    },
    MaintenanceCatalogueEntryV1 {
        id: MAINTENANCE_WINGET_REPAIR_ID,
        action_class: MaintenanceActionClass::Guidance,
        build_floor: None,
    },
];

/// Returns the original eight V1 rows in frozen order.
#[must_use]
pub fn catalogue_entries() -> &'static [MaintenanceCatalogueEntryV1] {
    &PUBLISHED[..V1_LEN]
}

/// Returns every V1 and V2 row. V1 order is unchanged.
#[must_use]
pub fn published_entries() -> &'static [MaintenanceCatalogueEntryV1] {
    &PUBLISHED
}

/// Catalogue generation of one id. V1 ids stay generation 1.
#[must_use]
pub(super) fn catalogue_generation(id: &str) -> Option<u32> {
    match PUBLISHED.iter().position(|entry| entry.id == id) {
        Some(index) if index < V1_LEN => Some(OPTIMIZE_CATALOGUE_VERSION),
        Some(_) => Some(OPTIMIZE_CATALOGUE_V2),
        None => None,
    }
}

/// Generation stamped on a list of rows. A V1-only slice stays generation 1.
#[must_use]
pub fn published_catalogue_version(entries: &[MaintenanceCatalogueEntryV1]) -> u32 {
    if entries
        .iter()
        .any(|entry| catalogue_generation(entry.id) == Some(OPTIMIZE_CATALOGUE_V2))
    {
        OPTIMIZE_CATALOGUE_V2
    } else {
        OPTIMIZE_CATALOGUE_VERSION
    }
}

/// Resolves one catalogue row, or `None` for anything outside the registry.
#[must_use]
pub fn catalogue_entry(id: &str) -> Option<&'static MaintenanceCatalogueEntryV1> {
    PUBLISHED.iter().find(|entry| entry.id == id)
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
