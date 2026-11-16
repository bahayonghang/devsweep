use super::msi::{RegistrationProvider, ToolResolver};
use super::*;
use crate::process::ProcessStatus;
use crate::software::{
    MsiContext, MsiRegistrationEvidenceV1, SoftwareEligibility, SoftwareEligibilityState,
    SoftwareEntryV1, SoftwareLastUsedEvidence, SoftwareSizeEvidence, build_selection_plan,
    software_entry_id, software_inventory_fingerprint,
};
use crate::software::{msi::MsiRegistrationBinding, msi_authority::BoundFile};
use std::{path::Path, sync::atomic::AtomicUsize};
use tempfile::TempDir;

const CODE: &str = "{12345678-1234-1234-1234-1234567890AB}";
const SID: &str = "S-1-5-21-100";

struct Registration {
    package_path: Mutex<Option<PathBuf>>,
    proof: Mutex<Option<MsiRegistrationEvidenceV1>>,
    installed: SoftwareInstalledState,
    binds: AtomicUsize,
    change_at: usize,
}
impl msi::RegistrationProvider for Registration {
    fn bind(&self, identity: &SoftwareIdentity) -> Result<MsiRegistrationBinding, ()> {
        assert_eq!(identity, &identity_value());
        let count = self.binds.fetch_add(1, Ordering::SeqCst) + 1;
        let mut evidence = self.proof.lock().unwrap().clone().ok_or(())?;
        if count >= self.change_at {
            evidence.registration_digest = format!("sha256:{}", "c".repeat(64));
        }
        Ok(MsiRegistrationBinding {
            evidence,
            _package: self
                .package_path
                .lock()
                .unwrap()
                .as_ref()
                .map(|path| BoundFile::open(path))
                .transpose()?,
        })
    }
    fn query(&self, identity: &SoftwareIdentity, user_sid: &str) -> SoftwareInstalledState {
        assert_eq!(identity, &identity_value());
        assert_eq!(user_sid, SID);
        self.installed
    }
}
struct Tool(std::path::PathBuf);
impl msi::ToolResolver for Tool {
    fn bind(&self) -> Result<BoundFile, ()> {
        BoundFile::open(&self.0)
    }
}
struct Process {
    maintain_cache: Mutex<Option<PathBuf>>,
    calls: Mutex<Vec<(PathBuf, Vec<String>)>>,
    status: ProcessStatus,
}
impl msi::ProcessAdapter for Process {
    fn run(
        &self,
        program: &Path,
        args: &[String],
        _: Option<&Arc<FlagCancelObserver>>,
        _: Duration,
    ) -> ProcessStatus {
        if let Some(cache) = self.maintain_cache.lock().unwrap().as_ref() {
            std::fs::remove_file(cache)
                .expect("Installer can maintain its cache after final registration check");
            #[cfg(windows)]
            assert!(
                std::fs::write(program, b"replacement").is_err(),
                "system tool binding stays immutable during dispatch"
            );
        }
        self.calls
            .lock()
            .unwrap()
            .push((program.to_path_buf(), args.to_vec()));
        self.status.clone()
    }
}
struct Inventory {
    registration: Arc<Registration>,
    tool: Arc<Tool>,
}
impl LiveSoftwareInventory for Inventory {
    fn inventory(&self) -> Result<SoftwareInventoryV1, SoftwareExecutionError> {
        let proof = self
            .registration
            .bind(&identity_value())
            .ok()
            .and_then(|mut bound| {
                bound.evidence.tool_digest = self.tool.bind().ok()?.digest;
                Some(bound.evidence)
            });
        Ok(inventory(proof))
    }
}
fn identity_value() -> SoftwareIdentity {
    SoftwareIdentity::Msi {
        product_code: CODE.to_string(),
        context: MsiContext::UserUnmanaged,
    }
}
fn inventory(proof: Option<MsiRegistrationEvidenceV1>) -> SoftwareInventoryV1 {
    let mut inventory = SoftwareInventoryV1 {
        version: 1,
        observed_at_unix_ms: unix_ms(SystemTime::now()),
        sources: Vec::new(),
        fingerprint: String::new(),
        entries: vec![SoftwareEntryV1 {
            id: software_entry_id(&identity_value()).unwrap(),
            identity: identity_value(),
            scope: SoftwareScope::CurrentUser,
            display_name: Some("MSI fixture".to_string()),
            publisher: None,
            version: Some("1.0".to_string()),
            icon: None,
            installed_on: None,
            provenance: Vec::new(),
            eligibility: SoftwareEligibility {
                state: if proof.is_some() {
                    SoftwareEligibilityState::Selectable
                } else {
                    SoftwareEligibilityState::Manual
                },
                reason: if proof.is_some() {
                    SoftwareEligibilityReason::EligibleCurrentUserMsi
                } else {
                    SoftwareEligibilityReason::MsiAuthorityUnavailable
                },
            },
            size: SoftwareSizeEvidence::Unknown {
                reason_code: "fixture".to_string(),
            },
            last_used: SoftwareLastUsedEvidence::default(),
            msi_registration: proof,
            launch: None,
        }],
    };
    inventory.fingerprint = software_inventory_fingerprint(&inventory).unwrap();
    inventory
}
struct Fixture {
    _temp: TempDir,
    tool: Arc<Tool>,
    registration: Arc<Registration>,
    process: Arc<Process>,
    executor: SoftwareExecutor,
    plan: SoftwareSelectionPlanV1,
    digest: String,
}
impl Fixture {
    fn new(status: ProcessStatus, installed: SoftwareInstalledState, change_at: usize) -> Self {
        let temp = TempDir::new().unwrap();
        let tool = Arc::new(Tool(temp.path().join("msiexec.exe")));
        std::fs::write(&tool.0, b"isolated fake, never executed").unwrap();
        let registration = Arc::new(Registration {
            package_path: Mutex::new(None),
            proof: Mutex::new(Some(MsiRegistrationEvidenceV1 {
                user_sid: SID.to_string(),
                package_code: CODE.to_string(),
                registration_digest: format!("sha256:{}", "a".repeat(64)),
                tool_digest: String::new(),
            })),
            installed,
            binds: AtomicUsize::new(0),
            change_at,
        });
        let process = Arc::new(Process {
            maintain_cache: Mutex::new(None),
            calls: Mutex::new(Vec::new()),
            status,
        });
        let live = Arc::new(Inventory {
            registration: registration.clone(),
            tool: tool.clone(),
        });
        let initial = live.inventory().unwrap();
        let plan = build_selection_plan(
            &initial,
            &[initial.entries[0].id.clone()],
            unix_ms(SystemTime::now()),
        )
        .unwrap();
        let digest = preview_selection_plan_live(&plan, &initial, unix_ms(SystemTime::now()))
            .unwrap()
            .digest;
        let executor = SoftwareExecutor {
            msi: msi::MsiRuntime {
                registrations: registration.clone(),
                tool: tool.clone(),
                process: process.clone(),
            },
            live_inventory: live,
            audit_path: Some(temp.path().join("audit/software.jsonl")),
            requery_offsets: [Duration::ZERO; 3],
            ..SoftwareExecutor::default()
        };
        Self {
            _temp: temp,
            tool,
            registration,
            process,
            executor,
            plan,
            digest,
        }
    }
    fn execute(&self) -> Result<SoftwareExecutionReportV1, SoftwareExecutionError> {
        self.executor.execute(SoftwareExecutionRequest {
            plan: &self.plan,
            expected_preview_digest: &self.digest,
            confirmed: true,
            cancel: None,
        })
    }
    fn calls(&self) -> usize {
        self.process.calls.lock().unwrap().len()
    }
}

