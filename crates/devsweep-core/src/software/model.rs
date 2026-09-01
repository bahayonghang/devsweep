//! Closed Software V1 observation and plan DTOs.

use serde::{Deserialize, Serialize};

/// Software inventory wire version.
pub const SOFTWARE_INVENTORY_VERSION: u32 = 1;
/// Software selection-plan wire version.
pub const SOFTWARE_PLAN_VERSION: u32 = 1;
/// Software preview wire version.
pub const SOFTWARE_PREVIEW_VERSION: u32 = 1;

/// One requested inventory source family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoftwareInventorySource {
    /// ARP, MSI, and current-user MSIX.
    All,
    /// Add/Remove Programs registry records.
    Arp,
    /// Windows Installer products.
    Msi,
    /// Current-user MSIX packages.
    Msix,
}

/// Registry hive participating in an exact ARP identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryHive {
    CurrentUser,
    LocalMachine,
}

/// Explicit Windows registry view participating in an exact ARP identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryView {
    Registry32,
    Registry64,
}

/// Windows Installer registration context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MsiContext {
    UserUnmanaged,
    UserManaged,
    Machine,
}

/// Exact source-local evidence identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum SoftwareSourceId {
    Arp {
        hive: RegistryHive,
        view: RegistryView,
    },
    Msi {
        context: MsiContext,
    },
    MsixCurrentUser,
}

/// Source result. One failed source never erases sibling observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareSourceState {
    Available,
    Partial,
    Permission,
    Unsupported,
}

/// Evidence for one exact inventory source.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareSourceEvidence {
    pub source: SoftwareSourceId,
    pub state: SoftwareSourceState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<String>,
}

impl SoftwareSourceEvidence {
    pub(crate) fn available(source: SoftwareSourceId) -> Self {
        Self {
            source,
            state: SoftwareSourceState::Available,
            reason_code: None,
        }
    }

    pub(crate) fn unavailable(
        source: SoftwareSourceId,
        state: SoftwareSourceState,
        reason_code: impl Into<String>,
    ) -> Self {
        Self {
            source,
            state,
            reason_code: Some(reason_code.into()),
        }
    }
}

/// Exact authoritative software identity. Display metadata never participates.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum SoftwareIdentity {
    Arp {
        hive: RegistryHive,
        view: RegistryView,
        subkey: String,
    },
    Msi {
        product_code: String,
        context: MsiContext,
    },
    Msix {
        package_full_name: String,
    },
}

/// Installation scope reported by an authoritative source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareScope {
    CurrentUser,
    Machine,
}

/// Exact first-match result of the ordered V1 eligibility table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareEligibilityReason {
    ProtectedProduct,
    SourceIncomplete,
    ConflictingIdentity,
    NoRemove,
    HiddenEntry,
    SystemOrUpdate,
    DependencyPackage,
    StubPackage,
    UnhealthyPackage,
    MsiExecutionNotSupportedV1,
    RegistryOnlyManual,
    UnsupportedSource,
    EligibleCurrentUserMsix,
}

/// Closed selectable/manual state; presentations never infer this value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareEligibilityState {
    Selectable,
    Manual,
}

/// Frozen eligibility decision stored in the inventory fingerprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareEligibility {
    pub state: SoftwareEligibilityState,
    pub reason: SoftwareEligibilityReason,
}

impl SoftwareEligibility {
    #[must_use]
    pub fn is_selectable(self) -> bool {
        self.state == SoftwareEligibilityState::Selectable
    }
}

/// Closed V1 size evidence. Paths never cross this boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum SoftwareSizeEvidence {
    Available {
        value_bytes: u64,
        basis: SoftwareSizeBasis,
        source_code: SoftwareSizeSourceCode,
        observed_at_unix_ms: u64,
    },
    Partial {
        lower_bound_bytes: u64,
        basis: SoftwareSizeBasis,
        source_code: SoftwareSizeSourceCode,
        reason_code: String,
        observed_at_unix_ms: u64,
    },
    Unknown {
        reason_code: String,
    },
}

/// Interpretation of a size value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareSizeBasis {
    ReportedEstimate,
    MeasuredInstalledLocation,
}

/// Exact provider for size evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareSizeSourceCode {
    ArpEstimatedSizeKib,
    MsiEstimatedSizeKib,
    MsixInstalledPath,
}

