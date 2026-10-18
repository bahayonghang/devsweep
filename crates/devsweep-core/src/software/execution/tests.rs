use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use tempfile::TempDir;

use super::*;
use crate::software::launch::{self, LaunchRefusal};
use crate::software::model::{EligibilityFlags, SoftwareObservation, SoftwareSizeSourceCode};
use crate::software::{
    FixtureLaunch, FixtureLaunchKind, MsiContext, RegistryHive, RegistryView,
    SOFTWARE_INVENTORY_VERSION, SoftwareEligibility, SoftwareEligibilityState, SoftwareEntryV1,
    SoftwareIdentity, SoftwareLastUsedEvidence, SoftwareSizeEvidence, SoftwareSourceEvidence,
    SoftwareSourceId, SoftwareSourceState,
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
        self.record()
    }
}

impl FixtureRemovalAdapter for FakeAdapter {
    fn start(
        &self,
        _action: &ValidatedSoftwareAction,
        _cancel: Option<&Arc<FlagCancelObserver>>,
        _timing: RemovalTiming,
    ) -> AdapterEvidence {
        self.record()
    }
}

impl FakeAdapter {
    fn record(&self) -> AdapterEvidence {
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
        launch: None,
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

const FIXTURE_PRODUCT: &str = "{11111111-2222-3333-4444-555555555555}";

fn fixture_root(temp: &TempDir) -> PathBuf {
    let root = temp.path().join("fixture");
    fs::create_dir_all(&root).unwrap();
    root
}

fn msi_launch(root: &Path) -> FixtureLaunch {
    FixtureLaunch {
        kind: FixtureLaunchKind::Msi,
        root: root.to_path_buf(),
        executable: root.join("msiexec.exe"),
        arguments: vec![
            "/x".to_string(),
            FIXTURE_PRODUCT.to_string(),
            "/qn".to_string(),
            "/norestart".to_string(),
        ],
        target_id: FIXTURE_PRODUCT.to_string(),
    }
}

fn vendor_launch(root: &Path, app_id: &str, arguments: Vec<String>) -> FixtureLaunch {
    FixtureLaunch {
        kind: FixtureLaunchKind::Vendor,
        root: root.to_path_buf(),
        executable: root.join(app_id).join("uninstall.exe"),
        arguments,
        target_id: app_id.to_string(),
    }
}

fn scoop_launch(root: &Path, app_id: &str) -> FixtureLaunch {
    FixtureLaunch {
        kind: FixtureLaunchKind::Scoop,
        root: root.to_path_buf(),
        executable: root.join("scoop.exe"),
        arguments: vec!["uninstall".to_string(), app_id.to_string()],
        target_id: app_id.to_string(),
    }
}

fn launched_entry(identity: SoftwareIdentity, launch: FixtureLaunch) -> SoftwareEntryV1 {
    let mut item = entry(identity);
    item.scope = super::super::scope_for_identity(&item.identity);
    item.eligibility = SoftwareEligibility {
        state: SoftwareEligibilityState::Selectable,
        reason: launch::eligibility_reason(launch.kind),
    };
    item.launch = Some(launch);
    item
}

fn inventory_of(item: SoftwareEntryV1) -> SoftwareInventoryV1 {
    let mut inventory = SoftwareInventoryV1 {
        version: SOFTWARE_INVENTORY_VERSION,
        observed_at_unix_ms: unix_ms(SystemTime::now()),
        sources: Vec::new(),
        entries: vec![item],
        fingerprint: String::new(),
    };
    refresh_fingerprint(&mut inventory);
    inventory
}

fn fixture_executor(
    path: PathBuf,
    evidence: AdapterEvidence,
    state: SoftwareInstalledState,
    inventory: SoftwareInventoryV1,
) -> (SoftwareExecutor, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let fixture_calls = Arc::new(AtomicUsize::new(0));
    let fixture_adapter = Arc::new(FakeAdapter {
        calls: Arc::clone(&fixture_calls),
        evidence,
        cancel_on_call: None,
    });
    let msix_calls = Arc::new(AtomicUsize::new(0));
    let (requery, requery_calls) = FakeRequery::repeated(state);
    let executor = SoftwareExecutor::for_test(
        path,
        Arc::new(FakeAdapter {
            calls: msix_calls,
            evidence: AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterDispatchFailed),
            cancel_on_call: None,
        }),
        Arc::new(requery),
        Arc::new(FakeLiveInventory { inventory }),
    )
    .with_fixture_adapter(fixture_adapter);
    (executor, fixture_calls, requery_calls)
}

fn fixture_listing(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for item in fs::read_dir(&directory).unwrap() {
            let path = item.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let bytes = fs::read(&path).unwrap();
                files.push((path, bytes));
            }
        }
    }
    files.sort();
    files
}