#[test]
fn msi_full_funnel_exact_argv_and_exit_state_matrix() {
    use SoftwareExecutionOutcome as Outcome;
    use SoftwareInstalledState as State;
    for code in [0, 3010, 1641, 1605, 1614, 1618, 1603, 1602, 3011, 9999] {
        for state in [
            State::Absent,
            State::Present,
            State::Unavailable,
            State::Conflicting,
        ] {
            let fixture = Fixture::new(
                if code == 0 {
                    ProcessStatus::Success
                } else {
                    ProcessStatus::Exit { code: Some(code) }
                },
                state,
                usize::MAX,
            );
            let result = fixture.execute().unwrap();
            let expected = match code {
                3010 | 1641 => Outcome::RebootRequired,
                0 => match state {
                    State::Absent => Outcome::Removed,
                    State::Present => Outcome::StillPresent,
                    _ => Outcome::UnknownAfterDispatch,
                },
                _ => Outcome::Failed,
            };
            assert_eq!(result.outcomes[0].outcome, expected, "{code}/{state:?}");
            assert_eq!(
                result.outcomes[0]
                    .msi
                    .as_ref()
                    .unwrap()
                    .process
                    .unwrap()
                    .exit_code,
                Some(code as u32)
            );
            let calls = fixture.process.calls.lock().unwrap();
            assert_eq!(calls.len(), 1);
            assert!(calls[0].0.is_absolute());
            assert_eq!(calls[0].0.file_name().unwrap(), "msiexec.exe");
            assert_eq!(calls[0].1, ["/x", CODE, "/qn", "/norestart"]);
            let journal =
                SoftwareAuditJournal::open(fixture.executor.audit_path.as_ref().unwrap()).unwrap();
            assert_eq!(
                journal.uninstall_removed(&result.outcomes[0].operation_id, &identity_value()),
                expected == Outcome::Removed
            );
        }
    }
}

