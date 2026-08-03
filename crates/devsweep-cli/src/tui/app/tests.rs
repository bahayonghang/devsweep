use std::{path::PathBuf, time::SystemTime};

use crossterm::event::KeyCode;

use super::*;
use crate::inventory::{
    CapacityObservation, INVENTORY_REPORT_VERSION, InventoryClassification, InventoryReport,
};
use crate::model::{
    CLEANUP_PLAN_VERSION, Ecosystem, Evidence, ScanCompleteness, ScanDiagnostic,
    ScanDiagnosticOutcome, ScanDiagnosticStage, ScanHealth, TargetKind,
};
use crate::tui::test_support::{key, plan_with_targets, render_text, representative_plan, target};

#[test]
fn update_handles_scan_selection_details_filter_help_and_quit() {
    let mut app = App::with_plan(representative_plan());

    let effects = app.update(key(KeyCode::Char('s')));
    assert!(matches!(effects.as_slice(), [Effect::StartScan { .. }]));
    assert_eq!(app.jobs.len(), 1);

    let first_id = app.targets[0].id.clone();
    assert!(app.selected_ids.contains(&first_id));
    app.update(key(KeyCode::Char(' ')));
    assert!(!app.selected_ids.contains(&first_id));

    app.update(key(KeyCode::Down));
    app.update(key(KeyCode::Enter));
    assert!(matches!(app.overlay, Overlay::Details));
    app.update(key(KeyCode::Esc));
    assert!(matches!(app.overlay, Overlay::None));

    app.update(key(KeyCode::Char('/')));
    assert!(app.filter_active);
    app.update(key(KeyCode::Char('n')));
    app.update(key(KeyCode::Char('p')));
    app.update(key(KeyCode::Enter));
    assert_eq!(app.filter, "np");
    assert!(!app.filter_active);

    app.update(key(KeyCode::Char('?')));
    assert!(matches!(app.overlay, Overlay::Help));
    app.update(key(KeyCode::Esc));

    // Finish the earlier scan job so quit is not blocked by D9.
    app.update(UiEvent::Worker(WorkerEvent::ScanFinished {
        job_id: 1,
        plan: representative_plan(),
        health: ScanHealth::complete(),
    }));
    app.update(key(KeyCode::Char('q')));
    assert!(app.should_quit);
}

