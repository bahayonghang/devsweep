use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use tempfile::TempDir;

use super::*;
use crate::software::{
    MsiContext, RegistryHive, RegistryView, SOFTWARE_INVENTORY_VERSION, SoftwareEligibility,
    SoftwareEligibilityState, SoftwareEntryV1, SoftwareLastUsedEvidence, SoftwareSizeEvidence,
};

struct FakeAdapter {
    calls: Arc<AtomicUsize>,
    evidence: AdapterEvidence,
    cancel_on_call: Option<Arc<FlagCancelObserver>>,
}

struct SlowAdapter {
    calls: Arc<AtomicUsize>,
    active: Arc<AtomicUsize>,
    max_active: Arc<AtomicUsize>,
}

impl MsixRemovalAdapter for SlowAdapter {
    fn remove(
        &self,
        _action: &ValidatedSoftwareAction,
        _cancel: Option<&Arc<FlagCancelObserver>>,
        _timing: RemovalTiming,
    ) -> AdapterEvidence {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_active.fetch_max(active, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_millis(30));
        self.active.fetch_sub(1, Ordering::SeqCst);
        AdapterEvidence::success()
    }
}

struct FakeLiveInventory {
    inventory: SoftwareInventoryV1,
}

impl LiveSoftwareInventory for FakeLiveInventory {
    fn inventory(&self) -> Result<SoftwareInventoryV1, SoftwareExecutionError> {
        Ok(self.inventory.clone())
    }
}

impl MsixRemovalAdapter for FakeAdapter {
    fn remove(
        &self,
        _action: &ValidatedSoftwareAction,
        _cancel: Option<&Arc<FlagCancelObserver>>,
        _timing: RemovalTiming,
    ) -> AdapterEvidence {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(cancel) = &self.cancel_on_call {
            cancel.request_cancel();
        }
        self.evidence
    }
}

struct FakeRequery {
    calls: Arc<AtomicUsize>,
    observations: Mutex<VecDeque<RequeryObservation>>,
    fallback: RequeryObservation,
}

impl FakeRequery {
    fn repeated(state: SoftwareInstalledState) -> (Self, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        (
            Self {
                calls: Arc::clone(&calls),
                observations: Mutex::new(VecDeque::new()),
                fallback: observation(state),
            },
            calls,
        )
    }

    fn sequence(states: &[SoftwareInstalledState]) -> (Self, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let observations = states.iter().copied().map(observation).collect();
        (
            Self {
                calls: Arc::clone(&calls),
                observations: Mutex::new(observations),
                fallback: observation(*states.last().unwrap()),
            },
            calls,
        )
    }
}

impl InstalledStateRequery for FakeRequery {
    fn query(&self, _package_full_name: &str) -> RequeryObservation {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.observations
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(self.fallback)
    }
}

fn observation(state: SoftwareInstalledState) -> RequeryObservation {
    RequeryObservation {
        state,
        error_code: match state {
            SoftwareInstalledState::Unavailable => Some(SoftwareAuditErrorCode::RequeryUnavailable),
            SoftwareInstalledState::Conflicting => Some(SoftwareAuditErrorCode::RequeryConflicting),
            SoftwareInstalledState::Present | SoftwareInstalledState::Absent => None,
        },
    }
}

fn entry(identity: SoftwareIdentity) -> SoftwareEntryV1 {
    SoftwareEntryV1 {
        id: super::super::software_id(&identity).unwrap(),
        identity,
        scope: SoftwareScope::CurrentUser,
        display_name: Some("Fixture".to_string()),
        publisher: Some("Fixture Publisher".to_string()),
        version: Some("1.0.0.0".to_string()),
        provenance: Vec::new(),
        eligibility: SoftwareEligibility {
            state: SoftwareEligibilityState::Selectable,
            reason: SoftwareEligibilityReason::EligibleCurrentUserMsix,
        },
        size: SoftwareSizeEvidence::Unknown {
            reason_code: "fixture".to_string(),
        },
        last_used: SoftwareLastUsedEvidence::default(),
    }
}