#[test]
fn msi_refuses_unconfirmed_stale_registration_restriction_and_binary_without_process() {
    let fixture = Fixture::new(
        ProcessStatus::Success,
        SoftwareInstalledState::Absent,
        usize::MAX,
    );
    assert!(
        fixture
            .executor
            .execute(SoftwareExecutionRequest {
                plan: &fixture.plan,
                expected_preview_digest: &fixture.digest,
                confirmed: false,
                cancel: None
            })
            .is_err()
    );
    assert_eq!(fixture.calls(), 0);
    *fixture.registration.proof.lock().unwrap() = None;
    assert!(fixture.execute().is_err());
    assert_eq!(fixture.calls(), 0);
    for change_at in [2, 3] {
        let fixture = Fixture::new(
            ProcessStatus::Success,
            SoftwareInstalledState::Absent,
            change_at,
        );
        let result = fixture.execute();
        assert!(
            result.is_err()
                || result.unwrap().outcomes[0].outcome
                    == SoftwareExecutionOutcome::UnknownAfterDispatch
        );
        assert_eq!(fixture.calls(), 0);
    }
    let fixture = Fixture::new(
        ProcessStatus::Success,
        SoftwareInstalledState::Absent,
        usize::MAX,
    );
    std::fs::write(&fixture.tool.0, b"same path, different tool contents").unwrap();
    assert!(fixture.execute().is_err());
    assert_eq!(fixture.calls(), 0);
}

#[test]
fn msi_interruptions_and_crash_recovery_never_manufacture_removed_or_redispatch() {
    for status in [
        ProcessStatus::Timeout,
        ProcessStatus::Canceled,
        ProcessStatus::Exit { code: None },
    ] {
        let fixture = Fixture::new(status, SoftwareInstalledState::Absent, usize::MAX);
        let result = fixture.execute().unwrap();
        assert_eq!(
            result.outcomes[0].outcome,
            SoftwareExecutionOutcome::UnknownAfterDispatch
        );
    }
    for (point, expected_calls, expected) in [
        (
            CrashPoint::BeforeDispatchStarted,
            0,
            SoftwareExecutionOutcome::CanceledBeforeStart,
        ),
        (
            CrashPoint::AfterDispatchStarted,
            0,
            SoftwareExecutionOutcome::UnknownAfterDispatch,
        ),
        (
            CrashPoint::AfterAdapterCompleted,
            1,
            SoftwareExecutionOutcome::Removed,
        ),
    ] {
        let mut fixture = Fixture::new(
            ProcessStatus::Success,
            SoftwareInstalledState::Absent,
            usize::MAX,
        );
        fixture.executor.crash_point = point;
        assert!(fixture.execute().is_err());
        fixture.executor.crash_point = CrashPoint::None;
        let recovered = fixture.executor.recover_startup().unwrap();
        assert_eq!(recovered[0].outcome, expected);
        assert_eq!(fixture.calls(), expected_calls);
        assert!(fixture.executor.recover_startup().unwrap().is_empty());
    }
}

#[test]
fn msi_audit_failure_prevents_start_and_post_dispatch_failure_is_explicitly_unknown() {
    for terminal in [false, true] {
        let mut fixture = Fixture::new(
            ProcessStatus::Success,
            SoftwareInstalledState::Absent,
            usize::MAX,
        );
        fixture.executor.append_fault = super::super::audit::SupportAppendFault {
            fail_started: !terminal,
            fail_terminal: terminal,
        };
        let error = fixture.execute().unwrap_err();
        assert_eq!(fixture.calls(), usize::from(terminal));
        assert_eq!(
            matches!(error, SoftwareExecutionError::AuditAfterDispatch(_)),
            terminal
        );
    }
}

#[test]
fn msi_journal_rejects_forged_terminal_and_missing_evidence_keeps_old_wire() {
    let fixture = Fixture::new(
        ProcessStatus::Exit { code: Some(1605) },
        SoftwareInstalledState::Absent,
        usize::MAX,
    );
    fixture.execute().unwrap();
    let path = fixture.executor.audit_path.as_ref().unwrap();
    let original = std::fs::read_to_string(path).unwrap();
    let mut lines: Vec<serde_json::Value> = original
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let last = lines.last_mut().unwrap();
    last["transition"]["outcome"] = "removed".into();
    last["status_code"] = "removed".into();
    std::fs::write(
        path,
        lines
            .iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>(),
    )
    .unwrap();
    assert!(SoftwareAuditJournal::open(path).is_err());
    assert_eq!(fixture.calls(), 1);
    let legacy: SoftwareInventoryV1 = serde_json::from_str(include_str!(
        "../../../tests/fixtures/software/inventory-legacy.json"
    ))
    .unwrap();
    assert!(legacy.entries[0].msi_registration.is_none());
    assert_eq!(
        serde_json::to_value(&legacy).unwrap(),
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../tests/fixtures/software/inventory-legacy.json"
        ))
        .unwrap()
    );
}