#[test]
fn inventory_report_is_display_only_and_cannot_change_cleanup_selection() {
    let mut app = App::with_plan(representative_plan());
    let targets_before = app.targets.clone();
    let selected_before = app.selected_ids.clone();
    let report = InventoryReport {
        version: INVENTORY_REPORT_VERSION,
        root: PathBuf::from("C:/inventory-root"),
        observations: vec![
            CapacityObservation {
                path: PathBuf::from("C:/inventory-root/archive"),
                classification: InventoryClassification::InventoryOnly,
                estimated_bytes: 4096,
                size_complete: true,
                warnings: Vec::new(),
            },
            CapacityObservation {
                path: PathBuf::from("C:/inventory-root/old-store"),
                classification: InventoryClassification::InventoryOnly,
                estimated_bytes: 1024,
                size_complete: false,
                warnings: Vec::new(),
            },
        ],
        health: ScanHealth::complete(),
        orphan_pnpm_store: None,
    };
    let job_id = app.start_job(JobKind::Inventory, "Inventory fixture");

    app.update(UiEvent::Worker(WorkerEvent::InventoryFinished {
        job_id,
        report: Box::new(report.clone()),
    }));
    let effects = app.update(key(KeyCode::Char('6')));
    assert!(effects.is_empty());
    app.update(key(KeyCode::Down));
    app.update(key(KeyCode::Char(' ')));
    app.update(key(KeyCode::Char('c')));

    assert_eq!(app.active_tab, ActiveTab::Inventory);
    assert_eq!(app.inventory_report.as_ref(), Some(&report));
    assert_eq!(app.inventory_selected_index, 1);
    assert_eq!(app.targets, targets_before);
    assert_eq!(app.selected_ids, selected_before);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn startup_effects_request_an_initial_scan() {
    let mut app = App::new();

    let effects = app.startup_effects();

    let [Effect::StartScan { job_id }] = effects.as_slice() else {
        panic!("startup requests one scan");
    };
    assert_eq!(*job_id, 1);
    assert_eq!(app.jobs.len(), 1);
    assert_eq!(app.jobs[0].label, "Scan current directory and globals");
    assert_eq!(
        app.logs.last().map(|entry| entry.message.as_str()),
        Some("Startup scan requested")
    );
}

#[test]
fn clean_confirmation_copy_differs_by_action_strength() {
    let mut app = App::with_plan(representative_plan());
    app.selected_ids.clear();
    app.selected_ids.insert(app.targets[0].id.clone());

    app.update(key(KeyCode::Char('c')));
    let Overlay::Confirm(confirm) = &app.overlay else {
        panic!("trash selection opens confirm");
    };
    assert_eq!(confirm.required_phrase, "confirm");
    assert!(confirm.message.contains("Trash"));
    assert!(!confirm.has_irreversible_commands);

    app.overlay = Overlay::None;
    app.selected_ids.clear();
    app.selected_ids.insert(app.targets[1].id.clone());

    app.update(key(KeyCode::Char('c')));
    let Overlay::Confirm(confirm) = &app.overlay else {
        panic!("command selection opens confirm");
    };
    assert_eq!(confirm.required_phrase, "confirm");
    assert!(confirm.message.contains("irreversible"));
    assert!(confirm.has_irreversible_commands);
    assert_eq!(confirm.command_previews.len(), 1);
    assert_eq!(
        confirm.command_previews[0].command,
        "argv: npm cache clean --force"
    );
}

#[test]
fn inspect_only_targets_are_not_selectable_for_cleanup() {
    let mut app = App::with_plan(representative_plan());
    app.selected_ids.clear();
    app.selected_index = 2;

    app.update(key(KeyCode::Char(' ')));

    let inspect_id = app.targets[2].id.clone();
    assert!(!app.selected_ids.contains(&inspect_id));
    assert!(
        app.logs.iter().any(|entry| {
            entry.message == "Inspect-only targets cannot be selected for cleanup"
        })
    );

    app.selected_ids.insert(inspect_id);
    app.update(key(KeyCode::Char('c')));
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn pycache_groups_collapse_without_changing_target_selection_or_identity() {
    let project_root = PathBuf::from("C:/workspace/python-project");
    let paths = [
        project_root.join("package_a/__pycache__"),
        project_root.join("package_b/__pycache__"),
        project_root.join("package_c/__pycache__"),
    ];
    let targets = paths
        .iter()
        .map(|path| {
            target(
                "python.__pycache__",
                Scope::Project {
                    root: project_root.clone(),
                },
                Ecosystem::Python,
                TargetKind::TestCache,
                Some(path.clone()),
                128,
                RiskLevel::Low,
                true,
                true,
                CleanAction::MoveToTrash { path: path.clone() },
            )
        })
        .collect();
    let mut app = App::with_plan(plan_with_targets(targets));
    let target_ids: Vec<TargetId> = app.targets.iter().map(|target| target.id.clone()).collect();

    let rows = app.visible_target_rows();
    assert_eq!(rows.len(), 1);
    assert!(matches!(rows[0], TargetListRow::PycacheGroup(_)));
    assert!(app.selected_target().is_none());
    assert_eq!(
        app.selected_pycache_group()
            .expect("collapsed group is selected")
            .target_indices
            .len(),
        3
    );

    app.update(key(KeyCode::Char(' ')));
    assert!(
        target_ids
            .iter()
            .all(|target_id| !app.selected_ids.contains(target_id))
    );
    app.update(key(KeyCode::Char(' ')));
    assert!(
        target_ids
            .iter()
            .all(|target_id| app.selected_ids.contains(target_id))
    );

    app.update(key(KeyCode::Enter));
    assert_eq!(app.visible_target_rows().len(), 3);
    assert!(app.selected_target().is_some());
    app.update(key(KeyCode::Char('g')));
    assert_eq!(app.visible_target_rows().len(), 1);

    app.filter = "package_b".to_string();
    let rows = app.visible_target_rows();
    assert_eq!(rows.len(), 1);
    assert!(matches!(rows[0], TargetListRow::Target(_)));
    assert!(
        app.selected_target()
            .and_then(|target| target.path.as_ref())
            .is_some_and(|path| path.ends_with("package_b/__pycache__"))
    );
}

#[test]
fn startup_default_selection_follows_selected_by_default_even_for_self_targets() {
    let current_exe = std::env::current_exe().expect("current executable path");
    let current_exe_dir = current_exe.parent().expect("current executable has parent");
    let safe_path = PathBuf::from("D:/code/web/.next/cache");
    let plan = CleanupPlan {
        version: CLEANUP_PLAN_VERSION,
        targets: vec![
            target(
                "rust.target",
                Scope::Project {
                    root: current_exe_dir.to_path_buf(),
                },
                Ecosystem::Rust,
                TargetKind::BuildArtifacts,
                Some(current_exe_dir.to_path_buf()),
                4096,
                RiskLevel::Low,
                true,
                false,
                CleanAction::Command {
                    program: "cargo".to_string(),
                    args: vec!["clean".to_string()],
                    cwd: None,
                    irreversible: true,
                },
            ),
            target(
                "node.next_cache",
                Scope::Project {
                    root: PathBuf::from("D:/code/web"),
                },
                Ecosystem::Node,
                TargetKind::BuildArtifacts,
                Some(safe_path.clone()),
                1024,
                RiskLevel::Low,
                true,
                true,
                CleanAction::MoveToTrash { path: safe_path },
            ),
        ],
    };

    let app = App::with_plan(plan);

    // The self-clean guard is enforced by the executor at run time; the UI
    // default selection is a pure projection of `selected_by_default`.
    assert!(
        app.targets
            .iter()
            .find(|target| target.id.as_str().starts_with("rust.target"))
            .is_some_and(|target| app.selected_ids.contains(&target.id))
    );
    assert!(
        app.targets
            .iter()
            .find(|target| target.id.as_str().starts_with("node.next_cache"))
            .is_some_and(|target| app.selected_ids.contains(&target.id))
    );
}

#[test]
fn accepted_confirmation_emits_selected_clean_plan() {
    let mut app = App::with_plan(representative_plan());
    app.selected_ids.clear();
    app.selected_ids.insert(app.targets[0].id.clone());

    app.update(key(KeyCode::Char('c')));
    let confirmation_digest_prefix = match &app.overlay {
        Overlay::Confirm(confirm) => confirm.plan_digest_prefix.clone(),
        _ => panic!("confirmation opens with a digest"),
    };
    for ch in "confirm".chars() {
        app.update(key(KeyCode::Char(ch)));
    }
    let effects = app.update(key(KeyCode::Enter));

    let [
        Effect::StartClean {
            job_id,
            plan,
            selected,
            plan_digest,
        },
    ] = effects.as_slice()
    else {
        panic!("confirmation emits clean effect");
    };
    assert_eq!(*job_id, 1);
    assert_eq!(plan.targets.len(), 1);
    assert_eq!(plan.targets[0].id, app.targets[0].id);
    assert_eq!(selected, &vec![app.targets[0].id.clone()]);
    assert_eq!(
        plan_digest,
        crate::plan::validate_scanned_plan(plan)
            .expect("frozen plan validates")
            .digest()
    );
    assert_eq!(
        confirmation_digest_prefix,
        plan_digest[..12],
        "the confirmation prefix belongs to the exact plan sent to the worker"
    );
    assert!(matches!(app.overlay, Overlay::None));
    let progress = app
        .cleanup_progress
        .as_ref()
        .expect("accepted confirmation initializes progress");
    assert_eq!(progress.job_id, *job_id);
    assert_eq!(progress.completed, 0);
    assert_eq!(progress.total, 1);
    assert_eq!(progress.summary, "Executing selected cleanup plan");
    assert!(!progress.finished);
    assert_eq!(progress.items.len(), 1);
    assert_eq!(progress.items[0].target_id, app.targets[0].id);
    assert_eq!(progress.items[0].status, CleanupItemStatus::Pending);
}

#[test]
fn accepted_confirmation_keeps_explicit_fresh_target_selection() {
    let mut target = representative_plan().targets[0].clone();
    target.last_modified = Some(SystemTime::now());
    target.selected_by_default = false;
    target.evidence.push(Evidence::RuleMatched {
        rule_id: "ranking.freshness_guard.7d".to_string(),
    });
    let mut app = App::with_plan(plan_with_targets(vec![target.clone()]));
    app.selected_ids.insert(target.id.clone());

    app.update(key(KeyCode::Char('c')));
    for ch in "confirm".chars() {
        app.update(key(KeyCode::Char(ch)));
    }
    let effects = app.update(key(KeyCode::Enter));

    let [Effect::StartClean { plan, selected, .. }] = effects.as_slice() else {
        panic!("confirmation emits clean effect");
    };
    assert_eq!(plan.targets.len(), 1);
    assert_eq!(plan.targets[0].id, target.id);
    assert_eq!(selected, &vec![target.id.clone()]);
    assert!(
        !plan.targets[0].selected_by_default,
        "explicit selection must not rewrite the ranking hint"
    );
}

#[test]
fn rejected_confirmation_shows_inline_feedback() {
    let mut app = App::with_plan(representative_plan());
    app.selected_ids.clear();
    app.selected_ids.insert(app.targets[1].id.clone());

    app.update(key(KeyCode::Char('c')));
    let effects = app.update(key(KeyCode::Enter));

    assert!(effects.is_empty());
    let Overlay::Confirm(confirm) = &app.overlay else {
        panic!("invalid confirmation keeps confirm overlay open");
    };
    assert_eq!(
        confirm.feedback.as_deref(),
        Some("Type confirm before pressing Enter.")
    );
    let rendered = render_text(&app);
    assert!(rendered.contains("Type confirm before pressing Enter."));
    assert!(rendered.contains("Input:"));
}

#[test]
fn finished_cleanup_progress_stays_visible_until_dismissed() {
    let mut app = App::with_plan(representative_plan());
    let job_id = app.start_job(JobKind::Clean, "Clean fixture");
    let target_id = app.targets[1].id.clone();
    app.cleanup_progress = Some(CleanupProgress {
        job_id,
        completed: 0,
        total: 1,
        summary: "Executing selected cleanup plan".to_string(),
        finished: false,
        items: vec![CleanupProgressItem {
            target_id: target_id.clone(),
            label: "npm cache clean".to_string(),
            status: CleanupItemStatus::Pending,
            detail: None,
        }],
    });

    app.update(UiEvent::Worker(WorkerEvent::CleanProgress {
        job_id,
        target_id,
        status: ExecutionTargetStatus::Succeeded,
        message: "npm cache clean".to_string(),
        detail: "completed".to_string(),
        completed: 0,
        total: 1,
    }));
    app.update(UiEvent::Worker(WorkerEvent::CleanFinished {
        job_id,
        report: ExecutionReport {
            dry_run: false,
            selected: 1,
            attempted: 1,
            succeeded: 1,
            failed: 0,
            skipped: 0,
            failures: Vec::new(),
            audit_log: Some(PathBuf::from("audit.jsonl")),
        },
    }));

    let progress = app
        .cleanup_progress
        .as_ref()
        .expect("finished cleanup result remains visible");
    assert!(progress.finished);
    assert_eq!(progress.completed, 1);
    assert_eq!(progress.total, 1);
    let rendered = render_text(&app);
    assert!(rendered.contains("Cleanup progress"));
    assert!(rendered.contains("1 / 1 finished: 1 succeeded, 0 failed, 0 skipped"));
    assert!(rendered.contains("Enter/Esc close"));

    app.update(key(KeyCode::Enter));

    assert!(app.cleanup_progress.is_none());
    assert!(!app.should_quit);
}

#[test]
fn worker_events_update_jobs_and_targets() {
    let mut app = App::new();
    let effects = app.update(key(KeyCode::Char('s')));
    let [Effect::StartScan { job_id }] = effects.as_slice() else {
        panic!("scan key starts scan");
    };
    let job_id = *job_id;

    app.update(UiEvent::Worker(WorkerEvent::ScanFinished {
        job_id,
        plan: representative_plan(),
        health: ScanHealth::complete(),
    }));

    assert_eq!(app.targets.len(), 3);
    assert_eq!(app.jobs[0].status, JobStatus::Succeeded);
    assert!(
        app.targets
            .iter()
            .find(|target| target.id.as_str().starts_with("node.next_cache"))
            .is_some_and(|target| app.selected_ids.contains(&target.id))
    );

    let report = ExecutionReport {
        dry_run: false,
        selected: 1,
        attempted: 1,
        succeeded: 1,
        failed: 0,
        skipped: 0,
        failures: Vec::new(),
        audit_log: Some(PathBuf::from("audit.jsonl")),
    };
    let clean_job_id = app.start_job(JobKind::Clean, "Clean fixture");
    app.update(UiEvent::Worker(WorkerEvent::CleanFinished {
        job_id: clean_job_id,
        report,
    }));
    assert!(
        app.logs
            .iter()
            .any(|entry| entry.message.contains("audit.jsonl"))
    );
}

#[test]
fn completed_scan_keeps_partial_health_and_capacity_totals_in_state() {
    let mut app = App::new();
    let effects = app.startup_effects();
    let [Effect::StartScan { job_id }] = effects.as_slice() else {
        panic!("startup requests a scan");
    };
    let job_id = *job_id;
    let mut plan = representative_plan();
    plan.targets[1].size_complete = false;
    plan.targets[2].size_complete = false;
    let mut health = ScanHealth::new(
        ScanCompleteness::Partial,
        vec![ScanDiagnostic {
            stage: ScanDiagnosticStage::CargoMetadata,
            path: PathBuf::from("C:/workspace/python-project/Cargo.toml"),
            outcome: ScanDiagnosticOutcome::OutputTruncated,
            detail: "captured cargo metadata output was truncated".to_string(),
            process: None,
        }],
    );
    health.totals = ScanTotals::from_cleanup_plan(&plan);

    app.update(UiEvent::Worker(WorkerEvent::ScanFinished {
        job_id,
        plan,
        health: health.clone(),
    }));

    assert_eq!(app.scan_health, health);
    assert_eq!(app.scan_health.totals.verified_bytes, 1024);
    assert_eq!(app.scan_health.totals.partial_lower_bound_bytes, 2048);
    assert_eq!(app.scan_health.totals.unknown_target_count, 1);
    assert!(app.logs.iter().any(|entry| {
        entry.level == AppLogLevel::Warning && entry.message.contains("partial health")
    }));
}

#[test]
fn scan_progress_shows_project_targets_before_global_scan_finishes() {
    let mut app = App::new();
    let effects = app.startup_effects();
    let [Effect::StartScan { job_id }] = effects.as_slice() else {
        panic!("startup requests scan");
    };
    let job_id = *job_id;
    let project_plan = plan_with_targets(vec![representative_plan().targets[0].clone()]);

    app.update(UiEvent::Worker(WorkerEvent::ScanStarted { job_id }));
    app.update(UiEvent::Worker(WorkerEvent::ScanProgress {
        job_id,
        phase: ScanPhase::Projects,
        message: "Project scan finished: 1 target(s)".to_string(),
        plan: Some(project_plan),
    }));

    assert_eq!(app.targets.len(), 1);
    assert!(matches!(app.targets[0].scope, Scope::Project { .. }));
    assert!(app.selected_ids.contains(&app.targets[0].id));
    assert_eq!(app.jobs[0].status, JobStatus::Running);
    assert_eq!(app.jobs[0].progress, "Project scan finished: 1 target(s)");

    app.active_tab = ActiveTab::JobsLogs;
    let rendered = render_text(&app);
    assert!(rendered.contains("Project scan finished: 1 target(s)"));
}

#[test]
fn scan_progress_applies_latest_cumulative_partial_plan() {
    let mut app = App::new();
    let effects = app.startup_effects();
    let [Effect::StartScan { job_id }] = effects.as_slice() else {
        panic!("startup requests scan");
    };
    let job_id = *job_id;
    let representative = representative_plan();
    let project_target = representative.targets[0].clone();
    let global_target = representative.targets[1].clone();

    app.update(UiEvent::Worker(WorkerEvent::ScanStarted { job_id }));
    app.update(UiEvent::Worker(WorkerEvent::ScanProgress {
        job_id,
        phase: ScanPhase::Projects,
        message: "Project scan finished: 1 target(s)".to_string(),
        plan: Some(plan_with_targets(vec![project_target.clone()])),
    }));
    app.update(UiEvent::Worker(WorkerEvent::ScanProgress {
        job_id,
        phase: ScanPhase::Global,
        message: "Global scan finished: 1 target(s)".to_string(),
        plan: Some(plan_with_targets(vec![
            global_target.clone(),
            project_target.clone(),
        ])),
    }));
    app.update(UiEvent::Worker(WorkerEvent::ScanProgress {
        job_id,
        phase: ScanPhase::Global,
        message: "Global scan finished: 1 target(s)".to_string(),
        plan: Some(plan_with_targets(vec![
            global_target.clone(),
            project_target.clone(),
        ])),
    }));

    assert_eq!(
        app.targets
            .iter()
            .map(|target| &target.id)
            .collect::<Vec<_>>(),
        vec![&global_target.id, &project_target.id]
    );
    assert_eq!(app.targets.len(), 2);
    assert!(app.selected_ids.contains(&project_target.id));
    assert!(!app.selected_ids.contains(&global_target.id));
}

#[test]
fn stale_scan_updates_do_not_overwrite_newer_scan_results() {
    let mut app = App::new();
    let startup_effects = app.startup_effects();
    let [Effect::StartScan { job_id: first_job }] = startup_effects.as_slice() else {
        panic!("startup requests scan");
    };
    let first_job = *first_job;
    app.update(UiEvent::Worker(WorkerEvent::ScanStarted {
        job_id: first_job,
    }));

    let manual_effects = app.update(key(KeyCode::Char('s')));
    let [Effect::StartScan { job_id: second_job }] = manual_effects.as_slice() else {
        panic!("manual scan starts second job");
    };
    let second_job = *second_job;
    let representative = representative_plan();
    let second_target = representative.targets[0].clone();
    let stale_target = representative.targets[1].clone();

    app.update(UiEvent::Worker(WorkerEvent::ScanStarted {
        job_id: second_job,
    }));
    app.update(UiEvent::Worker(WorkerEvent::ScanProgress {
        job_id: second_job,
        phase: ScanPhase::Projects,
        message: "Project scan finished: 1 target(s)".to_string(),
        plan: Some(plan_with_targets(vec![second_target.clone()])),
    }));
    app.update(UiEvent::Worker(WorkerEvent::ScanFinished {
        job_id: first_job,
        plan: plan_with_targets(vec![stale_target]),
        health: ScanHealth::complete(),
    }));

    assert_eq!(app.targets.len(), 1);
    assert_eq!(app.targets[0].id, second_target.id);
    assert_eq!(app.jobs[0].status, JobStatus::Succeeded);
    assert_eq!(app.jobs[1].status, JobStatus::Running);
}

#[test]
fn cancellation_key_requests_active_job_cancellation() {
    let mut app = App::with_plan(representative_plan());
    let effects = app.update(key(KeyCode::Char('s')));
    let [Effect::StartScan { job_id }] = effects.as_slice() else {
        panic!("scan key starts scan");
    };
    let job_id = *job_id;

    let effects = app.update(key(KeyCode::Char('x')));

    assert!(matches!(
        effects.as_slice(),
        [Effect::CancelJob { job_id: requested }] if *requested == job_id
    ));
    assert_eq!(app.jobs[0].status, JobStatus::Cancelling);
    assert_eq!(
        app.jobs[0].progress,
        "Cancellation requested; current action cannot be interrupted."
    );

    app.update(UiEvent::Worker(WorkerEvent::JobCanceled { job_id }));
    assert_eq!(app.jobs[0].status, JobStatus::Canceled);
}

#[test]
fn scan_updates_invalidate_open_confirmation_and_block_enter() {
    let mut app = App::with_plan(representative_plan());
    app.selected_ids.clear();
    let selected_target = app.targets[0].clone();
    app.selected_ids.insert(selected_target.id.clone());
    app.update(key(KeyCode::Char('c')));

    let effects = app.startup_effects();
    let [Effect::StartScan { job_id }] = effects.as_slice() else {
        panic!("startup requests a scan");
    };
    let job_id = *job_id;
    let updated_plan = plan_with_targets(vec![app.targets[1].clone()]);

    app.update(UiEvent::Worker(WorkerEvent::ScanProgress {
        job_id,
        phase: ScanPhase::Projects,
        message: "Scan found an updated target".to_string(),
        plan: Some(updated_plan.clone()),
    }));
    app.update(UiEvent::Worker(WorkerEvent::ScanFinished {
        job_id,
        plan: updated_plan,
        health: ScanHealth::complete(),
    }));
    for ch in "confirm".chars() {
        app.update(key(KeyCode::Char(ch)));
    }

    let effects = app.update(key(KeyCode::Enter));

    assert!(effects.is_empty());
    let Overlay::Confirm(confirm) = &app.overlay else {
        panic!("invalidated confirmation remains visible");
    };
    assert!(confirm.invalidated_by_scan);
    assert_eq!(
        confirm.feedback.as_deref(),
        Some("Scan results changed. Close this dialog and confirm the updated selection.")
    );
    assert!(render_text(&app).contains("This confirmation is disabled."));
    assert!(app.logs.iter().any(|entry| {
        entry
            .message
            .contains("Confirmation rejected because scan results changed")
    }));
}

#[test]
fn accepted_confirmation_executes_its_frozen_manifest() {
    let mut app = App::with_plan(representative_plan());
    app.selected_ids.clear();
    let frozen_target = app.targets[0].clone();
    app.selected_ids.insert(frozen_target.id.clone());
    app.update(key(KeyCode::Char('c')));

    app.selected_ids.clear();
    app.selected_ids.insert(app.targets[1].id.clone());
    for ch in "confirm".chars() {
        app.update(key(KeyCode::Char(ch)));
    }

    let effects = app.update(key(KeyCode::Enter));

    let [Effect::StartClean { plan, selected, .. }] = effects.as_slice() else {
        panic!("confirmation emits one clean effect");
    };
    assert_eq!(plan.targets, vec![frozen_target.clone()]);
    assert_eq!(selected, &vec![frozen_target.id]);
}

#[test]
fn staged_scan_preserves_explicit_selection_and_defaults_new_targets() {
    let original_target = representative_plan().targets[0].clone();
    let mut new_target = representative_plan().targets[1].clone();
    new_target.selected_by_default = true;
    let mut app = App::with_plan(plan_with_targets(vec![original_target.clone()]));

    app.update(key(KeyCode::Char(' ')));
    assert!(!app.selected_ids.contains(&original_target.id));

    let effects = app.startup_effects();
    let [Effect::StartScan { job_id }] = effects.as_slice() else {
        panic!("startup requests a scan");
    };
    let job_id = *job_id;
    let updated_plan = plan_with_targets(vec![original_target.clone(), new_target.clone()]);

    app.update(UiEvent::Worker(WorkerEvent::ScanProgress {
        job_id,
        phase: ScanPhase::Projects,
        message: "Project scan updated".to_string(),
        plan: Some(updated_plan.clone()),
    }));
    app.update(UiEvent::Worker(WorkerEvent::ScanFinished {
        job_id,
        plan: updated_plan,
        health: ScanHealth::complete(),
    }));

    assert!(!app.selected_ids.contains(&original_target.id));
    assert!(app.selected_ids.contains(&new_target.id));

    let effects = app.startup_effects();
    let [Effect::StartScan { job_id }] = effects.as_slice() else {
        panic!("follow-up scan starts");
    };
    app.update(UiEvent::Worker(WorkerEvent::ScanFinished {
        job_id: *job_id,
        plan: plan_with_targets(vec![new_target]),
        health: ScanHealth::complete(),
    }));
    assert!(!app.selection_overrides.contains_key(&original_target.id));
}

#[test]
fn active_cleanup_rejects_new_scan_and_cleanup_requests() {
    let mut app = App::with_plan(representative_plan());
    let job_id = app.start_job(JobKind::Clean, "Clean fixture");

    assert!(app.update(key(KeyCode::Char('s'))).is_empty());
    assert!(app.update(key(KeyCode::Char('c'))).is_empty());

    assert_eq!(app.jobs.len(), 1);
    assert_eq!(app.jobs[0].id, job_id);
    assert!(matches!(app.overlay, Overlay::None));
    assert!(app.logs.iter().any(|entry| {
        entry
            .message
            .contains("scan requests are disabled until it finishes")
    }));
    assert!(
        app.logs
            .iter()
            .any(|entry| { entry.message.contains("a second cleanup cannot start") })
    );
}

#[test]
fn cancelling_job_ignores_late_progress_events() {
    let mut app = App::with_plan(representative_plan());
    let job_id = app.start_job(JobKind::Clean, "Clean fixture");
    let target_id = app.targets[0].id.clone();
    let plan = plan_with_targets(vec![app.targets[0].clone()]);
    app.cleanup_progress = Some(cleanup_progress_for_plan(job_id, &plan));

    app.update(key(KeyCode::Char('x')));
    let cancelling_progress = app.jobs[0].progress.clone();
    app.update(UiEvent::Worker(WorkerEvent::JobProgress {
        job_id,
        message: "Late worker progress".to_string(),
    }));
    app.update(UiEvent::Worker(WorkerEvent::CleanProgress {
        job_id,
        target_id,
        status: ExecutionTargetStatus::Succeeded,
        message: "Late cleanup progress".to_string(),
        detail: "completed".to_string(),
        completed: 1,
        total: 1,
    }));

    assert_eq!(app.jobs[0].status, JobStatus::Cancelling);
    assert_eq!(app.jobs[0].progress, cancelling_progress);
    assert_eq!(
        app.cleanup_progress
            .as_ref()
            .map(|progress| progress.completed),
        Some(0)
    );
}

#[test]
fn terminal_jobs_ignore_late_worker_events() {
    let mut succeeded = App::with_plan(representative_plan());
    let succeeded_job = succeeded.start_job(JobKind::Clean, "Clean fixture");
    succeeded.update(UiEvent::Worker(WorkerEvent::CleanFinished {
        job_id: succeeded_job,
        report: ExecutionReport {
            dry_run: false,
            selected: 1,
            attempted: 1,
            succeeded: 1,
            failed: 0,
            skipped: 0,
            failures: Vec::new(),
            audit_log: None,
        },
    }));
    assert_terminal_job_ignores_late_events(&mut succeeded, succeeded_job, JobStatus::Succeeded);

    let mut failed = App::with_plan(representative_plan());
    let failed_job = failed.start_job(JobKind::Clean, "Clean fixture");
    failed.update(UiEvent::Worker(WorkerEvent::JobFailed {
        job_id: failed_job,
        message: "Cleanup failed".to_string(),
    }));
    assert_terminal_job_ignores_late_events(&mut failed, failed_job, JobStatus::Failed);

    let mut canceled = App::with_plan(representative_plan());
    let canceled_job = canceled.start_job(JobKind::Clean, "Clean fixture");
    canceled.update(key(KeyCode::Char('x')));
    canceled.update(UiEvent::Worker(WorkerEvent::JobCanceled {
        job_id: canceled_job,
    }));
    assert_terminal_job_ignores_late_events(&mut canceled, canceled_job, JobStatus::Canceled);
}

#[test]
fn byte_summaries_count_duplicate_paths_once() {
    let cache_path = std::env::temp_dir().join("devsweep-npm-cache");
    let plan = CleanupPlan {
        version: CLEANUP_PLAN_VERSION,
        targets: vec![
            target(
                "npm.cache.verify",
                Scope::Global,
                Ecosystem::Node,
                TargetKind::PackageCache,
                Some(cache_path.clone()),
                2048,
                RiskLevel::Low,
                true,
                false,
                CleanAction::Command {
                    program: "npm".to_string(),
                    args: vec!["cache".to_string(), "verify".to_string()],
                    cwd: None,
                    irreversible: true,
                },
            ),
            target(
                "npm.cache.clean",
                Scope::Global,
                Ecosystem::Node,
                TargetKind::PackageCache,
                Some(cache_path),
                2048,
                RiskLevel::Medium,
                true,
                false,
                CleanAction::Command {
                    program: "npm".to_string(),
                    args: vec![
                        "cache".to_string(),
                        "clean".to_string(),
                        "--force".to_string(),
                    ],
                    cwd: None,
                    irreversible: true,
                },
            ),
        ],
    };
    let app = App::with_plan(plan);

    assert_eq!(app.scope_bytes(ScopeKind::Global), 2048);
    assert_eq!(app.selected_bytes(), 2048);
    let mut app = app;
    app.selected_ids.remove(&app.targets[0].id);
    assert_eq!(
        app.confirm_state()
            .expect("selected cleanup target validates")
            .estimated_bytes,
        2048
    );
}

#[test]
fn quit_during_active_job_opens_confirm_instead_of_detaching() {
    let mut app = App::with_plan(representative_plan());
    let _job = app.start_job(JobKind::Clean, "Clean fixture");
    let effects = app.update(key(KeyCode::Char('q')));
    assert!(effects.is_empty());
    assert!(!app.should_quit);
    assert!(matches!(app.overlay, Overlay::QuitConfirm));

    // Wait path keeps the app running.
    app.update(key(KeyCode::Char('w')));
    assert!(matches!(app.overlay, Overlay::None));
    assert!(!app.should_quit);

    // Cancel-and-wait requests CancelJob and defers quit until terminal.
    app.overlay = Overlay::QuitConfirm;
    let effects = app.update(key(KeyCode::Char('c')));
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::CancelJob { .. }))
    );
    assert!(app.quit_after_jobs);
    assert!(!app.should_quit);
    assert_eq!(app.jobs[0].status, JobStatus::Cancelling);

    app.update(UiEvent::Worker(WorkerEvent::JobCanceled {
        job_id: app.jobs[0].id,
    }));
    assert!(app.should_quit);
}