fn fixture_inventory() -> SoftwareInventoryV1 {
    let identity = SoftwareIdentity::Msix {
        package_full_name: "Fixture_1.0.0.0_x64__publisher".to_string(),
    };
    let mut inventory = SoftwareInventoryV1 {
        version: SOFTWARE_INVENTORY_VERSION,
        observed_at_unix_ms: unix_ms(SystemTime::now()),
        sources: Vec::new(),
        entries: vec![entry(identity)],
        fingerprint: String::new(),
    };
    refresh_fingerprint(&mut inventory);
    inventory
}

fn refresh_fingerprint(inventory: &mut SoftwareInventoryV1) {
    inventory.fingerprint = super::super::inventory_fingerprint(inventory).unwrap();
}

fn plan_and_digest(inventory: &SoftwareInventoryV1) -> (SoftwareSelectionPlanV1, String) {
    let plan = super::super::build_selection_plan(
        inventory,
        &[inventory.entries[0].id.clone()],
        inventory.observed_at_unix_ms,
    )
    .unwrap();
    let preview =
        preview_selection_plan_live(&plan, inventory, inventory.observed_at_unix_ms).unwrap();
    (plan, preview.digest)
}

fn test_executor(
    path: PathBuf,
    evidence: AdapterEvidence,
    state: SoftwareInstalledState,
) -> (SoftwareExecutor, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    test_executor_with_inventory(path, evidence, state, fixture_inventory())
}

fn test_executor_with_inventory(
    path: PathBuf,
    evidence: AdapterEvidence,
    state: SoftwareInstalledState,
    live_inventory: SoftwareInventoryV1,
) -> (SoftwareExecutor, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let adapter_calls = Arc::new(AtomicUsize::new(0));
    let adapter = Arc::new(FakeAdapter {
        calls: Arc::clone(&adapter_calls),
        evidence,
        cancel_on_call: None,
    });
    let (requery, requery_calls) = FakeRequery::repeated(state);
    (
        SoftwareExecutor::for_test(
            path,
            adapter,
            Arc::new(requery),
            Arc::new(FakeLiveInventory {
                inventory: live_inventory,
            }),
        ),
        adapter_calls,
        requery_calls,
    )
}

#[test]
fn executor_private_live_source_rejects_non_msix_machine_protected_and_malformed_authority() {
    let identities = [
        SoftwareIdentity::Msi {
            product_code: "{00000000-0000-0000-0000-000000000001}".to_string(),
            context: MsiContext::UserUnmanaged,
        },
        SoftwareIdentity::Arp {
            hive: RegistryHive::CurrentUser,
            view: RegistryView::Registry64,
            subkey: "fixture".to_string(),
        },
        SoftwareIdentity::Msix {
            package_full_name: "..\\cmd.exe /c hostile".to_string(),
        },
    ];
    let mut inventories = identities
        .into_iter()
        .map(|identity| {
            let mut inventory = fixture_inventory();
            inventory.entries = vec![entry(identity)];
            refresh_fingerprint(&mut inventory);
            inventory
        })
        .collect::<Vec<_>>();

    let mut machine = fixture_inventory();
    machine.entries[0].scope = SoftwareScope::Machine;
    refresh_fingerprint(&mut machine);
    inventories.push(machine);

    let mut protected = fixture_inventory();
    protected.entries[0].eligibility = SoftwareEligibility {
        state: SoftwareEligibilityState::Manual,
        reason: SoftwareEligibilityReason::ProtectedProduct,
    };
    refresh_fingerprint(&mut protected);
    inventories.push(protected);

    for (index, inventory) in inventories.into_iter().enumerate() {
        let plan = SoftwareSelectionPlanV1 {
            version: super::super::SOFTWARE_PLAN_VERSION,
            inventory_fingerprint: inventory.fingerprint.clone(),
            inventory_observed_at_unix_ms: inventory.observed_at_unix_ms,
            expires_at_unix_ms: inventory.observed_at_unix_ms
                + super::super::SOFTWARE_INVENTORY_TTL_MS,
            selected_ids: vec![inventory.entries[0].id.clone()],
        };
        let temp = TempDir::new().unwrap();
        let (executor, adapter_calls, _) = test_executor_with_inventory(
            temp.path().join(format!("software-{index}.jsonl")),
            AdapterEvidence::success(),
            SoftwareInstalledState::Absent,
            inventory,
        );
        assert!(matches!(
            execute_fixture(
                &executor,
                &plan,
                &format!("sha256:{}", "0".repeat(64)),
                None,
            ),
            Err(SoftwareExecutionError::Plan(_))
        ));
        assert_eq!(adapter_calls.load(Ordering::SeqCst), 0);
    }
}