/// V1 deliberately has no available last-used variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum SoftwareLastUsedEvidence {
    Unknown { reason_code: SoftwareLastUsedReason },
}

impl Default for SoftwareLastUsedEvidence {
    fn default() -> Self {
        Self::Unknown {
            reason_code: SoftwareLastUsedReason::NoSupportedExactSource,
        }
    }
}

/// Only truthful V1 last-used reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareLastUsedReason {
    NoSupportedExactSource,
}

/// One exact inventory entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareEntryV1 {
    /// Opaque stable selection identity derived only from `identity`.
    pub id: String,
    pub identity: SoftwareIdentity,
    pub scope: SoftwareScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub provenance: Vec<SoftwareSourceId>,
    pub eligibility: SoftwareEligibility,
    pub size: SoftwareSizeEvidence,
    pub last_used: SoftwareLastUsedEvidence,
}

/// Immutable locale-neutral Software V1 inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareInventoryV1 {
    pub version: u32,
    pub observed_at_unix_ms: u64,
    pub sources: Vec<SoftwareSourceEvidence>,
    pub entries: Vec<SoftwareEntryV1>,
    pub fingerprint: String,
}

/// Untrusted persisted exact-id selection plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareSelectionPlanV1 {
    pub version: u32,
    pub inventory_fingerprint: String,
    pub inventory_observed_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub selected_ids: Vec<String>,
}

/// Locale-neutral serialized dry-run projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwarePreviewV1 {
    pub version: u32,
    pub inventory_fingerprint: String,
    /// Software removal is never reversible from DevSweep's perspective.
    pub irreversible: bool,
    pub selected: Vec<SoftwarePreviewItemV1>,
    pub digest: String,
}

/// One previewed current-user MSIX removal strategy. No program, argv, or path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwarePreviewItemV1 {
    pub id: String,
    pub identity: SoftwareIdentity,
    pub action_class: SoftwareActionClass,
    pub scope: SoftwareScope,
    pub eligibility: SoftwareEligibilityReason,
    pub strategy_token: String,
}

/// Closed Software V1 action class. Execution belongs to another task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareActionClass {
    RemoveCurrentUserMsix,
}

/// Internal source observation before deterministic assembly and fingerprinting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SoftwareObservation {
    pub identity: SoftwareIdentity,
    pub scope: SoftwareScope,
    pub display_name: Option<String>,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub provenance: Vec<SoftwareSourceId>,
    pub size: SoftwareSizeEvidence,
    pub flags: EligibilityFlags,
}

/// Internal facts consumed only by the canonical ordered refusal function.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct EligibilityFlags {
    pub protected: bool,
    pub source_incomplete: bool,
    pub conflicting_identity: bool,
    pub no_remove: bool,
    pub hidden: bool,
    pub system_or_update: bool,
    pub dependency: bool,
    pub stub: bool,
    pub unhealthy: bool,
    pub unsupported: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_used_wire_is_closed_to_truthful_unknown() {
        let value = serde_json::to_value(SoftwareLastUsedEvidence::default()).unwrap();
        assert_eq!(value["state"], "unknown");
        assert_eq!(value["reason_code"], "no_supported_exact_source");
        assert_eq!(value.as_object().unwrap().len(), 2);
    }

    #[test]
    fn size_wire_distinguishes_all_v1_states_in_integer_bytes() {
        let cases = [
            SoftwareSizeEvidence::Available {
                value_bytes: 2048,
                basis: SoftwareSizeBasis::ReportedEstimate,
                source_code: SoftwareSizeSourceCode::ArpEstimatedSizeKib,
                observed_at_unix_ms: 7,
            },
            SoftwareSizeEvidence::Partial {
                lower_bound_bytes: 1024,
                basis: SoftwareSizeBasis::MeasuredInstalledLocation,
                source_code: SoftwareSizeSourceCode::MsixInstalledPath,
                reason_code: "entry_budget_exhausted".to_string(),
                observed_at_unix_ms: 8,
            },
            SoftwareSizeEvidence::Unknown {
                reason_code: "not_reported".to_string(),
            },
        ];
        let json = serde_json::to_string(&cases).unwrap();
        assert!(json.contains("reported_estimate"));
        assert!(json.contains("measured_installed_location"));
        assert!(!json.contains("\"path\":"));
        assert!(!json.contains("InstallDate"));
        assert!(!json.contains("last_used_at"));
    }
}
