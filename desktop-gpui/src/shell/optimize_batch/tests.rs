use super::*;
use devsweep_core::optimize::{BatchOutcome, BatchSkipReason, PrecheckReason};

#[test]
fn desktop_batch_selection_preview_confirmation_and_history_use_the_same_executor() {
    let dir = std::env::temp_dir().join(format!(
        "devsweep-optimize-batch-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("optimize.jsonl");
    let mut session = Session::unbacked();
    session.optimize_batch.executor =
        Some(Arc::new(MaintenanceExecutor::isolated_batch_fixture(&path)));
    session.show_mode(Mode::Optimize);
    assert!(session.batch().selected.is_empty());
    session.toggle_batch_item("settings.search");
    session.toggle_batch_item("guidance.drive_optimize");
    assert!(session.batch().selected.is_empty());
    let ids = ["maintenance.clock_probe", "maintenance.retrim", "dns.flush"];
    for id in ids {
        session.toggle_batch_item(id);
    }
    session.review_batch();
    let old = session.take_jobs().pop().unwrap().run();
    session.toggle_batch_item("dns.flush");
    assert!(!session.apply_job(old));
    session.toggle_batch_item("dns.flush");
    session.review_batch();
    let review = session.take_jobs().pop().unwrap().run();
    assert!(session.apply_job(review));
    let previous = session.batch_generation();
    assert_eq!(
        session.batch().review.as_ref().unwrap().0.operations.len(),
        3
    );
    session.open_settings();
    session.confirm_batch(previous);
    assert!(session.take_jobs().is_empty());
    assert!(!session.batch().reviewed());
    session.show_mode(Mode::Clean);
    session.show_mode(Mode::Optimize);
    session.confirm_batch(previous);
    assert!(session.take_jobs().is_empty());
    session.review_batch();
    let review = session.take_jobs().pop().unwrap().run();
    assert!(session.apply_job(review));
    let generation = session.batch_generation();
    session.confirm_batch(generation);
    session.confirm_batch(generation);
    let mut jobs = session.take_jobs();
    assert_eq!(jobs.len(), 1);
    assert!(session.batch().running());
    session.cancel_batch(previous);
    session.toggle_batch_item("dns.flush");
    assert_eq!(session.batch().selected.len(), 3);
    assert!(session.apply_job(jobs.pop().unwrap().run()));
    let report = session.batch().report.as_ref().unwrap();
    assert_eq!(report.outcomes.len(), 3);
    assert!(matches!(
        report.outcomes[0].outcome,
        BatchOutcome::Failed { .. }
    ));
    assert_eq!(
        report.outcomes[1].outcome,
        BatchOutcome::Skipped {
            reason: BatchSkipReason::Precheck(PrecheckReason::UacDeclined)
        }
    );
    assert_eq!(report.outcomes[2].outcome, BatchOutcome::Succeeded);
    let history = std::fs::read_to_string(&path).unwrap();
    assert!(
        report
            .outcomes
            .iter()
            .all(|item| history.contains(&item.operation_id))
    );
    assert!(history.lines().all(|line| line.contains(&report.batch_id)));
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(dir.join("optimize.lock")).unwrap();
    std::fs::remove_dir(dir).unwrap();
}

#[test]
fn unbacked_and_clean_fixture_sessions_cannot_run_maintenance() {
    let mut session = Session::unbacked();
    session.show_mode(Mode::Optimize);
    session.toggle_batch_item("dns.flush");
    session.review_batch();
    assert!(session.take_jobs().is_empty());
    assert!(session.install_clean_review_fixture("en:review"));
    session.review_batch();
    assert!(session.take_jobs().is_empty());
}
