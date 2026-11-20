//! Normal Software row → review → confirmed worker, using a private test adapter.
use super::*;
use devsweep_core::software::*;
use std::sync::Mutex;

pub(super) struct Fixture {
    inventory: Mutex<SoftwareInventoryV1>,
    calls: AtomicU32,
    outcome: SoftwareExecutionOutcome,
    msi_exit: Option<u32>,
}
impl Fixture {
    pub(super) fn session(outcome: SoftwareExecutionOutcome) -> (Session, Arc<Self>, String) {
        let inventory: SoftwareInventoryV1 = serde_json::from_str(include_str!(
            "../../../crates/devsweep-core/tests/fixtures/software/inventory-evidence.json"
        ))
        .unwrap();
        Self::with_inventory(inventory, outcome, None)
    }

    pub(super) fn msi(
        outcome: SoftwareExecutionOutcome,
        exit: u32,
    ) -> (Session, Arc<Self>, String) {
        let mut inventory: SoftwareInventoryV1 = serde_json::from_str(include_str!(
            "../../../crates/devsweep-core/tests/fixtures/software/inventory-evidence.json"
        ))
        .unwrap();
        let entry = &mut inventory.entries[0];
        let product_code = "{12345678-1234-1234-1234-1234567890AB}";
        entry.identity = SoftwareIdentity::Msi {
            product_code: product_code.into(),
            context: MsiContext::UserUnmanaged,
        };
        entry.id = software_entry_id(&entry.identity).unwrap();
        entry.display_name = Some("Windows Installer Fixture".into());
        entry.provenance = vec![SoftwareSourceId::Msi {
            context: MsiContext::UserUnmanaged,
        }];
        entry.eligibility.reason = SoftwareEligibilityReason::EligibleCurrentUserMsi;
        entry.msi_registration = Some(MsiRegistrationEvidenceV1 {
            user_sid: "S-1-5-21-100".into(),
            package_code: product_code.into(),
            registration_digest: format!("sha256:{}", "a".repeat(64)),
            tool_digest: format!("sha256:{}", "b".repeat(64)),
        });
        crate::software_review::seal_inventory(&mut inventory).unwrap();
        Self::with_inventory(inventory, outcome, Some(exit))
    }

    fn with_inventory(
        inventory: SoftwareInventoryV1,
        outcome: SoftwareExecutionOutcome,
        msi_exit: Option<u32>,
    ) -> (Session, Arc<Self>, String) {
        let id = inventory.entries[0].id.clone();
        let fixture = Arc::new(Self {
            inventory: Mutex::new(inventory.clone()),
            calls: AtomicU32::new(0),
            outcome,
            msi_exit,
        });
        let mut session = Session::unbacked();
        session.show_mode(Mode::Software);
        session.host_executor = Some(fixture.clone());
        session.software_review.apply_host_load(
            crate::software_review::SoftwareHostLoad::Inventory(Ok(inventory)),
            Locale::En,
        );
        session.sync_software_rows();
        (session, fixture, id)
    }
}

impl ActionExecutor for Fixture {
    fn prepare(&self, intent: HostIntent) -> Result<Prepared, String> {
        let HostIntent::Uninstall(row) = &intent else {
            panic!("unexpected action")
        };
        let live = self.inventory.lock().unwrap();
        let (plan, digest) = row.preview(&live, live.observed_at_unix_ms)?;
        Ok(Prepared {
            intent,
            plan: Plan::Uninstall(plan),
            digest,
        })
    }
    fn execute(&self, prepared: &Prepared, confirmed: bool) -> Result<ActionResult, String> {
        let (HostIntent::Uninstall(row), Plan::Uninstall(plan)) =
            (&prepared.intent, &prepared.plan)
        else {
            panic!("unexpected action")
        };
        row.execute(
            plan,
            &prepared.digest,
            confirmed,
            Arc::default(),
            |request| {
                assert!(request.confirmed && request.cancel.is_some());
                let live = self.inventory.lock().unwrap();
                let preview =
                    preview_selection_plan_live(request.plan, &live, live.observed_at_unix_ms)?;
                validate_preview_digest(&preview, request.expected_preview_digest)?;
                self.calls.fetch_add(1, Ordering::SeqCst);
                Ok(SoftwareExecutionReportV1 {
                    version: 1,
                    irreversible: true,
                    outcomes: vec![SoftwareActionOutcomeV1 {
                        operation_id: "fixture-op".into(),
                        software_id: row.id.clone(),
                        irreversible: true,
                        msi: self.msi_exit.map(|exit_code| MsiExecutionEvidenceV1 {
                            user_sid: live.entries[0]
                                .msi_registration
                                .as_ref()
                                .unwrap()
                                .user_sid
                                .clone(),
                            process: Some(MsiProcessEvidenceV1 {
                                exit_code: Some(exit_code),
                                interruption: None,
                            }),
                        }),
                        outcome: self.outcome,
                        installed_state: Some(
                            if self.outcome == SoftwareExecutionOutcome::Removed {
                                SoftwareInstalledState::Absent
                            } else {
                                SoftwareInstalledState::Present
                            },
                        ),
                        reboot_evidence: SoftwareRebootEvidence::None,
                        error_code: None,
                    }],
                })
            },
        )
    }
}

