//! Read-only Software V1 inventory and preview planning.
//!
//! This domain never creates a [`crate::model::CleanupPlan`] and never reads or
//! executes vendor uninstall command fields.

use std::{collections::BTreeMap, sync::Arc, thread, time::SystemTime};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::process::FlagCancelObserver;

mod arp;
#[path = "execution/audit.rs"]
mod audit;
mod execution;
mod leftovers;
mod model;
mod msi;
mod msix;
mod plan;
mod startup;
mod updates;

pub use audit::SoftwareSupportAuditRecordV1;
pub use execution::{
    MSIX_CANCEL_GRACE, MSIX_MONITOR_TIMEOUT, MSIX_REQUERY_OFFSETS, SOFTWARE_AUDIT_VERSION,
    SOFTWARE_EXECUTION_VERSION, SoftwareActionOutcomeV1, SoftwareAuditError,
    SoftwareAuditErrorCode, SoftwareAuditRecordV1, SoftwareAuditRequeryResult,
    SoftwareAuditStatusCode, SoftwareAuditTransition, SoftwareExecutionError,
    SoftwareExecutionOutcome, SoftwareExecutionReportV1, SoftwareExecutionRequest,
    SoftwareExecutor, SoftwareInstalledState, SoftwareRebootEvidence, ValidatedSoftwareAction,
    software_audit_v1_path,
};
pub use leftovers::{
    SOFTWARE_LEFTOVER_VERSION, SoftwareLeftoverAppV1, SoftwareLeftoverCandidateV1,
    SoftwareLeftoverCertainty, SoftwareLeftoverError, SoftwareLeftoverExecutionRequest,
    SoftwareLeftoverOrigin, SoftwareLeftoverOutcomeV1, SoftwareLeftoverPlanPreviewV1,
    SoftwareLeftoverPlanV1, SoftwareLeftoverPreviewV1, SoftwareLeftoverReportV1,
    SoftwareLeftoverSelectionV1, discover_software_leftovers, execute_software_leftovers,
    plan_software_leftovers,
};
pub use model::{
    MsiContext, RegistryHive, RegistryView, SOFTWARE_INVENTORY_VERSION, SOFTWARE_PLAN_VERSION,
    SOFTWARE_PREVIEW_VERSION, SoftwareActionClass, SoftwareEligibility, SoftwareEligibilityReason,
    SoftwareEligibilityState, SoftwareEntryV1, SoftwareIdentity, SoftwareInventorySource,
    SoftwareInventoryV1, SoftwareLastUsedEvidence, SoftwareLastUsedReason, SoftwarePreviewItemV1,
    SoftwarePreviewV1, SoftwareScope, SoftwareSelectionPlanV1, SoftwareSizeBasis,
    SoftwareSizeEvidence, SoftwareSizeSourceCode, SoftwareSourceEvidence, SoftwareSourceId,
    SoftwareSourceState,
};
pub use plan::{
    SOFTWARE_INVENTORY_TTL_MS, SoftwarePlanError, build_selection_plan, preview_selection_plan,
    preview_selection_plan_live, validate_preview_digest,
};
pub use startup::{
    SOFTWARE_STARTUP_VERSION, SoftwareStartupEntryV1, SoftwareStartupError, SoftwareStartupListV1,
    SoftwareStartupLocation, SoftwareStartupSourceV1, SoftwareStartupState, SoftwareStartupToggle,
    SoftwareStartupToggleReportV1, SoftwareSupportErrorCode, SoftwareSupportOutcomeCode,
    list_startup_entries, set_startup_enabled,
};
pub use updates::{
    SOFTWARE_UPDATES_VERSION, SoftwareUpdateRowV1, SoftwareUpdatesReason, SoftwareUpdatesV1,
    check_software_updates,
};

use model::{EligibilityFlags, SoftwareObservation};