#[test]
fn successful_cleanup_tombstones_target_until_rescan() {
    let mut app = App::with_plan(representative_plan());
    let target_id = app.targets[0].id.clone();
    app.selected_ids.insert(target_id.clone());
    let job_id = app.start_job(JobKind::Clean, "Clean fixture");

    app.update(UiEvent::Worker(WorkerEvent::CleanProgress {
        job_id,
        target_id: target_id.clone(),
        status: ExecutionTargetStatus::Succeeded,
        message: "cleaned".to_string(),
        detail: "ok".to_string(),
        completed: 1,
        total: 1,
    }));

    assert!(app.is_cleaned(&target_id));
    assert!(!app.selected_ids.contains(&target_id));
    assert!(
        !app.selected_targets()
            .iter()
            .any(|target| target.id == target_id)
    );

    app.update(key(KeyCode::Char(' ')));
    assert!(
        app.logs.iter().any(|entry| {
            entry
                .message
                .contains("Cleaned targets stay disabled until the next rescan")
        }) || !app.selected_ids.contains(&target_id)
    );

    let effects = app.startup_effects();
    let [Effect::StartScan { job_id }] = effects.as_slice() else {
        panic!("rescan starts");
    };
    app.update(UiEvent::Worker(WorkerEvent::ScanFinished {
        job_id: *job_id,
        plan: representative_plan(),
        health: ScanHealth::complete(),
    }));
    assert!(!app.is_cleaned(&target_id));
}