fn execute_fixture(
    executor: &SoftwareExecutor,
    plan: &SoftwareSelectionPlanV1,
    digest: &str,
    cancel: Option<Arc<FlagCancelObserver>>,
) -> Result<SoftwareExecutionReportV1, SoftwareExecutionError> {
    executor.execute(SoftwareExecutionRequest {
        plan,
        expected_preview_digest: digest,
        confirmed: true,
        cancel,
    })
}

#[test]
fn missing_confirmation_fails_before_audit_and_adapter() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("software.jsonl");
    let (executor, adapter_calls, _) = test_executor(
        path.clone(),
        AdapterEvidence::success(),
        SoftwareInstalledState::Absent,
    );
    let inventory = fixture_inventory();
    let (plan, digest) = plan_and_digest(&inventory);
    assert!(matches!(
        executor.execute(SoftwareExecutionRequest {
            plan: &plan,
            expected_preview_digest: &digest,
            confirmed: false,
            cancel: None,
        }),
        Err(SoftwareExecutionError::ConfirmationRequired)
    ));
    assert_eq!(adapter_calls.load(Ordering::SeqCst), 0);
    assert!(!path.exists());
}

#[test]
fn opaque_action_is_constructed_only_from_live_exact_current_user_msix() {
    let inventory = fixture_inventory();
    let (plan, digest) = plan_and_digest(&inventory);
    let actions =
        construct_validated_actions(&plan, &inventory, &digest, inventory.observed_at_unix_ms)
            .unwrap();
    assert_eq!(actions.len(), 1);
    assert_eq!(
        actions[0].package_full_name(),
        "Fixture_1.0.0.0_x64__publisher"
    );
    assert!(!std::any::type_name::<ValidatedSoftwareAction>().is_empty());
}

#[test]
fn msi_registry_machine_manual_stale_and_malformed_identities_have_no_constructor() {
    let identities = [
        SoftwareIdentity::Msi {
            product_code: "{00000000-0000-0000-0000-000000000001}".to_string(),
            context: MsiContext::UserUnmanaged,
        },
        SoftwareIdentity::Arp {
            hive: RegistryHive::CurrentUser,
            view: RegistryView::Registry64,
            subkey: "fixture".to_string(),
        },
    ];
    for identity in identities {
        let mut inventory = fixture_inventory();
        inventory.entries = vec![entry(identity)];
        refresh_fingerprint(&mut inventory);
        let (plan, _) = plan_and_digest_without_preview(&inventory);
        assert!(matches!(
            preview_selection_plan_live(&plan, &inventory, inventory.observed_at_unix_ms),
            Err(SoftwarePlanError::InvalidSelectedIdentity(_))
        ));
    }

    let mut machine = fixture_inventory();
    machine.entries[0].scope = SoftwareScope::Machine;
    refresh_fingerprint(&mut machine);
    let (plan, _) = plan_and_digest_without_preview(&machine);
    assert!(matches!(
        preview_selection_plan_live(&plan, &machine, machine.observed_at_unix_ms),
        Err(SoftwarePlanError::ManualSelection(_))
    ));

    let mut manual = fixture_inventory();
    manual.entries[0].eligibility = SoftwareEligibility {
        state: SoftwareEligibilityState::Manual,
        reason: SoftwareEligibilityReason::ProtectedProduct,
    };
    refresh_fingerprint(&mut manual);
    assert!(matches!(
        super::super::build_selection_plan(
            &manual,
            &[manual.entries[0].id.clone()],
            manual.observed_at_unix_ms
        ),
        Err(SoftwarePlanError::ManualSelection(_))
    ));

    let mut stale = fixture_inventory();
    let (stale_plan, _) = plan_and_digest(&stale);
    stale.entries.clear();
    refresh_fingerprint(&mut stale);
    assert!(matches!(
        preview_selection_plan_live(&stale_plan, &stale, stale.observed_at_unix_ms),
        Err(SoftwarePlanError::StaleSelection(_))
    ));

    let mut malformed = fixture_inventory();
    malformed.entries[0] = entry(SoftwareIdentity::Msix {
        package_full_name: "..\\cmd.exe /c hostile".to_string(),
    });
    refresh_fingerprint(&mut malformed);
    let (plan, _) = plan_and_digest_without_preview(&malformed);
    assert!(matches!(
        preview_selection_plan_live(&plan, &malformed, malformed.observed_at_unix_ms),
        Err(SoftwarePlanError::InvalidSelectedIdentity(_))
    ));
}

