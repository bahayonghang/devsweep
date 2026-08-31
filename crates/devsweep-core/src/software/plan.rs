//! Untrusted Software V1 selection, live revalidation, and opaque preview digest.

use std::{collections::BTreeSet, fmt};

use sha2::{Digest, Sha256};

use super::model::{
    SOFTWARE_INVENTORY_VERSION, SOFTWARE_PLAN_VERSION, SOFTWARE_PREVIEW_VERSION,
    SoftwareActionClass, SoftwareEligibilityReason, SoftwareIdentity, SoftwareInventoryV1,
    SoftwarePreviewItemV1, SoftwarePreviewV1, SoftwareScope, SoftwareSelectionPlanV1,
};

/// A saved Software inventory remains selection authority for fifteen minutes.
pub const SOFTWARE_INVENTORY_TTL_MS: u64 = 15 * 60 * 1_000;

/// Caller-correctable Software plan failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoftwarePlanError {
    UnsupportedInventoryVersion(u32),
    UnsupportedPlanVersion(u32),
    InvalidFingerprint,
    InventoryFingerprintMismatch,
    InventoryExpired,
    EmptySelection,
    DuplicateSelection(String),
    UnknownSelection(String),
    ManualSelection(String),
    StaleSelection(String),
    InvalidSelectedIdentity(String),
    DigestMismatch,
    SerializationFailed,
}

impl fmt::Display for SoftwarePlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedInventoryVersion(version) => {
                write!(
                    formatter,
                    "unsupported software inventory version {version}"
                )
            }
            Self::UnsupportedPlanVersion(version) => {
                write!(formatter, "unsupported software plan version {version}")
            }
            Self::InvalidFingerprint => {
                formatter.write_str("invalid software inventory fingerprint")
            }
            Self::InventoryFingerprintMismatch => {
                formatter.write_str("software inventory fingerprint mismatch")
            }
            Self::InventoryExpired => formatter.write_str("software inventory expired"),
            Self::EmptySelection => formatter.write_str("software plan selection is empty"),
            Self::DuplicateSelection(id) => write!(formatter, "duplicate software selection {id}"),
            Self::UnknownSelection(id) => write!(formatter, "unknown software selection {id}"),
            Self::ManualSelection(id) => write!(formatter, "manual software entry selected {id}"),
            Self::StaleSelection(id) => write!(formatter, "stale software selection {id}"),
            Self::InvalidSelectedIdentity(id) => {
                write!(
                    formatter,
                    "selected identity is not current-user MSIX: {id}"
                )
            }
            Self::DigestMismatch => formatter.write_str("software preview digest mismatch"),
            Self::SerializationFailed => {
                formatter.write_str("software canonical serialization failed")
            }
        }
    }
}

impl std::error::Error for SoftwarePlanError {}

/// Builds an untrusted exact-id plan from a saved immutable inventory.
pub fn build_selection_plan(
    inventory: &SoftwareInventoryV1,
    selected_ids: &[String],
    now_unix_ms: u64,
) -> Result<SoftwareSelectionPlanV1, SoftwarePlanError> {
    validate_inventory_header(inventory)?;
    validate_selections(inventory, selected_ids)?;
    let expires_at_unix_ms = inventory
        .observed_at_unix_ms
        .checked_add(SOFTWARE_INVENTORY_TTL_MS)
        .ok_or(SoftwarePlanError::InventoryExpired)?;
    if now_unix_ms > expires_at_unix_ms {
        return Err(SoftwarePlanError::InventoryExpired);
    }
    Ok(SoftwareSelectionPlanV1 {
        version: SOFTWARE_PLAN_VERSION,
        inventory_fingerprint: inventory.fingerprint.clone(),
        inventory_observed_at_unix_ms: inventory.observed_at_unix_ms,
        expires_at_unix_ms,
        selected_ids: selected_ids.to_vec(),
    })
}

