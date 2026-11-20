//! Result entry, explicit candidate selection and strict worker dispatch with no host side effects.
use super::*;
use devsweep_core::software::*;
use std::sync::atomic::AtomicBool;

#[derive(Default)]
pub(super) struct Fixture {
    stale: AtomicBool,
    fail: AtomicBool,
    skip: std::sync::Mutex<Option<SoftwareSupportErrorCode>>,
    calls: AtomicU32,
}

impl Fixture {
    pub(super) fn session() -> (Session, Arc<Self>) {
        let (mut session, _, id) = super::msix::Fixture::msi(SoftwareExecutionOutcome::Removed, 0);
        session.activate_mode_row(Mode::Software, id);
        apply_one(&mut session);
        assert!(session.host_confirm(true));
        apply_one(&mut session);
        let fixture = Arc::new(Self::default());
        session.host_executor = Some(fixture.clone());
        (session, fixture)
    }

    fn directory() -> SoftwareLeftoverCandidateV1 {
        SoftwareLeftoverCandidateV1 {
            id: "fixture-candidate".into(),
            path: r"C:\DevSweep Fixture\Application Cache".into(),
            origin: SoftwareLeftoverOrigin::LocalAppData,
            certainty: SoftwareLeftoverCertainty::Uncertain,
            selected_by_default: false,
            size: SoftwareSizeEvidence::Unknown {
                reason_code: "fixture".into(),
            },
        }
    }
}

impl ActionExecutor for Fixture {
    fn prepare(&self, intent: HostIntent) -> Result<Prepared, String> {
        let HostIntent::Leftovers(context) = &intent else {
            panic!("unexpected intent")
        };
        let dir = Self::directory();
        let (plan, digest) = if let Some(candidate) = &context.candidate {
            assert_eq!(**candidate, dir);
            let plan = SoftwareLeftoverPlanV1 {
                version: 1,
                software_id: context.id.clone(),
                identity: SoftwareIdentity::Msi {
                    product_code: "{12345678-1234-1234-1234-1234567890AB}".into(),
                    context: MsiContext::UserUnmanaged,
                },
                display_name: None,
                publisher: None,
                uninstall_operation_id: context.operation.clone(),
                selected_candidate_ids: vec![dir.id],
            };
            (Plan::Leftover(plan), format!("sha256:{}", "a".repeat(64)))
        } else {
            (Plan::LeftoverChoices(vec![dir]), String::new())
        };
        Ok(Prepared {
            intent,
            plan,
            digest,
        })
    }
    fn execute(&self, prepared: &Prepared, confirmed: bool) -> Result<ActionResult, String> {
        let (HostIntent::Leftovers(context), Plan::Leftover(plan)) =
            (&prepared.intent, &prepared.plan)
        else {
            panic!("unexpected dispatch")
        };
        let code =
            context.execute(
                plan,
                &prepared.digest,
                confirmed,
                &Arc::default(),
                |inventory, plan, digest, confirmed, cancel| {
                    assert!(confirmed && cancel.is_some());
                    assert!(inventory.entries.iter().any(
                        |entry| entry.id == plan.software_id && entry.identity == plan.identity
                    ));
                    assert_eq!(plan.uninstall_operation_id, "fixture-op");
                    if self.stale.load(Ordering::SeqCst)
                        || digest != format!("sha256:{}", "a".repeat(64))
                    {
                        return Err(SoftwareLeftoverError::DigestMismatch);
                    }
                    let skip = *self.skip.lock().unwrap();
                    let (outcome, error_code) = if let Some(reason) = skip {
                        (SoftwareSupportOutcomeCode::Skipped, Some(reason))
                    } else {
                        self.calls.fetch_add(1, Ordering::SeqCst);
                        if self.fail.load(Ordering::SeqCst) {
                            (
                                SoftwareSupportOutcomeCode::Failed,
                                Some(SoftwareSupportErrorCode::TrashFailed),
                            )
                        } else {
                            (SoftwareSupportOutcomeCode::Succeeded, None)
                        }
                    };
                    Ok(SoftwareLeftoverStrictReportV1 {
                        version: 1,
                        operation_id: Some("fixture-move".into()),
                        outcome: if self.fail.load(Ordering::SeqCst) {
                            SoftwareLeftoverStrictOutcome::Failed
                        } else {
                            SoftwareLeftoverStrictOutcome::Succeeded
                        },
                        error_code: None,
                        outcomes: vec![SoftwareLeftoverOutcomeV1 {
                            candidate_id: plan.selected_candidate_ids[0].clone(),
                            certainty: SoftwareLeftoverCertainty::Uncertain,
                            outcome,
                            estimated_bytes: None,
                            error_code,
                        }],
                        moved_known_bytes: 0,
                        lower_bound: false,
                    })
                },
            )?;
        Ok(ActionResult {
            intent: prepared.intent.clone(),
            code,
            reverse: None,
            uninstall: None,
        })
    }
}