fn plan_and_digest_without_preview(
    inventory: &SoftwareInventoryV1,
) -> (SoftwareSelectionPlanV1, String) {
    let plan = super::super::build_selection_plan(
        inventory,
        &[inventory.entries[0].id.clone()],
        inventory.observed_at_unix_ms,
    )
    .unwrap();
    (plan, format!("sha256:{}", "0".repeat(64)))
}

#[test]
fn all_msi_manual_and_digest_mismatch_reach_no_adapter() {
    let temp = TempDir::new().unwrap();
    let (executor, adapter_calls, _) = test_executor(
        temp.path().join("software.jsonl"),
        AdapterEvidence::success(),
        SoftwareInstalledState::Absent,
    );
    let mut inventory = fixture_inventory();
    inventory.entries[0] = entry(SoftwareIdentity::Msi {
        product_code: "{00000000-0000-0000-0000-000000000001}".to_string(),
        context: MsiContext::Machine,
    });
    inventory.entries[0].eligibility = SoftwareEligibility {
        state: SoftwareEligibilityState::Manual,
        reason: SoftwareEligibilityReason::MsiExecutionNotSupportedV1,
    };
    refresh_fingerprint(&mut inventory);
    assert!(matches!(
        super::super::build_selection_plan(
            &inventory,
            &[inventory.entries[0].id.clone()],
            inventory.observed_at_unix_ms
        ),
        Err(SoftwarePlanError::ManualSelection(_))
    ));
    assert_eq!(adapter_calls.load(Ordering::SeqCst), 0);

    let inventory = fixture_inventory();
    let (plan, _) = plan_and_digest(&inventory);
    let error = execute_fixture(
        &executor,
        &plan,
        &format!("sha256:{}", "f".repeat(64)),
        None,
    )
    .expect_err("digest mismatch fails before audit and adapter");
    assert!(matches!(
        error,
        SoftwareExecutionError::Plan(SoftwarePlanError::DigestMismatch)
    ));
    assert_eq!(adapter_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn terminal_evidence_table_is_exact_and_has_no_partial_state() {
    let cases = [
        (
            AdapterEvidence::success(),
            SoftwareInstalledState::Absent,
            SoftwareExecutionOutcome::Removed,
        ),
        (
            AdapterEvidence::success(),
            SoftwareInstalledState::Present,
            SoftwareExecutionOutcome::StillPresent,
        ),
        (
            AdapterEvidence::success(),
            SoftwareInstalledState::Unavailable,
            SoftwareExecutionOutcome::UnknownAfterDispatch,
        ),
        (
            AdapterEvidence::success(),
            SoftwareInstalledState::Conflicting,
            SoftwareExecutionOutcome::UnknownAfterDispatch,
        ),
        (
            AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterOperationFailed),
            SoftwareInstalledState::Absent,
            SoftwareExecutionOutcome::Removed,
        ),
        (
            AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterOperationFailed),
            SoftwareInstalledState::Present,
            SoftwareExecutionOutcome::Failed,
        ),
        (
            AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterOperationFailed),
            SoftwareInstalledState::Unavailable,
            SoftwareExecutionOutcome::UnknownAfterDispatch,
        ),
        (
            AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterOperationFailed),
            SoftwareInstalledState::Conflicting,
            SoftwareExecutionOutcome::UnknownAfterDispatch,
        ),
        (
            AdapterEvidence::reboot_required(),
            SoftwareInstalledState::Absent,
            SoftwareExecutionOutcome::RebootRequired,
        ),
        (
            AdapterEvidence::reboot_required(),
            SoftwareInstalledState::Present,
            SoftwareExecutionOutcome::RebootRequired,
        ),
        (
            AdapterEvidence::reboot_required(),
            SoftwareInstalledState::Unavailable,
            SoftwareExecutionOutcome::RebootRequired,
        ),
        (
            AdapterEvidence::reboot_required(),
            SoftwareInstalledState::Conflicting,
            SoftwareExecutionOutcome::RebootRequired,
        ),
        (
            AdapterEvidence::unfinished(SoftwareAuditErrorCode::AdapterTimedOut),
            SoftwareInstalledState::Absent,
            SoftwareExecutionOutcome::Removed,
        ),
        (
            AdapterEvidence::unfinished(SoftwareAuditErrorCode::AdapterTimedOut),
            SoftwareInstalledState::Present,
            SoftwareExecutionOutcome::UnknownAfterDispatch,
        ),
        (
            AdapterEvidence::unfinished(SoftwareAuditErrorCode::AdapterTimedOut),
            SoftwareInstalledState::Unavailable,
            SoftwareExecutionOutcome::UnknownAfterDispatch,
        ),
        (
            AdapterEvidence::unfinished(SoftwareAuditErrorCode::AdapterTimedOut),
            SoftwareInstalledState::Conflicting,
            SoftwareExecutionOutcome::UnknownAfterDispatch,
        ),
    ];
    for (adapter, installed, expected) in cases {
        assert_eq!(classify_terminal(adapter, installed), expected);
    }
    let serialized = serde_json::to_string(&cases.map(|case| case.2)).unwrap();
    assert!(!serialized.contains("partial"));
    assert!(!serialized.contains("reversible"));
}