#[test]
fn msix_requires_the_confirm_control_and_retains_the_exact_requery_result() {
    for (locale, outcome, expected) in [
        (
            Locale::En,
            SoftwareExecutionOutcome::Removed,
            "Windows no longer lists",
        ),
        (
            Locale::ZhCn,
            SoftwareExecutionOutcome::Failed,
            "Windows 仍列出",
        ),
    ] {
        let (mut session, fixture, id) = Fixture::session(outcome);
        session.set_locale(locale);
        session.activate_mode_row(Mode::Software, id.clone());
        apply_one(&mut session);
        let lines = session.host_review_lines().join(" ");
        assert!(
            lines.contains("EvidenceFixture_1.0.0.0_x64__publisher")
                && lines.contains(crate::copy::pick(locale, "cannot be undone", "无法撤销"))
        );
        assert!(!session.host_confirm(false));
        session.apply_key(NavKey::Enter);
        assert!(session.take_jobs().is_empty());
        session.activate_mode_row(Mode::Software, id);
        apply_one(&mut session);
        assert_eq!(fixture.calls.load(Ordering::SeqCst), 0);
        assert!(session.host_confirm(true));
        assert!(!session.host_confirm(true));
        apply_one(&mut session);
        assert_eq!(fixture.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            session.host_review.results[0]
                .uninstall
                .as_ref()
                .unwrap()
                .outcome,
            outcome
        );
        assert!(session.host_review_lines().join(" ").contains(expected));
    }
}