#[test]
fn viewport_keeps_selection_visible_for_long_lists() {
    let mut targets = Vec::new();
    for index in 0..30 {
        let mut target = representative_plan().targets[0].clone();
        target.id = TargetId::new(format!("item-{index}"));
        targets.push(target);
    }
    let mut app = App::with_plan(CleanupPlan {
        version: CLEANUP_PLAN_VERSION,
        targets,
    });
    app.selected_index = 0;
    app.list_scroll = 0;
    for _ in 0..20 {
        app.move_selection(1);
    }
    app.ensure_selection_visible(5);
    assert!(app.selected_index >= app.list_scroll);
    assert!(app.selected_index < app.list_scroll + 5);
    app.page_selection(1);
    app.ensure_selection_visible(5);
    assert!(app.selected_index < app.targets.len());
}

fn assert_terminal_job_ignores_late_events(
    app: &mut App,
    job_id: JobId,
    expected_status: JobStatus,
) {
    let target_id = app.targets[0].id.clone();
    let targets = app.targets.clone();
    let progress = app.jobs[0].progress.clone();
    let cleanup_progress = app.cleanup_progress.clone();

    app.update(UiEvent::Worker(WorkerEvent::JobProgress {
        job_id,
        message: "Late worker progress".to_string(),
    }));
    app.update(UiEvent::Worker(WorkerEvent::CleanProgress {
        job_id,
        target_id,
        status: ExecutionTargetStatus::Succeeded,
        message: "Late cleanup progress".to_string(),
        detail: "completed".to_string(),
        completed: 1,
        total: 1,
    }));
    app.update(UiEvent::Worker(WorkerEvent::CleanFinished {
        job_id,
        report: ExecutionReport {
            dry_run: false,
            selected: 1,
            attempted: 1,
            succeeded: 1,
            failed: 0,
            skipped: 0,
            failures: Vec::new(),
            audit_log: None,
        },
    }));
    app.update(UiEvent::Worker(WorkerEvent::JobCanceled { job_id }));

    assert_eq!(app.jobs[0].status, expected_status);
    assert_eq!(app.jobs[0].progress, progress);
    assert_eq!(app.cleanup_progress, cleanup_progress);
    assert_eq!(app.targets, targets);
}