#[test]
fn executor_covers_removed_reboot_present_failure_success_and_unknown_fixtures() {
    let cases = [
        (
            AdapterEvidence::success(),
            SoftwareInstalledState::Absent,
            SoftwareExecutionOutcome::Removed,
        ),
        (
            AdapterEvidence::reboot_required(),
            SoftwareInstalledState::Absent,
            SoftwareExecutionOutcome::RebootRequired,
        ),
        (
            AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterOperationFailed),
            SoftwareInstalledState::Present,
            SoftwareExecutionOutcome::Failed,
        ),
        (
            AdapterEvidence::success(),
            SoftwareInstalledState::Present,
            SoftwareExecutionOutcome::StillPresent,
        ),
        (
            AdapterEvidence::unfinished(SoftwareAuditErrorCode::AdapterTimedOut),
            SoftwareInstalledState::Present,
            SoftwareExecutionOutcome::UnknownAfterDispatch,
        ),
        (
            AdapterEvidence::success(),
            SoftwareInstalledState::Conflicting,
            SoftwareExecutionOutcome::UnknownAfterDispatch,
        ),
    ];
    for (index, (adapter, installed, expected)) in cases.into_iter().enumerate() {
        let temp = TempDir::new().unwrap();
        let (executor, adapter_calls, requery_calls) = test_executor(
            temp.path().join(format!("software-{index}.jsonl")),
            adapter,
            installed,
        );
        let inventory = fixture_inventory();
        let (plan, digest) = plan_and_digest(&inventory);
        let report = execute_fixture(&executor, &plan, &digest, None).unwrap();
        assert_eq!(report.outcomes[0].outcome, expected);
        assert!(report.irreversible);
        assert_eq!(adapter_calls.load(Ordering::SeqCst), 1);
        assert_eq!(requery_calls.load(Ordering::SeqCst), 3);
    }
}