/// Validates a saved plan against its saved inventory, then exact live identities.
pub fn preview_selection_plan(
    plan: &SoftwareSelectionPlanV1,
    saved_inventory: &SoftwareInventoryV1,
    live_inventory: &SoftwareInventoryV1,
    now_unix_ms: u64,
) -> Result<SoftwarePreviewV1, SoftwarePlanError> {
    validate_saved_plan(plan, saved_inventory, now_unix_ms)?;
    validate_inventory_header(live_inventory)?;
    let mut selected = Vec::with_capacity(plan.selected_ids.len());
    for id in &plan.selected_ids {
        let saved = saved_inventory
            .entries
            .iter()
            .find(|entry| &entry.id == id)
            .ok_or_else(|| SoftwarePlanError::UnknownSelection(id.clone()))?;
        let live = live_inventory
            .entries
            .iter()
            .find(|entry| &entry.id == id)
            .ok_or_else(|| SoftwarePlanError::StaleSelection(id.clone()))?;
        if live.identity != saved.identity
            || live.version != saved.version
            || !live.eligibility.is_selectable()
            || live.eligibility.reason != SoftwareEligibilityReason::EligibleCurrentUserMsix
        {
            return Err(SoftwarePlanError::StaleSelection(id.clone()));
        }
        let SoftwareIdentity::Msix { .. } = &live.identity else {
            return Err(SoftwarePlanError::InvalidSelectedIdentity(id.clone()));
        };
        if live.scope != SoftwareScope::CurrentUser {
            return Err(SoftwarePlanError::InvalidSelectedIdentity(id.clone()));
        }
        selected.push(preview_item(live)?);
    }
    selected.sort_by(|left, right| left.id.cmp(&right.id));
    let digest = preview_digest(&plan.inventory_fingerprint, &selected)?;
    Ok(SoftwarePreviewV1 {
        version: SOFTWARE_PREVIEW_VERSION,
        inventory_fingerprint: plan.inventory_fingerprint.clone(),
        irreversible: true,
        selected,
        digest,
    })
}

/// Revalidates an untrusted saved plan directly against a fresh exact-identity
/// inventory. This is the frozen CLI boundary: the plan contributes only its
/// opaque selection ids, expiry, and prior fingerprint; all strategy facts are
/// reconstructed from the live current-user MSIX inventory.
pub fn preview_selection_plan_live(
    plan: &SoftwareSelectionPlanV1,
    live_inventory: &SoftwareInventoryV1,
    now_unix_ms: u64,
) -> Result<SoftwarePreviewV1, SoftwarePlanError> {
    validate_live_plan_header(plan, now_unix_ms)?;
    validate_inventory_header(live_inventory)?;

    let mut selected = Vec::with_capacity(plan.selected_ids.len());
    let mut seen = BTreeSet::new();
    for id in &plan.selected_ids {
        if !seen.insert(id) {
            return Err(SoftwarePlanError::DuplicateSelection(id.clone()));
        }
        let live = live_inventory
            .entries
            .iter()
            .find(|entry| &entry.id == id)
            .ok_or_else(|| SoftwarePlanError::StaleSelection(id.clone()))?;
        selected.push(preview_item(live)?);
    }
    selected.sort_by(|left, right| left.id.cmp(&right.id));
    let digest = preview_digest(&plan.inventory_fingerprint, &selected)?;
    Ok(SoftwarePreviewV1 {
        version: SOFTWARE_PREVIEW_VERSION,
        inventory_fingerprint: plan.inventory_fingerprint.clone(),
        irreversible: true,
        selected,
        digest,
    })
}

/// Recomputes and checks an expected preview digest before any future dispatch.
pub fn validate_preview_digest(
    preview: &SoftwarePreviewV1,
    expected_digest: &str,
) -> Result<(), SoftwarePlanError> {
    if preview.version != SOFTWARE_PREVIEW_VERSION {
        return Err(SoftwarePlanError::DigestMismatch);
    }
    let actual = preview_digest(&preview.inventory_fingerprint, &preview.selected)?;
    if preview.digest != actual || expected_digest != actual {
        return Err(SoftwarePlanError::DigestMismatch);
    }
    Ok(())
}

fn validate_saved_plan(
    plan: &SoftwareSelectionPlanV1,
    inventory: &SoftwareInventoryV1,
    now_unix_ms: u64,
) -> Result<(), SoftwarePlanError> {
    if plan.version != SOFTWARE_PLAN_VERSION {
        return Err(SoftwarePlanError::UnsupportedPlanVersion(plan.version));
    }
    validate_inventory_header(inventory)?;
    if plan.inventory_fingerprint != inventory.fingerprint
        || plan.inventory_observed_at_unix_ms != inventory.observed_at_unix_ms
    {
        return Err(SoftwarePlanError::InventoryFingerprintMismatch);
    }
    if now_unix_ms > plan.expires_at_unix_ms {
        return Err(SoftwarePlanError::InventoryExpired);
    }
    validate_selections(inventory, &plan.selected_ids)
}

fn validate_live_plan_header(
    plan: &SoftwareSelectionPlanV1,
    now_unix_ms: u64,
) -> Result<(), SoftwarePlanError> {
    if plan.version != SOFTWARE_PLAN_VERSION {
        return Err(SoftwarePlanError::UnsupportedPlanVersion(plan.version));
    }
    if !is_sha256(&plan.inventory_fingerprint) {
        return Err(SoftwarePlanError::InvalidFingerprint);
    }
    if now_unix_ms > plan.expires_at_unix_ms
        || plan.expires_at_unix_ms
            != plan
                .inventory_observed_at_unix_ms
                .checked_add(SOFTWARE_INVENTORY_TTL_MS)
                .ok_or(SoftwarePlanError::InventoryExpired)?
    {
        return Err(SoftwarePlanError::InventoryExpired);
    }
    if plan.selected_ids.is_empty() {
        return Err(SoftwarePlanError::EmptySelection);
    }
    Ok(())
}