pub(super) fn start_review(session: &mut Session) {
    assert!(session.review_leftovers(session.host_review.generation));
    apply_one(session);
    assert_eq!(session.host_review.phase, HostPhase::Choosing);
    assert!(!session.host_confirm(true));
    assert!(session.choose_leftover(session.host_review.generation, 0));
}

#[test]
fn leftovers_are_bound_to_exact_success_and_current_user() {
    let (mut session, _) = Fixture::session();
    let inventory = session.software_review.inventory().unwrap();
    let base = session.host_review.results[0].clone();
    assert!(host_review::Leftovers::after(&base, inventory).is_some());
    for change in 0..7 {
        let mut changed = base.clone();
        let item = changed.uninstall.as_mut().unwrap();
        match change {
            0 => {
                item.outcome = SoftwareExecutionOutcome::Failed;
            }
            1 => {
                item.outcome = SoftwareExecutionOutcome::RebootRequired;
            }
            2 => {
                item.msi
                    .as_mut()
                    .unwrap()
                    .process
                    .as_mut()
                    .unwrap()
                    .exit_code = Some(1641);
            }
            3 => {
                item.msi.as_mut().unwrap().user_sid = "S-1-5-21-200".into();
            }
            4 => {
                item.installed_state = Some(SoftwareInstalledState::Present);
            }
            5 => {
                item.software_id = "different-id".into();
            }
            _ => {
                changed.uninstall = None;
            }
        }
        assert!(host_review::Leftovers::after(&changed, inventory).is_none());
    }
    session.host_cancel();
    assert!(!session.review_leftovers(session.host_review.generation));
}

#[test]
fn leftovers_require_candidate_preview_confirmation_and_current_digest() {
    for (stale, fail, expected, calls) in [
        (false, false, "leftover_moved", 1),
        (true, false, "digest_mismatch", 0),
        (false, true, "leftover_failed", 1),
    ] {
        let (mut session, fixture) = Fixture::session();
        start_review(&mut session);
        apply_one(&mut session);
        session.apply_key(NavKey::Enter);
        assert!(!session.host_confirm(false));
        assert!(session.take_jobs().is_empty());
        fixture.stale.store(stale, Ordering::SeqCst);
        fixture.fail.store(fail, Ordering::SeqCst);
        assert!(session.host_confirm(true));
        assert!(!session.host_confirm(true));
        apply_one(&mut session);
        assert_eq!(session.host_review.results[0].code, expected);
        assert_eq!(fixture.calls.load(Ordering::SeqCst), calls);
    }
}

#[test]
fn completed_review_displays_the_actual_skipped_directory_and_reason() {
    for (reason, expected, en, zh) in [
        (
            SoftwareSupportErrorCode::Protected,
            "leftover_protected",
            "protected",
            "受保护",
        ),
        (
            SoftwareSupportErrorCode::NotPresent,
            "leftover_missing",
            "gone",
            "已不存在",
        ),
        (
            SoftwareSupportErrorCode::UnsafePath,
            "leftover_unsafe",
            "unsafe",
            "不安全",
        ),
    ] {
        let (mut session, fixture) = Fixture::session();
        start_review(&mut session);
        apply_one(&mut session);
        *fixture.skip.lock().unwrap() = Some(reason);
        assert!(session.host_confirm(true));
        apply_one(&mut session);
        assert_eq!(session.host_review.results[0].code, expected);
        assert_eq!(fixture.calls.load(Ordering::SeqCst), 0);
        for (locale, label) in [(Locale::En, en), (Locale::ZhCn, zh)] {
            assert!(
                session
                    .host_review
                    .lines(locale)
                    .iter()
                    .any(|line| line.contains(label))
            );
        }
    }
}