#[test]
fn msix_changed_identity_digest_or_eligibility_never_reaches_removal() {
    for change in 0..3 {
        let (mut session, fixture, id) = Fixture::session(SoftwareExecutionOutcome::Removed);
        session.activate_mode_row(Mode::Software, id);
        apply_one(&mut session);
        {
            let mut inventory = fixture.inventory.lock().unwrap();
            match change {
                0 => inventory.entries.clear(),
                1 => inventory.entries[0].version = Some("2.0.0.0".into()),
                _ => {
                    inventory.entries[0].eligibility = SoftwareEligibility {
                        state: SoftwareEligibilityState::Manual,
                        reason: SoftwareEligibilityReason::DependencyPackage,
                    }
                }
            }
            crate::software_review::seal_inventory(&mut inventory).unwrap();
        }
        assert!(session.host_confirm(true));
        apply_one(&mut session);
        assert!(matches!(
            session.host_review.results[0].code.as_str(),
            "stale_confirmation" | "software_not_eligible"
        ));
        assert_eq!(fixture.calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn msix_late_preview_refresh_and_navigation_do_not_restore_confirmation() {
    let (mut session, fixture, id) = Fixture::session(SoftwareExecutionOutcome::Removed);
    session.activate_mode_row(Mode::Software, id.clone());
    let old = session.take_jobs().pop().unwrap().run();
    session.show_mode(Mode::Clean);
    assert!(!session.apply_job(old));
    session.show_mode(Mode::Software);
    assert!(!session.host_confirm(true));
    session.activate_mode_row(Mode::Software, id);
    apply_one(&mut session);
    session.software_refresh();
    assert!(!session.host_confirm(true));
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn msix_refresh_menu_settings_and_close_discard_pending_authority() {
    for change in 0..4 {
        let (mut session, fixture, id) = Fixture::session(SoftwareExecutionOutcome::Removed);
        session.activate_mode_row(Mode::Software, id);
        apply_one(&mut session);
        match change {
            0 => session.software_refresh(),
            1 => session.toggle_menu(),
            2 => session.open_settings(),
            _ => {
                session.apply_key(NavKey::Escape);
            }
        }
        assert!(!session.host_confirm(true));
        assert_eq!(fixture.calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn msix_fixture_and_batch_cannot_authorize_a_host_uninstall() {
    let (mut session, fixture, id) = Fixture::session(SoftwareExecutionOutcome::Removed);
    let intent = session.software_review.host_intent(&id).unwrap().unwrap();
    assert!(!session.host_preview(vec![intent.clone(), intent]));
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../crates/devsweep-core/tests/fixtures/software/inventory-evidence.json");
    session
        .software_review
        .open_inventory_file(&path, Locale::En);
    assert_eq!(
        session.software_review.host_intent(&id).unwrap_err(),
        "fixture_read_only"
    );
    session.activate_mode_row(Mode::Software, id);
    assert!(session.take_jobs().is_empty());
    assert!(!session.host_confirm(true));
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn msix_delayed_preview_cannot_unlock_a_running_removal() {
    let (mut session, fixture, id) = Fixture::session(SoftwareExecutionOutcome::Removed);
    session.activate_mode_row(Mode::Software, id);
    apply_one(&mut session);
    let preview_generation = session.host_review.generation;
    assert!(session.host_confirm(true));
    assert!(!session.apply_job(JobResult::HostPreview {
        generation: preview_generation,
        result: Err("late_preview".into()),
    }));
    assert!(session.host_action_busy());
    assert!(!session.host_confirm(true));
    apply_one(&mut session);
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 1);
    assert!(!session.host_action_busy());
    assert_eq!(session.host_review.results[0].code, "removed");
}

#[test]
fn msi_requires_a_fresh_review_and_preserves_typed_exit_evidence() {
    for (exit, outcome) in [
        (0, SoftwareExecutionOutcome::Removed),
        (3010, SoftwareExecutionOutcome::RebootRequired),
        (1641, SoftwareExecutionOutcome::RebootRequired),
        (1603, SoftwareExecutionOutcome::Failed),
    ] {
        let (mut session, fixture, id) = Fixture::msi(outcome, exit);
        session.activate_mode_row(Mode::Software, &id);
        apply_one(&mut session);
        assert!(
            session
                .host_review_lines()
                .join(" ")
                .contains("{12345678-1234-1234-1234-1234567890AB}")
        );
        session.apply_key(NavKey::Enter);
        assert!(!session.host_confirm(false));
        session.activate_mode_row(Mode::Software, &id);
        apply_one(&mut session);
        assert_eq!(fixture.calls.load(Ordering::SeqCst), 0);
        assert!(session.host_confirm(true));
        assert!(!session.host_confirm(true));
        apply_one(&mut session);
        assert_eq!(fixture.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            session.host_review.results[0]
                .uninstall
                .as_ref()
                .unwrap()
                .outcome,
            outcome
        );
        assert!(
            session
                .host_review_lines()
                .join(" ")
                .contains(&exit.to_string())
        );
    }
}

#[test]
fn msi_rejects_changed_sid_registration_and_refused_context_before_dispatch() {
    for change in 0..4 {
        let (mut session, fixture, id) = Fixture::msi(SoftwareExecutionOutcome::Removed, 0);
        session.activate_mode_row(Mode::Software, &id);
        apply_one(&mut session);
        {
            let mut inventory = fixture.inventory.lock().unwrap();
            let entry = &mut inventory.entries[0];
            match change {
                0 => entry.msi_registration.as_mut().unwrap().user_sid = "S-1-5-21-200".into(),
                1 => {
                    entry.msi_registration.as_mut().unwrap().tool_digest =
                        format!("sha256:{}", "c".repeat(64))
                }
                2 => entry.msi_registration = None,
                _ => {
                    entry.eligibility = SoftwareEligibility {
                        state: SoftwareEligibilityState::Manual,
                        reason: SoftwareEligibilityReason::MsiRequiresAdministrator,
                    }
                }
            }
            crate::software_review::seal_inventory(&mut inventory).unwrap();
        }
        assert!(session.host_confirm(true));
        apply_one(&mut session);
        assert_ne!(session.host_review.results[0].code, "removed");
        assert_eq!(fixture.calls.load(Ordering::SeqCst), 0);
    }
}