#[test]
fn legacy_msi_removed_record_is_readable_but_cannot_authorize_new_leftovers() {
    use crate::software::{
        SoftwareLeftoverCertainty, SoftwareSupportAuditRecordV1, SoftwareSupportOutcomeCode,
    };
    let fixture = Fixture::new(
        ProcessStatus::Success,
        SoftwareInstalledState::Absent,
        usize::MAX,
    );
    let result = fixture.execute().unwrap();
    let path = fixture.executor.audit_path.as_ref().unwrap();
    let leftover = |operation| {
        SoftwareSupportAuditRecordV1::leftover_moved(
            operation,
            1,
            &result.outcomes[0].operation_id,
            &identity_value(),
            "fixture-candidate",
            SoftwareLeftoverCertainty::Certain,
            SoftwareSupportOutcomeCode::Succeeded,
            Some(0),
            None,
        )
    };
    {
        let mut journal = SoftwareAuditJournal::open(path).unwrap();
        journal.append_support(leftover("old-leftover")).unwrap();
    }
    let lines = std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| {
            let mut value: serde_json::Value = serde_json::from_str(line).unwrap();
            value.as_object_mut().unwrap().remove("msi");
            format!("{value}\n")
        })
        .collect::<String>();
    std::fs::write(path, lines).unwrap();
    let mut journal =
        SoftwareAuditJournal::open(path).expect("legacy MSI history remains readable");
    assert!(!journal.uninstall_removed(&result.outcomes[0].operation_id, &identity_value()));
    assert!(journal.append_support(leftover("new-leftover")).is_err());
}

struct FixedInventory(SoftwareInventoryV1);
impl LiveSoftwareInventory for FixedInventory {
    fn inventory(&self) -> Result<SoftwareInventoryV1, SoftwareExecutionError> {
        Ok(self.0.clone())
    }
}

#[test]
fn msi_identity_scope_sid_and_all_manual_restrictions_refuse_before_dispatch() {
    for case in 0..12 {
        let mut fixture = Fixture::new(
            ProcessStatus::Success,
            SoftwareInstalledState::Absent,
            usize::MAX,
        );
        let mut live = fixture.executor.live_inventory.inventory().unwrap();
        let entry = &mut live.entries[0];
        match case {
            0 => {
                if let SoftwareIdentity::Msi { product_code, .. } = &mut entry.identity {
                    product_code.push_str(" /i x");
                }
            }
            1 => {
                if let SoftwareIdentity::Msi { context, .. } = &mut entry.identity {
                    *context = MsiContext::Machine;
                }
            }
            2 => {
                if let SoftwareIdentity::Msi { context, .. } = &mut entry.identity {
                    *context = MsiContext::UserManaged;
                }
            }
            3 => entry.msi_registration.as_mut().unwrap().user_sid = "S-1-5-21-999".to_string(),
            4 => entry.msi_registration = None,
            5 => entry.scope = SoftwareScope::Machine,
            _ => {
                entry.eligibility.state = SoftwareEligibilityState::Manual;
                entry.eligibility.reason = [
                    SoftwareEligibilityReason::NoRemove,
                    SoftwareEligibilityReason::ProtectedProduct,
                    SoftwareEligibilityReason::SourceIncomplete,
                    SoftwareEligibilityReason::ConflictingIdentity,
                    SoftwareEligibilityReason::HiddenEntry,
                    SoftwareEligibilityReason::MsiRequiresAdministrator,
                ][case - 6];
            }
        }
        live.fingerprint = software_inventory_fingerprint(&live).unwrap();
        fixture.executor.live_inventory = Arc::new(FixedInventory(live));
        assert!(fixture.execute().is_err(), "case {case}");
        assert_eq!(fixture.calls(), 0);
    }
}

#[test]
fn msi_releases_cache_for_installer_maintenance_but_retains_tool_binding() {
    let fixture = Fixture::new(
        ProcessStatus::Success,
        SoftwareInstalledState::Absent,
        usize::MAX,
    );
    let cache = fixture._temp.path().join("cached-package.msi");
    std::fs::write(&cache, b"isolated cached package binding").unwrap();
    *fixture.registration.package_path.lock().unwrap() = Some(cache.clone());
    *fixture.process.maintain_cache.lock().unwrap() = Some(cache.clone());
    assert_eq!(
        fixture.execute().unwrap().outcomes[0].outcome,
        SoftwareExecutionOutcome::Removed
    );
    assert!(!cache.exists());
    std::fs::write(&fixture.tool.0, b"tool binding released after dispatch").unwrap();
}