fn assert_audit_redacted(path: &Path) {
    let text = fs::read_to_string(path).unwrap();
    assert!(text.find("\"kind\":\"validated\"").unwrap() < text.find("dispatch_started").unwrap());
    for forbidden in [
        "UninstallString",
        "QuietUninstallString",
        "cmd.exe",
        "powershell",
        "argv",
        "program",
        "windows-action-helper",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}");
    }
}

fn named_observation(identity: SoftwareIdentity, name: &str) -> SoftwareObservation {
    SoftwareObservation {
        scope: super::super::scope_for_identity(&identity),
        identity,
        display_name: Some(name.to_string()),
        publisher: Some("Publisher".to_string()),
        version: Some("1.0".to_string()),
        provenance: Vec::new(),
        size: SoftwareSizeEvidence::Unknown {
            reason_code: "not_reported".to_string(),
        },
        flags: EligibilityFlags::default(),
    }
}

#[test]
fn identity_collision_keeps_arp_msi_and_msix_separate_and_missing_size_is_not_zero() {
    let display_name = "Same Name";
    let arp = named_observation(
        SoftwareIdentity::Arp {
            hive: RegistryHive::CurrentUser,
            view: RegistryView::Registry64,
            subkey: "vendor-app".to_string(),
        },
        display_name,
    );
    let msi = named_observation(
        SoftwareIdentity::Msi {
            product_code: FIXTURE_PRODUCT.to_string(),
            context: MsiContext::UserUnmanaged,
        },
        display_name,
    );
    let msix = named_observation(
        SoftwareIdentity::Msix {
            package_full_name: "Same_1.0.0.0_x64__publisher".to_string(),
        },
        display_name,
    );
    let inventory = super::super::assemble_inventory(
        1,
        vec![
            SoftwareSourceEvidence::unavailable(
                SoftwareSourceId::Msi {
                    context: MsiContext::Machine,
                },
                SoftwareSourceState::Partial,
                "source_worker_panicked",
            ),
            SoftwareSourceEvidence::available(SoftwareSourceId::Arp {
                hive: RegistryHive::CurrentUser,
                view: RegistryView::Registry64,
            }),
            SoftwareSourceEvidence::available(SoftwareSourceId::MsixCurrentUser),
        ],
        vec![arp, msi, msix],
    )
    .unwrap();
    assert_eq!(inventory.entries.len(), 3);
    assert_eq!(inventory.sources.len(), 3);
    assert!(inventory.sources.iter().any(|source| {
        source.state == SoftwareSourceState::Partial
            && source.reason_code.as_deref() == Some("source_worker_panicked")
    }));
    let ids = inventory
        .entries
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 3);
    assert!(
        ids.iter()
            .all(|id| ids.iter().filter(|other| *other == id).count() == 1)
    );
    assert!(inventory.entries.iter().all(|item| {
        item.display_name.as_deref() == Some(display_name)
            && matches!(item.size, SoftwareSizeEvidence::Unknown { .. })
    }));
    let missing = super::super::reported_size(None, SoftwareSizeSourceCode::ArpEstimatedSizeKib, 1);
    let encoded = serde_json::to_string(&missing).unwrap();
    assert!(!encoded.contains("\"value_bytes\":0"));
    assert!(matches!(missing, SoftwareSizeEvidence::Unknown { .. }));
}