/// Inventories the requested Software sources without creating cleanup authority.
pub fn inventory_software(
    source: SoftwareInventorySource,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> Result<SoftwareInventoryV1> {
    let observed_at_unix_ms = unix_ms(SystemTime::now());
    let collected = match source {
        SoftwareInventorySource::All => {
            overlapped_all_sources(observed_at_unix_ms, cancel.cloned())
        }
        SoftwareInventorySource::Arp => arp::inventory(observed_at_unix_ms),
        SoftwareInventorySource::Msi => msi::inventory(observed_at_unix_ms),
        SoftwareInventorySource::Msix => msix::inventory(observed_at_unix_ms, cancel.cloned()),
    };
    assemble_inventory(
        observed_at_unix_ms,
        collected.evidence,
        collected.observations,
    )
}

fn overlapped_all_sources(
    observed_at_unix_ms: u64,
    cancel: Option<Arc<FlagCancelObserver>>,
) -> SourceInventory {
    let arp_job = spawn_source("devsweep-software-arp", move || {
        arp::inventory(observed_at_unix_ms)
    });
    let msi_job = spawn_source("devsweep-software-msi", move || {
        msi::inventory(observed_at_unix_ms)
    });
    let msix = msix::inventory(observed_at_unix_ms, cancel);
    let arp = finish_source(
        arp_job,
        || arp::inventory(observed_at_unix_ms),
        failed_arp_inventory,
    );
    let msi = finish_source(
        msi_job,
        || msi::inventory(observed_at_unix_ms),
        failed_msi_inventory,
    );
    let mut merged = arp;
    merged.evidence.extend(msi.evidence);
    merged.observations.extend(msi.observations);
    merged.evidence.extend(msix.evidence);
    merged.observations.extend(msix.observations);
    merged
}

fn spawn_source(
    name: &str,
    job: impl FnOnce() -> SourceInventory + Send + 'static,
) -> Option<thread::JoinHandle<SourceInventory>> {
    thread::Builder::new()
        .name(name.to_string())
        .spawn(job)
        .ok()
}

fn finish_source(
    job: Option<thread::JoinHandle<SourceInventory>>,
    fallback: impl FnOnce() -> SourceInventory,
    panicked: impl FnOnce() -> SourceInventory,
) -> SourceInventory {
    match job {
        Some(handle) => handle.join().unwrap_or_else(|_| panicked()),
        None => fallback(),
    }
}

fn failed_arp_inventory() -> SourceInventory {
    let mut evidence = Vec::new();
    for hive in [RegistryHive::CurrentUser, RegistryHive::LocalMachine] {
        for view in [RegistryView::Registry32, RegistryView::Registry64] {
            evidence.push(SoftwareSourceEvidence::unavailable(
                SoftwareSourceId::Arp { hive, view },
                SoftwareSourceState::Partial,
                "source_worker_panicked",
            ));
        }
    }
    SourceInventory {
        evidence,
        observations: Vec::new(),
    }
}

fn failed_msi_inventory() -> SourceInventory {
    SourceInventory {
        evidence: [
            MsiContext::UserUnmanaged,
            MsiContext::UserManaged,
            MsiContext::Machine,
        ]
        .into_iter()
        .map(|context| {
            SoftwareSourceEvidence::unavailable(
                SoftwareSourceId::Msi { context },
                SoftwareSourceState::Partial,
                "source_worker_panicked",
            )
        })
        .collect(),
        observations: Vec::new(),
    }
}

pub(crate) struct SourceInventory {
    evidence: Vec<SoftwareSourceEvidence>,
    observations: Vec<SoftwareObservation>,
}

impl SourceInventory {
    fn one(evidence: SoftwareSourceEvidence, observations: Vec<SoftwareObservation>) -> Self {
        Self {
            evidence: vec![evidence],
            observations,
        }
    }
}

fn assemble_inventory(
    observed_at_unix_ms: u64,
    mut sources: Vec<SoftwareSourceEvidence>,
    observations: Vec<SoftwareObservation>,
) -> Result<SoftwareInventoryV1> {
    sources.sort();
    sources.dedup();

    let mut exact = BTreeMap::<SoftwareIdentity, SoftwareObservation>::new();
    for mut observation in observations {
        observation.provenance.sort();
        observation.provenance.dedup();
        let authoritative_scope = scope_for_identity(&observation.identity);
        if observation.scope != authoritative_scope {
            observation.scope = authoritative_scope;
            observation.flags.conflicting_identity = true;
        }
        if let Some(existing) = exact.get_mut(&observation.identity) {
            if existing.display_name != observation.display_name {
                existing.display_name = None;
                existing.flags.conflicting_identity = true;
            }
            if existing.publisher != observation.publisher {
                existing.publisher = None;
                existing.flags.conflicting_identity = true;
            }
            if existing.version != observation.version {
                existing.version = None;
                existing.flags.conflicting_identity = true;
            }
            if existing.size != observation.size {
                existing.size = SoftwareSizeEvidence::Unknown {
                    reason_code: "conflicting_identity".to_string(),
                };
                existing.flags.conflicting_identity = true;
            }
            merge_flags(&mut existing.flags, observation.flags);
            existing.provenance.extend(observation.provenance);
            existing.provenance.sort();
            existing.provenance.dedup();
            continue;
        }
        exact.insert(observation.identity.clone(), observation);
    }

    let mut entries = Vec::with_capacity(exact.len());
    for (_, observation) in exact {
        let eligibility = ordered_eligibility(&observation.identity, observation.flags);
        entries.push(SoftwareEntryV1 {
            id: software_id(&observation.identity)?,
            identity: observation.identity,
            scope: observation.scope,
            display_name: observation.display_name,
            publisher: observation.publisher,
            version: observation.version,
            provenance: observation.provenance,
            eligibility,
            size: observation.size,
            last_used: SoftwareLastUsedEvidence::default(),
        });
    }
    entries.sort_by(|left, right| left.id.cmp(&right.id));

    let mut inventory = SoftwareInventoryV1 {
        version: SOFTWARE_INVENTORY_VERSION,
        observed_at_unix_ms,
        sources,
        entries,
        fingerprint: String::new(),
    };
    inventory.fingerprint = inventory_fingerprint(&inventory)?;
    Ok(inventory)
}

fn scope_for_identity(identity: &SoftwareIdentity) -> SoftwareScope {
    match identity {
        SoftwareIdentity::Arp { hive, .. } => {
            if *hive == RegistryHive::CurrentUser {
                SoftwareScope::CurrentUser
            } else {
                SoftwareScope::Machine
            }
        }
        SoftwareIdentity::Msi { context, .. } => {
            if *context == MsiContext::Machine {
                SoftwareScope::Machine
            } else {
                SoftwareScope::CurrentUser
            }
        }
        SoftwareIdentity::Msix { .. } => SoftwareScope::CurrentUser,
    }
}

fn merge_flags(existing: &mut EligibilityFlags, incoming: EligibilityFlags) {
    existing.protected |= incoming.protected;
    existing.source_incomplete |= incoming.source_incomplete;
    existing.conflicting_identity |= incoming.conflicting_identity;
    existing.no_remove |= incoming.no_remove;
    existing.hidden |= incoming.hidden;
    existing.system_or_update |= incoming.system_or_update;
    existing.dependency |= incoming.dependency;
    existing.stub |= incoming.stub;
    existing.unhealthy |= incoming.unhealthy;
    existing.unsupported |= incoming.unsupported;
}

fn ordered_eligibility(
    identity: &SoftwareIdentity,
    flags: EligibilityFlags,
) -> SoftwareEligibility {
    use SoftwareEligibilityReason as Reason;
    let reason = if flags.protected {
        Reason::ProtectedProduct
    } else if flags.source_incomplete {
        Reason::SourceIncomplete
    } else if flags.conflicting_identity {
        Reason::ConflictingIdentity
    } else if flags.no_remove {
        Reason::NoRemove
    } else if flags.hidden {
        Reason::HiddenEntry
    } else if flags.system_or_update {
        Reason::SystemOrUpdate
    } else if flags.dependency {
        Reason::DependencyPackage
    } else if flags.stub {
        Reason::StubPackage
    } else if flags.unhealthy {
        Reason::UnhealthyPackage
    } else if matches!(identity, SoftwareIdentity::Msi { .. }) {
        Reason::MsiExecutionNotSupportedV1
    } else if matches!(identity, SoftwareIdentity::Arp { .. }) {
        Reason::RegistryOnlyManual
    } else if flags.unsupported {
        Reason::UnsupportedSource
    } else {
        Reason::EligibleCurrentUserMsix
    };
    SoftwareEligibility {
        state: if reason == Reason::EligibleCurrentUserMsix {
            SoftwareEligibilityState::Selectable
        } else {
            SoftwareEligibilityState::Manual
        },
        reason,
    }
}

fn software_id(identity: &SoftwareIdentity) -> Result<String> {
    let canonical = serde_json::to_vec(&("devsweep.software.identity.v1", identity))
        .context("failed to serialize software identity")?;
    let family = match identity {
        SoftwareIdentity::Arp { .. } => "arp",
        SoftwareIdentity::Msi { .. } => "msi",
        SoftwareIdentity::Msix { .. } => "msix",
    };
    Ok(format!(
        "software:v1:{family}:{:x}",
        Sha256::digest(canonical)
    ))
}

fn inventory_fingerprint(inventory: &SoftwareInventoryV1) -> Result<String> {
    let canonical = serde_json::to_vec(&(
        "devsweep.software.inventory.v1",
        inventory.version,
        inventory.observed_at_unix_ms,
        &inventory.sources,
        &inventory.entries,
    ))
    .context("failed to serialize software inventory fingerprint")?;
    Ok(format!("sha256:{:x}", Sha256::digest(canonical)))
}

fn reported_size(
    kib: Option<u64>,
    source_code: SoftwareSizeSourceCode,
    observed_at_unix_ms: u64,
) -> SoftwareSizeEvidence {
    match kib.and_then(|value| value.checked_mul(1024)) {
        Some(value_bytes) => SoftwareSizeEvidence::Available {
            value_bytes,
            basis: SoftwareSizeBasis::ReportedEstimate,
            source_code,
            observed_at_unix_ms,
        },
        None => SoftwareSizeEvidence::Unknown {
            reason_code: if kib.is_some() {
                "reported_estimate_overflow"
            } else {
                "not_reported"
            }
            .to_string(),
        },
    }
}

fn is_protected_product(display_name: Option<&str>, publisher: Option<&str>) -> bool {
    display_name.is_some_and(|name| name.trim().eq_ignore_ascii_case("devsweep"))
        || publisher.is_some_and(|value| {
            value
                .split(|character: char| !character.is_ascii_alphanumeric())
                .any(|word| word.eq_ignore_ascii_case("devsweep"))
        })
}

fn unix_ms(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
#[path = "tests.rs"]
mod inventory_tests;

#[cfg(all(test, windows))]
mod windows {
    #[test]
    fn current_process_sid_is_exact_and_mta_worker_joins() {
        let sid = super::msix::current_process_sid().expect("current process SID");
        assert!(sid.starts_with("S-1-"));
        let _ = super::msix::inventory(1, None);
        assert_eq!(super::msix::active_mta_workers(), 0);
    }
}