#[test]
fn cancellation_before_dispatch_is_closed_and_calls_no_adapter() {
    let temp = TempDir::new().unwrap();
    let (executor, adapter_calls, requery_calls) = test_executor(
        temp.path().join("software.jsonl"),
        AdapterEvidence::success(),
        SoftwareInstalledState::Absent,
    );
    let cancel = Arc::new(FlagCancelObserver::new());
    cancel.request_cancel();
    let inventory = fixture_inventory();
    let (plan, digest) = plan_and_digest(&inventory);
    let report = execute_fixture(&executor, &plan, &digest, Some(cancel)).unwrap();
    assert_eq!(
        report.outcomes[0].outcome,
        SoftwareExecutionOutcome::CanceledBeforeStart
    );
    assert_eq!(adapter_calls.load(Ordering::SeqCst), 0);
    assert_eq!(requery_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn post_dispatch_cancel_or_timeout_never_reports_canceled() {
    for code in [
        SoftwareAuditErrorCode::AdapterCanceledAfterDispatch,
        SoftwareAuditErrorCode::AdapterTimedOut,
    ] {
        let temp = TempDir::new().unwrap();
        let cancel = Arc::new(FlagCancelObserver::new());
        let adapter_calls = Arc::new(AtomicUsize::new(0));
        let adapter = Arc::new(FakeAdapter {
            calls: Arc::clone(&adapter_calls),
            evidence: AdapterEvidence::unfinished(code),
            cancel_on_call: Some(Arc::clone(&cancel)),
        });
        let (requery, _) = FakeRequery::repeated(SoftwareInstalledState::Present);
        let executor = SoftwareExecutor::for_test(
            temp.path().join("software.jsonl"),
            adapter,
            Arc::new(requery),
            Arc::new(FakeLiveInventory {
                inventory: fixture_inventory(),
            }),
        );
        let inventory = fixture_inventory();
        let (plan, digest) = plan_and_digest(&inventory);
        let report = execute_fixture(&executor, &plan, &digest, Some(cancel)).unwrap();
        assert_eq!(
            report.outcomes[0].outcome,
            SoftwareExecutionOutcome::UnknownAfterDispatch
        );
        assert!(report.outcomes[0].outcome.was_dispatched());
        assert_eq!(adapter_calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn audit_lock_failure_occurs_before_adapter_call() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("software.jsonl");
    let held = SoftwareAuditJournal::open(&path).unwrap();
    let (executor, adapter_calls, _) = test_executor(
        path,
        AdapterEvidence::success(),
        SoftwareInstalledState::Absent,
    );
    let inventory = fixture_inventory();
    let (plan, digest) = plan_and_digest(&inventory);
    let error = execute_fixture(&executor, &plan, &digest, None)
        .expect_err("contended audit lock fails closed");
    assert!(matches!(
        error,
        SoftwareExecutionError::Audit(SoftwareAuditError::LockUnavailable(_))
    ));
    assert_eq!(adapter_calls.load(Ordering::SeqCst), 0);
    drop(held);
}

#[test]
fn process_single_flight_serializes_concurrent_adapters() {
    let temp = TempDir::new().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let adapter = Arc::new(SlowAdapter {
        calls: Arc::clone(&calls),
        active: Arc::clone(&active),
        max_active: Arc::clone(&max_active),
    });
    let inventory = fixture_inventory();
    let (plan, digest) = plan_and_digest(&inventory);
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let mut workers = Vec::new();
    for index in 0..2 {
        let (requery, _) = FakeRequery::repeated(SoftwareInstalledState::Absent);
        let executor = SoftwareExecutor::for_test(
            temp.path().join(format!("software-{index}.jsonl")),
            adapter.clone(),
            Arc::new(requery),
            Arc::new(FakeLiveInventory {
                inventory: inventory.clone(),
            }),
        );
        let plan = plan.clone();
        let digest = digest.clone();
        let barrier = Arc::clone(&barrier);
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            execute_fixture(&executor, &plan, &digest, None).unwrap();
        }));
    }
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(max_active.load(Ordering::SeqCst), 1);
}

#[test]
fn crashes_before_and_after_dispatch_recover_without_removal_retry() {
    for crash_point in [
        CrashPoint::BeforeDispatchStarted,
        CrashPoint::AfterDispatchStarted,
        CrashPoint::AfterAdapterCompleted,
    ] {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("software.jsonl");
        let (mut executor, adapter_calls, requery_calls) = test_executor(
            path,
            AdapterEvidence::success(),
            SoftwareInstalledState::Absent,
        );
        executor.crash_point = crash_point;
        let inventory = fixture_inventory();
        let (plan, digest) = plan_and_digest(&inventory);
        assert!(matches!(
            execute_fixture(&executor, &plan, &digest, None),
            Err(SoftwareExecutionError::CrashInjected(_))
        ));
        let calls_before_recovery = adapter_calls.load(Ordering::SeqCst);
        executor.crash_point = CrashPoint::None;
        let recovered = executor.recover_startup().unwrap();
        assert_eq!(adapter_calls.load(Ordering::SeqCst), calls_before_recovery);
        assert_eq!(recovered.len(), 1);
        if crash_point == CrashPoint::BeforeDispatchStarted {
            assert_eq!(
                recovered[0].outcome,
                SoftwareExecutionOutcome::CanceledBeforeStart
            );
            assert_eq!(requery_calls.load(Ordering::SeqCst), 0);
        } else {
            assert_eq!(recovered[0].outcome, SoftwareExecutionOutcome::Removed);
            assert_eq!(requery_calls.load(Ordering::SeqCst), 3);
        }
        assert!(executor.recover_startup().unwrap().is_empty());
    }
}

#[test]
fn recovery_uses_last_identity_only_requery_observation() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("software.jsonl");
    let adapter_calls = Arc::new(AtomicUsize::new(0));
    let adapter = Arc::new(FakeAdapter {
        calls: Arc::clone(&adapter_calls),
        evidence: AdapterEvidence::success(),
        cancel_on_call: None,
    });
    let (requery, requery_calls) = FakeRequery::sequence(&[
        SoftwareInstalledState::Present,
        SoftwareInstalledState::Absent,
        SoftwareInstalledState::Absent,
    ]);
    let mut executor = SoftwareExecutor::for_test(
        path,
        adapter,
        Arc::new(requery),
        Arc::new(FakeLiveInventory {
            inventory: fixture_inventory(),
        }),
    );
    executor.crash_point = CrashPoint::AfterDispatchStarted;
    let inventory = fixture_inventory();
    let (plan, digest) = plan_and_digest(&inventory);
    let _ = execute_fixture(&executor, &plan, &digest, None);
    executor.crash_point = CrashPoint::None;
    let recovered = executor.recover_startup().unwrap();
    assert_eq!(recovered[0].outcome, SoftwareExecutionOutcome::Removed);
    assert_eq!(adapter_calls.load(Ordering::SeqCst), 0);
    assert_eq!(requery_calls.load(Ordering::SeqCst), 3);
}

#[test]
fn one_conflicting_identity_observation_remains_fail_closed() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("software.jsonl");
    let adapter_calls = Arc::new(AtomicUsize::new(0));
    let adapter = Arc::new(FakeAdapter {
        calls: Arc::clone(&adapter_calls),
        evidence: AdapterEvidence::success(),
        cancel_on_call: None,
    });
    let (requery, requery_calls) = FakeRequery::sequence(&[
        SoftwareInstalledState::Conflicting,
        SoftwareInstalledState::Absent,
        SoftwareInstalledState::Absent,
    ]);
    let executor = SoftwareExecutor::for_test(
        path,
        adapter,
        Arc::new(requery),
        Arc::new(FakeLiveInventory {
            inventory: fixture_inventory(),
        }),
    );
    let inventory = fixture_inventory();
    let (plan, digest) = plan_and_digest(&inventory);
    let report = execute_fixture(&executor, &plan, &digest, None).unwrap();
    assert_eq!(
        report.outcomes[0].outcome,
        SoftwareExecutionOutcome::UnknownAfterDispatch
    );
    assert_eq!(adapter_calls.load(Ordering::SeqCst), 1);
    assert_eq!(requery_calls.load(Ordering::SeqCst), 3);
}