#[test]
fn shared_vendor_folder_is_not_deleted() {
    let temp = TempDir::new().unwrap();
    let root = fixture_root(&temp);
    let shared = root.join("shared");
    fs::create_dir_all(&shared).unwrap();
    fs::write(shared.join("payload.txt"), b"keep").unwrap();
    fs::create_dir_all(root.join("vendor-b")).unwrap();
    fs::write(root.join("vendor-b").join("keep.txt"), b"sibling").unwrap();
    let before = fixture_listing(&root);
    let launch = vendor_launch(&root, "vendor-a", vec!["/S".to_string()]);
    assert!(matches!(
        launch::prepare_fixture_start(&launch),
        Err(LaunchRefusal::MissingTool)
    ));
    assert!(!super::provider::refusal_starts_process(
        LaunchRefusal::MissingTool
    ));
    let _ = super::provider::run_fixture_launch(&launch, std::time::Duration::from_millis(20));
    let identity = SoftwareIdentity::Arp {
        hive: RegistryHive::CurrentUser,
        view: RegistryView::Registry64,
        subkey: "vendor-a".to_string(),
    };
    let inventory = inventory_of(launched_entry(identity, launch));
    let (executor, calls, _) = fixture_executor(
        temp.path().join("software.jsonl"),
        AdapterEvidence::success(),
        SoftwareInstalledState::Absent,
        inventory.clone(),
    );
    let (plan, digest) = plan_and_digest(&inventory);
    let report = execute_fixture(&executor, &plan, &digest, None).unwrap();
    assert_eq!(
        report.outcomes[0].outcome,
        SoftwareExecutionOutcome::Removed
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(fixture_listing(&root), before);
    assert_eq!(fs::read(shared.join("payload.txt")).unwrap(), b"keep");
}

#[test]
fn malformed_registry_command_does_not_start() {
    let temp = TempDir::new().unwrap();
    let root = fixture_root(&temp);
    let executable = root.join("uninstall.exe");
    fs::create_dir_all(executable.parent().unwrap()).unwrap();
    fs::write(&executable, b"not-a-real-tool").unwrap();
    let before = fixture_listing(&root);
    let quoted = format!("\"{}\" /c", executable.display());
    let samples = [
        "cmd.exe /c calc",
        "powershell -command Remove-Item",
        "powershell -enc AA==",
        quoted.as_str(),
        "msiexec.exe /x {11111111-2222-3333-4444-555555555555} /qn",
        "C:\\Windows\\System32\\msiexec.exe /x {11111111-2222-3333-4444-555555555555} /qn /norestart",
    ];
    for sample in samples {
        let refusal = launch::recognize_registry_uninstall(sample, &root, "vendor-app")
            .expect_err("malformed command stays manual");
        assert!(!super::provider::refusal_starts_process(refusal));
    }
    assert_eq!(fixture_listing(&root), before);

    let mut launch = vendor_launch(&root, "vendor-app", vec!["/c".to_string()]);
    launch.executable = executable;
    let inventory = inventory_of(launched_entry(
        SoftwareIdentity::Arp {
            hive: RegistryHive::CurrentUser,
            view: RegistryView::Registry64,
            subkey: "vendor-app".to_string(),
        },
        launch,
    ));
    let (executor, calls, requery_calls) = fixture_executor(
        temp.path().join("software.jsonl"),
        AdapterEvidence::success(),
        SoftwareInstalledState::Absent,
        inventory.clone(),
    );
    let (plan, _) = plan_and_digest_without_preview(&inventory);
    assert!(
        execute_fixture(
            &executor,
            &plan,
            &format!("sha256:{}", "0".repeat(64)),
            None
        )
        .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(requery_calls.load(Ordering::SeqCst), 0);
    assert_eq!(fixture_listing(&root), before);
}

#[test]
fn missing_tool_does_not_start_or_change_the_fixture() {
    let temp = TempDir::new().unwrap();
    let root = fixture_root(&temp);
    fs::write(root.join("marker.txt"), b"unchanged").unwrap();
    let before = fixture_listing(&root);
    let launch = scoop_launch(&root, "git");
    assert!(matches!(
        launch::prepare_fixture_start(&launch),
        Err(LaunchRefusal::MissingTool)
    ));
    let evidence =
        super::provider::run_fixture_launch(&launch, std::time::Duration::from_millis(20));
    assert_eq!(
        evidence,
        AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterDispatchFailed)
    );
    let inventory = inventory_of(launched_entry(
        SoftwareIdentity::Arp {
            hive: RegistryHive::CurrentUser,
            view: RegistryView::Registry64,
            subkey: "git".to_string(),
        },
        launch,
    ));
    let path = temp.path().join("software.jsonl");
    let (requery, _) = FakeRequery::repeated(SoftwareInstalledState::Present);
    let executor = SoftwareExecutor::for_test(
        path.clone(),
        Arc::new(FakeAdapter {
            calls: Arc::new(AtomicUsize::new(0)),
            evidence: AdapterEvidence::success(),
            cancel_on_call: None,
        }),
        Arc::new(requery),
        Arc::new(FakeLiveInventory {
            inventory: inventory.clone(),
        }),
    );
    let (plan, digest) = plan_and_digest(&inventory);
    let report = execute_fixture(&executor, &plan, &digest, None).unwrap();
    assert_ne!(
        report.outcomes[0].outcome,
        SoftwareExecutionOutcome::Removed
    );
    assert_eq!(report.outcomes[0].outcome, SoftwareExecutionOutcome::Failed);
    assert_eq!(fixture_listing(&root), before);
    assert_audit_redacted(&path);
}

#[test]
fn command_drift_between_plan_and_launch_does_not_start() {
    let temp = TempDir::new().unwrap();
    let root = fixture_root(&temp);
    let launch = vendor_launch(&root, "vendor-app", vec!["/S".to_string()]);
    let mut inventory = inventory_of(launched_entry(
        SoftwareIdentity::Arp {
            hive: RegistryHive::CurrentUser,
            view: RegistryView::Registry64,
            subkey: "vendor-app".to_string(),
        },
        launch,
    ));
    let (plan, digest) = plan_and_digest(&inventory);
    let saved_fingerprint = inventory.fingerprint.clone();
    inventory.entries[0].launch.as_mut().unwrap().arguments = vec!["/quiet".to_string()];
    refresh_fingerprint(&mut inventory);
    assert_eq!(inventory.fingerprint, saved_fingerprint);
    let (executor, calls, requery_calls) = fixture_executor(
        temp.path().join("software.jsonl"),
        AdapterEvidence::success(),
        SoftwareInstalledState::Absent,
        inventory,
    );
    let error = execute_fixture(&executor, &plan, &digest, None).expect_err("drift");
    assert!(matches!(
        error,
        SoftwareExecutionError::Plan(SoftwarePlanError::DigestMismatch)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(requery_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn reboot_outcome_is_not_removed() {
    let temp = TempDir::new().unwrap();
    let root = fixture_root(&temp);
    let inventory = inventory_of(launched_entry(
        SoftwareIdentity::Msi {
            product_code: FIXTURE_PRODUCT.to_string(),
            context: MsiContext::UserUnmanaged,
        },
        msi_launch(&root),
    ));
    let (executor, calls, _) = fixture_executor(
        temp.path().join("software.jsonl"),
        AdapterEvidence::reboot_required(),
        SoftwareInstalledState::Absent,
        inventory.clone(),
    );
    let (plan, digest) = plan_and_digest(&inventory);
    let report = execute_fixture(&executor, &plan, &digest, None).unwrap();
    assert_eq!(
        report.outcomes[0].outcome,
        SoftwareExecutionOutcome::RebootRequired
    );
    assert_ne!(
        report.outcomes[0].outcome,
        SoftwareExecutionOutcome::Removed
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        super::provider::outcome_from_process_signal(
            Some(3010),
            false,
            SoftwareInstalledState::Absent
        ),
        SoftwareExecutionOutcome::RebootRequired
    );
}

#[test]
fn exit_code_or_opened_window_alone_is_not_removed() {
    assert_ne!(
        super::provider::outcome_from_process_signal(
            Some(0),
            false,
            SoftwareInstalledState::Present
        ),
        SoftwareExecutionOutcome::Removed
    );
    assert_ne!(
        super::provider::outcome_from_process_signal(None, true, SoftwareInstalledState::Present),
        SoftwareExecutionOutcome::Removed
    );
    assert_ne!(
        super::provider::outcome_from_process_signal(
            Some(0),
            true,
            SoftwareInstalledState::Unavailable
        ),
        SoftwareExecutionOutcome::Removed
    );
    assert_eq!(
        super::provider::outcome_from_process_signal(
            Some(0),
            false,
            SoftwareInstalledState::Absent
        ),
        SoftwareExecutionOutcome::Removed
    );

    let temp = TempDir::new().unwrap();
    let root = fixture_root(&temp);
    let inventory = inventory_of(launched_entry(
        SoftwareIdentity::Arp {
            hive: RegistryHive::CurrentUser,
            view: RegistryView::Registry64,
            subkey: "vendor-app".to_string(),
        },
        vendor_launch(&root, "vendor-app", vec!["/S".to_string()]),
    ));
    let (executor, _, _) = fixture_executor(
        temp.path().join("software.jsonl"),
        AdapterEvidence::success(),
        SoftwareInstalledState::Present,
        inventory.clone(),
    );
    let (plan, digest) = plan_and_digest(&inventory);
    let report = execute_fixture(&executor, &plan, &digest, None).unwrap();
    assert_eq!(
        report.outcomes[0].outcome,
        SoftwareExecutionOutcome::StillPresent
    );
}

#[test]
fn failed_canceled_still_present_and_unknown_do_not_authorize_leftovers() {
    let cases = [
        (
            None,
            AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterOperationFailed),
            SoftwareInstalledState::Present,
            SoftwareExecutionOutcome::Failed,
        ),
        (
            Some(true),
            AdapterEvidence::success(),
            SoftwareInstalledState::Present,
            SoftwareExecutionOutcome::CanceledBeforeStart,
        ),
        (
            None,
            AdapterEvidence::success(),
            SoftwareInstalledState::Present,
            SoftwareExecutionOutcome::StillPresent,
        ),
        (
            None,
            AdapterEvidence::unfinished(SoftwareAuditErrorCode::AdapterTimedOut),
            SoftwareInstalledState::Present,
            SoftwareExecutionOutcome::UnknownAfterDispatch,
        ),
    ];
    for (index, (cancel_first, evidence, state, expected)) in cases.into_iter().enumerate() {
        let temp = TempDir::new().unwrap();
        let root = fixture_root(&temp);
        let identity = SoftwareIdentity::Arp {
            hive: RegistryHive::CurrentUser,
            view: RegistryView::Registry64,
            subkey: "vendor-app".to_string(),
        };
        let inventory = inventory_of(launched_entry(
            identity.clone(),
            vendor_launch(&root, "vendor-app", Vec::new()),
        ));
        let path = temp.path().join(format!("software-{index}.jsonl"));
        let (executor, calls, _) =
            fixture_executor(path.clone(), evidence, state, inventory.clone());
        let (plan, digest) = plan_and_digest(&inventory);
        let cancel = cancel_first.map(|_| {
            let flag = Arc::new(FlagCancelObserver::new());
            flag.request_cancel();
            flag
        });
        let report = execute_fixture(&executor, &plan, &digest, cancel).unwrap();
        assert_eq!(report.outcomes[0].outcome, expected);
        assert_ne!(
            report.outcomes[0].outcome,
            SoftwareExecutionOutcome::Removed
        );
        if expected == SoftwareExecutionOutcome::CanceledBeforeStart {
            assert_eq!(calls.load(Ordering::SeqCst), 0);
        }
        let journal = super::super::audit::SoftwareAuditJournal::open(&path).unwrap();
        assert!(!journal.uninstall_succeeded(&report.outcomes[0].operation_id, &identity));
    }
}

#[test]
fn msi_vendor_and_scoop_fixtures_are_removed_only_when_requery_is_absent() {
    let temp = TempDir::new().unwrap();
    let root = fixture_root(&temp);
    let cases = [
        (
            launched_entry(
                SoftwareIdentity::Msi {
                    product_code: FIXTURE_PRODUCT.to_string(),
                    context: MsiContext::UserUnmanaged,
                },
                msi_launch(&root),
            ),
            FIXTURE_PRODUCT,
        ),
        (
            launched_entry(
                SoftwareIdentity::Arp {
                    hive: RegistryHive::CurrentUser,
                    view: RegistryView::Registry64,
                    subkey: "vendor-app".to_string(),
                },
                vendor_launch(&root, "vendor-app", vec!["/S".to_string()]),
            ),
            "vendor-app",
        ),
        (
            launched_entry(
                SoftwareIdentity::Arp {
                    hive: RegistryHive::CurrentUser,
                    view: RegistryView::Registry64,
                    subkey: "scoop-app".to_string(),
                },
                scoop_launch(&root, "scoop-app"),
            ),
            "scoop-app",
        ),
    ];
    for (index, (item, absent_key)) in cases.into_iter().enumerate() {
        for (present, expected) in [
            (false, SoftwareExecutionOutcome::Removed),
            (true, SoftwareExecutionOutcome::StillPresent),
        ] {
            let inventory = inventory_of(item.clone());
            let path = temp
                .path()
                .join(format!("software-{index}-{present}.jsonl"));
            let calls = Arc::new(AtomicUsize::new(0));
            let adapter = Arc::new(FakeAdapter {
                calls: Arc::clone(&calls),
                evidence: AdapterEvidence::success(),
                cancel_on_call: None,
            });
            let seen = Arc::new(Mutex::new(Vec::new()));
            let requery = KeyedRequery {
                absent_key: if present {
                    "other-id".to_string()
                } else {
                    absent_key.to_string()
                },
                seen: Arc::clone(&seen),
            };
            let executor = SoftwareExecutor::for_test(
                path.clone(),
                Arc::new(FakeAdapter {
                    calls: Arc::new(AtomicUsize::new(0)),
                    evidence: AdapterEvidence::failure(
                        SoftwareAuditErrorCode::AdapterDispatchFailed,
                    ),
                    cancel_on_call: None,
                }),
                Arc::new(requery),
                Arc::new(FakeLiveInventory {
                    inventory: inventory.clone(),
                }),
            )
            .with_fixture_adapter(adapter);
            let (plan, digest) = plan_and_digest(&inventory);
            let report = execute_fixture(&executor, &plan, &digest, None).unwrap();
            assert_eq!(report.outcomes[0].outcome, expected);
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert!(seen.lock().unwrap().iter().all(|key| key == absent_key));
            if !present {
                assert_audit_redacted(&path);
            }
        }
    }
}

#[test]
fn fixture_crash_after_dispatch_does_not_retry_and_stays_unknown_when_still_present() {
    let temp = TempDir::new().unwrap();
    let root = fixture_root(&temp);
    let path = temp.path().join("software.jsonl");
    let inventory = inventory_of(launched_entry(
        SoftwareIdentity::Arp {
            hive: RegistryHive::CurrentUser,
            view: RegistryView::Registry64,
            subkey: "vendor-app".to_string(),
        },
        vendor_launch(&root, "vendor-app", vec!["/S".to_string()]),
    ));
    let (mut executor, calls, requery_calls) = fixture_executor(
        path,
        AdapterEvidence::success(),
        SoftwareInstalledState::Present,
        inventory.clone(),
    );
    executor.crash_point = CrashPoint::AfterDispatchStarted;
    let (plan, digest) = plan_and_digest(&inventory);
    assert!(matches!(
        execute_fixture(&executor, &plan, &digest, None),
        Err(SoftwareExecutionError::CrashInjected(_))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    executor.crash_point = CrashPoint::None;
    let recovered = executor.recover_startup().unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        recovered[0].outcome,
        SoftwareExecutionOutcome::UnknownAfterDispatch
    );
    assert_eq!(requery_calls.load(Ordering::SeqCst), 3);
}

#[cfg(windows)]
fn install_disposable_provider(launch: &FixtureLaunch) {
    let parent = launch.executable.parent().expect("launch directory");
    fs::create_dir_all(parent).unwrap();
    fs::copy(
        crate::process::test_support::process_fixture_exe(),
        &launch.executable,
    )
    .unwrap();
    fs::write(parent.join("provider.json"), b"{}").unwrap();
}

#[cfg(windows)]
fn marker_paths(root: &Path) -> Vec<PathBuf> {
    fixture_listing(root)
        .into_iter()
        .map(|(path, _)| path)
        .filter(|path| {
            path.file_name().and_then(|name| name.to_str()) == Some("provider-launch.marker")
        })
        .collect()
}

#[cfg(windows)]
fn recorded_provider_calls(executable: &Path) -> Vec<Vec<String>> {
    fs::read_to_string(
        executable
            .parent()
            .expect("launch directory")
            .join("provider-calls.jsonl"),
    )
    .unwrap()
    .lines()
    .filter(|line| !line.is_empty())
    .map(|line| serde_json::from_str(line).unwrap())
    .collect()
}

#[test]
#[cfg(windows)]
fn disposable_provider_executables_run_isolated_msi_vendor_and_scoop() {
    let timeout = std::time::Duration::from_secs(15);
    let source = crate::process::test_support::process_fixture_exe();
    let source_dir = source.parent().expect("process_fixture directory");
    let msi_temp = TempDir::new().unwrap();
    let vendor_temp = TempDir::new().unwrap();
    let scoop_temp = TempDir::new().unwrap();
    let launches = [
        msi_launch(&fixture_root(&msi_temp)),
        vendor_launch(
            &fixture_root(&vendor_temp),
            "vendor-app",
            vec!["/S".to_string()],
        ),
        scoop_launch(&fixture_root(&scoop_temp), "scoop-app"),
    ];
    let temps = [msi_temp.path(), vendor_temp.path(), scoop_temp.path()];
    for (launch, temp) in launches.iter().zip(temps) {
        assert_ne!(
            launch.executable.file_name().and_then(|name| name.to_str()),
            Some("windows-action-helper.exe")
        );
        install_disposable_provider(launch);
        fs::write(launch.root.join("keep.txt"), b"retain").unwrap();
        let evidence = super::provider::run_fixture_launch(launch, timeout);
        assert_eq!(
            evidence,
            AdapterEvidence::success(),
            "{}",
            launch.executable.display()
        );
        let markers = marker_paths(temp);
        assert_eq!(markers.len(), 1, "{}", launch.executable.display());
        assert!(
            markers[0].starts_with(&launch.root),
            "{} is outside {}",
            markers[0].display(),
            launch.root.display()
        );
        assert!(!source_dir.join("provider-launch.marker").exists());
        assert_eq!(
            recorded_provider_calls(&launch.executable),
            vec![launch.arguments.clone()]
        );
        assert_eq!(fs::read(launch.root.join("keep.txt")).unwrap(), b"retain");
        assert!(!launch.root.join("windows-action-helper.exe").exists());
    }
}

struct KeyedRequery {
    absent_key: String,
    seen: Arc<Mutex<Vec<String>>>,
}

impl InstalledStateRequery for KeyedRequery {
    fn query(&self, key: &str) -> RequeryObservation {
        self.seen.lock().unwrap().push(key.to_string());
        if key == self.absent_key {
            observation(SoftwareInstalledState::Absent)
        } else {
            observation(SoftwareInstalledState::Present)
        }
    }
}