fn preview_item(
    live: &super::model::SoftwareEntryV1,
) -> Result<SoftwarePreviewItemV1, SoftwarePlanError> {
    if !live.eligibility.is_selectable()
        || live.eligibility.reason != SoftwareEligibilityReason::EligibleCurrentUserMsix
        || live.scope != SoftwareScope::CurrentUser
    {
        return Err(SoftwarePlanError::ManualSelection(live.id.clone()));
    }
    let SoftwareIdentity::Msix { package_full_name } = &live.identity else {
        return Err(SoftwarePlanError::InvalidSelectedIdentity(live.id.clone()));
    };
    if !valid_package_full_name(package_full_name) {
        return Err(SoftwarePlanError::InvalidSelectedIdentity(live.id.clone()));
    }
    Ok(SoftwarePreviewItemV1 {
        id: live.id.clone(),
        identity: live.identity.clone(),
        action_class: SoftwareActionClass::RemoveCurrentUserMsix,
        scope: SoftwareScope::CurrentUser,
        eligibility: SoftwareEligibilityReason::EligibleCurrentUserMsix,
        strategy_token: strategy_token(&live.identity, live.version.as_deref())?,
    })
}

pub(crate) fn valid_package_full_name(value: &str) -> bool {
    if value.is_empty()
        || value.len() > 8 * 1024
        || !value.is_ascii()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
    {
        return false;
    }
    let parts = value.split('_').collect::<Vec<_>>();
    if parts.len() != 5
        || parts[0].is_empty()
        || parts[1].is_empty()
        || parts[2].is_empty()
        || parts[4].is_empty()
    {
        return false;
    }
    let version = parts[1].split('.').collect::<Vec<_>>();
    version.len() == 4
        && version
            .iter()
            .all(|part| !part.is_empty() && part.parse::<u16>().is_ok())
        && matches!(parts[2], "x86" | "x64" | "arm" | "arm64" | "neutral")
        && parts[0]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
        && parts[3]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
        && parts[4].bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn validate_inventory_header(inventory: &SoftwareInventoryV1) -> Result<(), SoftwarePlanError> {
    if inventory.version != SOFTWARE_INVENTORY_VERSION {
        return Err(SoftwarePlanError::UnsupportedInventoryVersion(
            inventory.version,
        ));
    }
    if !is_sha256(&inventory.fingerprint) {
        return Err(SoftwarePlanError::InvalidFingerprint);
    }
    let actual = super::inventory_fingerprint(inventory)
        .map_err(|_| SoftwarePlanError::SerializationFailed)?;
    if inventory.fingerprint != actual {
        return Err(SoftwarePlanError::InventoryFingerprintMismatch);
    }
    Ok(())
}

fn validate_selections(
    inventory: &SoftwareInventoryV1,
    selected_ids: &[String],
) -> Result<(), SoftwarePlanError> {
    if selected_ids.is_empty() {
        return Err(SoftwarePlanError::EmptySelection);
    }
    let mut seen = BTreeSet::new();
    for id in selected_ids {
        if !seen.insert(id) {
            return Err(SoftwarePlanError::DuplicateSelection(id.clone()));
        }
        let entry = inventory
            .entries
            .iter()
            .find(|entry| &entry.id == id)
            .ok_or_else(|| SoftwarePlanError::UnknownSelection(id.clone()))?;
        if !entry.eligibility.is_selectable() {
            return Err(SoftwarePlanError::ManualSelection(id.clone()));
        }
    }
    Ok(())
}

fn strategy_token(
    identity: &SoftwareIdentity,
    version: Option<&str>,
) -> Result<String, SoftwarePlanError> {
    let canonical = serde_json::to_vec(&("devsweep.software.strategy.v1", identity, version))
        .map_err(|_| SoftwarePlanError::SerializationFailed)?;
    Ok(format!("software-token:sha256:{}", hex_sha256(&canonical)))
}

fn preview_digest(
    inventory_fingerprint: &str,
    selected: &[SoftwarePreviewItemV1],
) -> Result<String, SoftwarePlanError> {
    let canonical = serde_json::to_vec(&(
        "devsweep.software.preview.v1",
        inventory_fingerprint,
        selected,
    ))
    .map_err(|_| SoftwarePlanError::SerializationFailed)?;
    Ok(format!("sha256:{}", hex_sha256(&canonical)))
}

fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::software::model::{
        SoftwareEligibility, SoftwareEligibilityState, SoftwareEntryV1, SoftwareLastUsedEvidence,
        SoftwareSizeEvidence,
    };

    fn inventory(full_name: &str, observed_at: u64) -> SoftwareInventoryV1 {
        let identity = SoftwareIdentity::Msix {
            package_full_name: full_name.to_string(),
        };
        let id = super::super::software_id(&identity).unwrap();
        let mut inventory = SoftwareInventoryV1 {
            version: SOFTWARE_INVENTORY_VERSION,
            observed_at_unix_ms: observed_at,
            sources: Vec::new(),
            entries: vec![SoftwareEntryV1 {
                id,
                identity,
                scope: SoftwareScope::CurrentUser,
                display_name: Some("Example".to_string()),
                publisher: None,
                version: Some("1.0.0.0".to_string()),
                provenance: Vec::new(),
                eligibility: SoftwareEligibility {
                    state: SoftwareEligibilityState::Selectable,
                    reason: SoftwareEligibilityReason::EligibleCurrentUserMsix,
                },
                size: SoftwareSizeEvidence::Unknown {
                    reason_code: "not_measured".to_string(),
                },
                last_used: SoftwareLastUsedEvidence::default(),
            }],
            fingerprint: String::new(),
        };
        inventory.fingerprint = super::super::inventory_fingerprint(&inventory).unwrap();
        inventory
    }

    #[test]
    fn removed_changed_reinstalled_and_expired_plans_fail_closed() {
        let saved = inventory("Example_1.0.0.0_x64__publisher", 1_000);
        let plan = build_selection_plan(&saved, &[saved.entries[0].id.clone()], 1_001).unwrap();

        let mut removed = SoftwareInventoryV1 {
            entries: Vec::new(),
            ..saved.clone()
        };
        removed.fingerprint = super::super::inventory_fingerprint(&removed).unwrap();
        assert!(matches!(
            preview_selection_plan(&plan, &saved, &removed, 1_002),
            Err(SoftwarePlanError::StaleSelection(_))
        ));

        let reinstalled = inventory("Example_2.0.0.0_x64__publisher", 1_002);
        assert!(matches!(
            preview_selection_plan(&plan, &saved, &reinstalled, 1_002),
            Err(SoftwarePlanError::StaleSelection(_))
        ));

        let mut changed = saved.clone();
        changed.entries[0].version = Some("changed".to_string());
        changed.fingerprint = super::super::inventory_fingerprint(&changed).unwrap();
        assert!(matches!(
            preview_selection_plan(&plan, &saved, &changed, 1_002),
            Err(SoftwarePlanError::StaleSelection(_))
        ));

        assert_eq!(
            preview_selection_plan(&plan, &saved, &saved, plan.expires_at_unix_ms + 1),
            Err(SoftwarePlanError::InventoryExpired)
        );
    }

    #[test]
    fn preview_digest_is_stable_and_contains_only_opaque_strategy_tokens() {
        let saved = inventory("Example_1.0.0.0_x64__publisher", 1_000);
        let plan = build_selection_plan(&saved, &[saved.entries[0].id.clone()], 1_001).unwrap();
        let first = preview_selection_plan(&plan, &saved, &saved, 1_002).unwrap();
        let second = preview_selection_plan(&plan, &saved, &saved, 1_002).unwrap();
        assert_eq!(first, second);
        let json = serde_json::to_string(&first).unwrap();
        assert!(!json.contains("program"));
        assert!(!json.contains("argv"));
        assert!(!json.contains("installed_path"));
        assert!(json.contains("software-token:sha256:"));
        validate_preview_digest(&first, &first.digest).unwrap();
        assert_eq!(
            validate_preview_digest(
                &first,
                "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
            ),
            Err(SoftwarePlanError::DigestMismatch)
        );
    }

    #[test]
    fn hostile_display_observations_never_enter_the_selection_plan() {
        let mut saved = inventory("Example_1.0.0.0_x64__publisher", 1_000);
        let hostile = "\"cmd.exe /c calc\" & %TEMP% | ${env:USERPROFILE}";
        saved.entries[0].display_name = Some(hostile.to_string());
        saved.fingerprint = super::super::inventory_fingerprint(&saved).unwrap();
        let plan = build_selection_plan(&saved, &[saved.entries[0].id.clone()], 1_001).unwrap();
        let json = serde_json::to_string(&plan).unwrap();
        assert!(!json.contains(hostile));
        assert!(!json.contains("program"));
        assert!(!json.contains("argv"));
    }
}
