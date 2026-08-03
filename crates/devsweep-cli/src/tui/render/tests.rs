use std::path::{Path, PathBuf};

use crossterm::event::KeyCode;
use ratatui::{Terminal, backend::TestBackend};

use super::{overlays::dry_run_lines, targets::target_details_lines, *};
use crate::execution::{ExecutionReport, ExecutionTargetStatus};
use crate::inventory::{
    CapacityObservation, INVENTORY_REPORT_VERSION, InventoryClassification, InventoryReport,
    OrphanPnpmStoreFinding, PnpmProjectReference,
};
use crate::model::{
    CLEANUP_PLAN_VERSION, CleanAction, CleanupPlan, Ecosystem, Evidence, RiskLevel,
    ScanCompleteness, ScanDiagnostic, ScanDiagnosticOutcome, ScanDiagnosticStage, ScanHealth,
    ScanProcessOutput, ScanProcessProbe, ScanProcessStatus, ScanTotals, Scope, TargetKind,
};
use crate::tui::app::{
    ActiveTab, App, CleanupItemStatus, CleanupProgress, CleanupProgressItem, JobKind, Overlay,
    UiEvent, WorkerEvent, cleanup_progress_for_plan,
};
use crate::tui::display::{
    command_preview, compact_text, display_path, sanitize_display_text, scope_label,
};
use crate::tui::test_support::{
    key, render_text, render_text_with_size, representative_plan, target,
};

#[test]
fn representative_state_renders_with_targets_details_and_jobs() {
    let mut app = App::with_plan(representative_plan());
    let job_id = app.start_job(JobKind::Scan, "Scan fixture");
    app.update(UiEvent::Worker(WorkerEvent::JobProgress {
        job_id,
        message: "Scanning fixture".to_string(),
    }));
    app.active_tab = ActiveTab::JobsLogs;

    let backend = TestBackend::new(120, 32);
    let mut terminal = Terminal::new(backend).expect("test terminal");

    terminal
        .draw(|frame| render_app(frame, &app))
        .expect("representative app renders");
}

#[test]
fn representative_state_renders_at_full_supported_size() {
    let app = App::with_plan(representative_plan());
    let rendered = render_text_with_size(&app, 100, 28);

    assert!(rendered.contains("devsweep"));
    assert!(rendered.contains("Summary"));
    assert!(rendered.contains("Categories"));
    assert!(rendered.contains("Targets"));
    assert!(rendered.contains("Details"));
    assert!(rendered.contains("Global"));
    assert!(rendered.contains("Sel Risk Size Target"));
    assert!(rendered.contains("> [x] Low"));
    assert!(rendered.contains("1.0 KiB"));
}

#[test]
fn representative_state_renders_at_degraded_supported_size() {
    let app = App::with_plan(representative_plan());
    let rendered = render_text_with_size(&app, 80, 24);

    assert!(rendered.contains("Summary"));
    assert!(rendered.contains("Targets"));
    assert!(rendered.contains("Details"));
    assert!(!rendered.contains("Categories"));
}

#[test]
fn pycache_group_renders_collapsed_and_expanded_target_paths() {
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
    let mut app = App::with_plan(CleanupPlan {
        version: CLEANUP_PLAN_VERSION,
        targets,
    });

    let collapsed = render_text(&app);
    assert!(collapsed.contains("3 __pycache__ entries"));
    assert!(!collapsed.contains("package_a/__pycache__"));

    app.update(key(KeyCode::Char('g')));
    let expanded = render_text(&app);
    assert!(expanded.contains("package_a/__pycache__"));
    assert!(expanded.contains("package_b/__pycache__"));
    assert!(expanded.contains("package_c/__pycache__"));
}

#[test]
fn partial_scan_health_renders_truthful_totals_and_diagnostics() {
    let mut app = App::with_plan(representative_plan());
    let mut plan = representative_plan();
    plan.targets[1].size_complete = false;
    plan.targets[2].size_complete = false;
    let mut health = ScanHealth::new(
        ScanCompleteness::Partial,
        vec![ScanDiagnostic {
            stage: ScanDiagnosticStage::CargoMetadata,
            path: PathBuf::from("C:/workspace/project/Cargo.toml"),
            outcome: ScanDiagnosticOutcome::OutputTruncated,
            detail: "captured cargo metadata output was truncated".to_string(),
            process: Some(ScanProcessProbe {
                status: ScanProcessStatus::InvalidOutput,
                stdout: ScanProcessOutput {
                    truncated: true,
                    retained_bytes: 1024,
                    total_bytes: 4096,
                },
                stderr: ScanProcessOutput {
                    truncated: false,
                    retained_bytes: 0,
                    total_bytes: 0,
                },
            }),
        }],
    );
    health.totals = ScanTotals::from_cleanup_plan(&plan);
    let job_id = app.start_job(JobKind::Scan, "Scan fixture");
    app.update(UiEvent::Worker(WorkerEvent::ScanFinished {
        job_id,
        plan,
        health,
    }));
    app.active_tab = ActiveTab::JobsLogs;

    let rendered = render_text_with_size(&app, 120, 50);

    assert!(rendered.contains("Scan partial"));
    assert!(rendered.contains("Verified 1.0 KiB"));
    assert!(rendered.contains("Partial lower bound >= 2.0 KiB"));
    assert!(rendered.contains("Unknown 1"));
    assert!(rendered.contains("Scan diagnostics"));
    assert!(rendered.contains("cargo metadata output truncated"));
    assert!(rendered.contains("path C:/workspace/project/Cargo.toml"));
    assert!(rendered.contains("status InvalidOutput"));
    assert!(rendered.contains("stdout 1024/4096 truncated"));
    assert!(rendered.contains("stderr 0/0"));
    assert!(rendered.contains("detail captured cargo metadata output was truncated"));
}

#[test]
fn inventory_tab_renders_read_only_observations_health_and_inspection() {
    let mut app = App::with_plan(representative_plan());
    let mut health = ScanHealth::new(
        ScanCompleteness::Partial,
        vec![ScanDiagnostic {
            stage: ScanDiagnosticStage::Sizing,
            path: PathBuf::from("C:/inventory-root/archive"),
            outcome: ScanDiagnosticOutcome::Skipped,
            detail: "size entry budget exhausted".to_string(),
            process: None,
        }],
    );
    health.totals = ScanTotals {
        verified_bytes: 4096,
        partial_lower_bound_bytes: 2048,
        unknown_target_count: 1,
    };
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
                estimated_bytes: 2048,
                size_complete: false,
                warnings: vec![crate::model::SizingWarning {
                    kind: crate::model::SizingWarningKind::EntryBudgetExhausted,
                    detail: "size entry budget exhausted".to_string(),
                }],
            },
        ],
        health,
        orphan_pnpm_store: Some(OrphanPnpmStoreFinding {
            classification: InventoryClassification::InspectOnly,
            candidate_path: PathBuf::from("C:/inventory-root/.pnpm-store"),
            configured_store: PathBuf::from("C:/Users/test/AppData/Local/pnpm/store"),
            project_references: vec![PnpmProjectReference {
                path: PathBuf::from("C:/inventory-root/app/.npmrc"),
                references_candidate: false,
                references_configured_store: true,
            }],
        }),
    };
    let job_id = app.start_job(JobKind::Inventory, "Inventory fixture");
    app.update(UiEvent::Worker(WorkerEvent::InventoryFinished {
        job_id,
        report: Box::new(report),
    }));
    app.active_tab = ActiveTab::Inventory;

    let rendered = render_text_with_size(&app, 120, 50);

    assert!(rendered.contains("Capacity observations"));
    assert!(rendered.contains("inventory only"));
    assert!(rendered.contains("C:/inventory-root/archive"));
    assert!(rendered.contains(">= 2.0 KiB"));
    assert!(rendered.contains("Health partial"));
    assert!(rendered.contains("Verified 4.0 KiB"));
    assert!(rendered.contains("Inspect-only pnpm store"));
    assert!(rendered.contains("Configured"), "{rendered}");
    assert!(rendered.contains("pnpm/store"), "{rendered}");
    assert!(rendered.contains("Reference"), "{rendered}");
    assert!(rendered.contains(".npmrc"), "{rendered}");
    assert!(rendered.contains("Diagnostics"));
    assert!(rendered.contains("size entry budget exhausted"));
    assert!(!rendered.contains("node.next_cache:C:"));
}

#[test]
fn rules_tab_lists_catalogue_entries() {
    let mut app = App::with_plan(representative_plan());
    app.active_tab = ActiveTab::Rules;

    let rendered = render_text_with_size(&app, 120, 50);

    assert!(rendered.contains("Project rules"), "project section header");
    assert!(rendered.contains("rust.target"), "project rule listed");
    assert!(
        rendered.contains("gradle.caches"),
        "new global cache listed"
    );
}

#[test]
fn target_details_show_freshness_guard_evidence() {
    let mut target = representative_plan().targets[0].clone();
    target.evidence.push(Evidence::RuleMatched {
        rule_id: "ranking.freshness_guard.7d".to_string(),
    });

    let lines = target_details_lines(&target);
    let rendered = lines
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("rule ranking.freshness_guard.7d"));
}

#[test]
fn target_details_show_typed_sizing_warnings() {
    let mut target = representative_plan().targets[0].clone();
    target.sizing_warnings.push(crate::model::SizingWarning {
        kind: crate::model::SizingWarningKind::EntryBudgetExhausted,
        detail: "review rescan reached the entry budget".to_string(),
    });

    let rendered = target_details_lines(&target)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("Sizing warnings"));
    assert!(rendered.contains("entry budget exhausted"));
    assert!(rendered.contains("review rescan reached the entry budget"));
}

#[test]
fn smoke_renders_dashboard_details_confirm_and_jobs_logs_states() {
    let mut app = App::with_plan(representative_plan());

    let dashboard = render_text(&app);
    assert!(dashboard.contains("Dashboard"));
    assert!(dashboard.contains("Selected"));

    app.overlay = Overlay::Details;
    let details = render_text(&app);
    assert!(details.contains("Target details"));
    assert!(details.contains("Evidence"));

    app.overlay = Overlay::None;
    app.selected_ids.clear();
    app.selected_ids.insert(app.targets[1].id.clone());
    app.update(key(KeyCode::Char('c')));
    let confirm = render_text(&app);
    assert!(confirm.contains("Confirm cleanup"));
    assert!(confirm.contains("Irreversible command-backed cleanup"));
    assert!(confirm.contains("Cleanup commands"));
    assert!(confirm.contains("argv: npm cache clean --force"));
    assert!(confirm.contains("Plan digest:"));
    assert!(confirm.contains("Required: confirm"));
    assert!(confirm.contains("Enter runs after confirm matches"));

    app.overlay = Overlay::None;
    app.active_tab = ActiveTab::JobsLogs;
    let job_id = app.start_job(JobKind::Scan, "Scan fixture");
    app.update(UiEvent::Worker(WorkerEvent::JobProgress {
        job_id,
        message: "Scanning fixture".to_string(),
    }));
    let jobs_logs = render_text(&app);
    assert!(jobs_logs.contains("Jobs"));
    assert!(jobs_logs.contains("Logs"));
    assert!(jobs_logs.contains("Scanning fixture"));
}

#[test]
fn display_hygiene_quotes_argv_and_strips_controls() {
    assert_eq!(
        command_preview(
            "npm",
            &["cache".into(), "clean".into(), "--force".into()],
            &None
        ),
        "argv: npm cache clean --force"
    );
    assert!(
        command_preview("tool", &["path with space".into()], &None).contains("\"path with space\"")
    );
    assert!(!sanitize_display_text("a\u{1b}[31mb\u{07}c").contains('\u{1b}'));
    assert!(!sanitize_display_text("a\u{1b}[31mb\u{07}c").contains('\u{07}'));
    let cjk = "中文路径需要按显示宽度截断并且不能越界溢出";
    let truncated = compact_text(cjk, 12);
    assert!(unicode_width::UnicodeWidthStr::width(truncated.as_str()) <= 12);
}

#[test]
fn cleaned_target_renders_as_tombstone() {
    let mut app = App::with_plan(representative_plan());
    let target_id = app.targets[0].id.clone();
    app.cleaned_ids.insert(target_id);
    let text = render_text(&app);
    assert!(text.contains("cleaned; rescan to refresh") || text.contains("[-]"));
}

#[test]
fn invalidated_confirmation_renders_reconfirmation_state() {
    let mut app = App::with_plan(representative_plan());
    app.update(key(KeyCode::Char('c')));
    let effects = app.startup_effects();
    let [crate::tui::app::Effect::StartScan { job_id }] = effects.as_slice() else {
        panic!("startup requests a scan");
    };

    app.update(UiEvent::Worker(WorkerEvent::ScanProgress {
        job_id: *job_id,
        phase: crate::scan::ScanPhase::Projects,
        message: "Scan state changed".to_string(),
        plan: None,
    }));

    let rendered = render_text(&app);
    assert!(rendered.contains("This confirmation is disabled."));
    assert!(rendered.contains("confirm the updated selection again"));
    assert!(rendered.contains("[Enter] Blocked"));
}

#[test]
fn confirmation_and_dry_run_show_selected_targets_across_scopes() {
    let mut app = App::with_plan(representative_plan());
    app.selected_ids.clear();
    app.selected_ids.insert(app.targets[0].id.clone());
    app.selected_ids.insert(app.targets[1].id.clone());
    let project_scope = scope_label(&app.targets[0].scope);
    let project_path = app.targets[0]
        .path
        .as_ref()
        .expect("project target path")
        .display()
        .to_string();
    let global_path = app.targets[1]
        .path
        .as_ref()
        .expect("global target path")
        .display()
        .to_string();

    let dry_run = dry_run_lines(&app)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(dry_run.contains(&project_scope));
    assert!(dry_run.contains("Global"));
    assert!(dry_run.contains(&project_path));
    assert!(dry_run.contains(&global_path));

    app.update(key(KeyCode::Char('c')));
    let Overlay::Confirm(confirm) = &app.overlay else {
        panic!("mixed selection opens confirm");
    };
    assert_eq!(confirm.selected_targets.len(), 2);
    assert!(
        confirm
            .selected_targets
            .iter()
            .any(|target| target.contains(&project_scope))
    );
    assert!(
        confirm
            .selected_targets
            .iter()
            .any(|target| target.contains("Global"))
    );
    let rendered = render_text(&app);
    assert!(rendered.contains("Selected targets:"));
    assert!(rendered.contains("Project"));
    assert!(rendered.contains("Global"));
}

#[test]
fn accepted_irreversible_confirmation_renders_initial_progress() {
    let mut app = App::with_plan(representative_plan());
    app.selected_ids.clear();
    app.selected_ids.insert(app.targets[1].id.clone());

    app.update(key(KeyCode::Char('c')));
    let phrase = match &app.overlay {
        Overlay::Confirm(confirm) => confirm.required_phrase.clone(),
        _ => panic!("command selection opens confirm"),
    };
    for ch in phrase.chars() {
        app.update(key(KeyCode::Char(ch)));
    }
    let effects = app.update(key(KeyCode::Enter));

    assert!(matches!(
        effects.as_slice(),
        [crate::tui::app::Effect::StartClean { .. }]
    ));
    let progress = app
        .cleanup_progress
        .as_ref()
        .expect("accepted confirmation shows progress immediately");
    assert_eq!(progress.completed, 0);
    assert_eq!(progress.total, 1);
    assert!(!progress.finished);
    let rendered = render_text(&app);
    assert!(rendered.contains("Cleanup progress"));
    assert!(rendered.contains("0 / 1 Executing selected cleanup plan"));
    assert!(rendered.contains("PENDING"));
}

#[test]
fn cleanup_progress_updates_job_and_renders_modal() {
    let mut app = App::with_plan(representative_plan());
    let job_id = app.start_job(JobKind::Clean, "Clean fixture");
    let target_id = app.targets[1].id.clone();
    app.cleanup_progress = Some(CleanupProgress {
        job_id,
        completed: 0,
        total: 3,
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
        completed: 1,
        total: 3,
    }));

    assert_eq!(app.jobs[0].progress, "1 / 3 npm cache clean");
    let rendered = render_text(&app);
    assert!(rendered.contains("Cleanup progress"));
    assert!(rendered.contains("1 / 3 npm cache clean"));
    assert!(rendered.contains("[##########----------------------]"));
    assert!(rendered.contains("OK"));

    app.active_tab = ActiveTab::JobsLogs;
    app.cleanup_progress = None;
    let jobs_logs = render_text(&app);
    assert!(jobs_logs.contains("1 / 3 npm cache clean"));
    assert!(jobs_logs.contains("INFO Clean"));
}

#[test]
fn cleanup_progress_renders_mixed_target_results() {
    let mut app = App::with_plan(representative_plan());
    let job_id = app.start_job(JobKind::Clean, "Clean fixture");
    let plan = CleanupPlan {
        version: crate::model::CLEANUP_PLAN_VERSION,
        targets: app.targets.clone(),
    };
    app.cleanup_progress = Some(cleanup_progress_for_plan(job_id, &plan));
    let success_id = app.targets[0].id.clone();
    let failed_id = app.targets[1].id.clone();
    let skipped_id = app.targets[2].id.clone();

    app.update(UiEvent::Worker(WorkerEvent::CleanProgress {
        job_id,
        target_id: success_id,
        status: ExecutionTargetStatus::Succeeded,
        message: "node cache: completed".to_string(),
        detail: "completed".to_string(),
        completed: 1,
        total: 3,
    }));
    app.update(UiEvent::Worker(WorkerEvent::CleanProgress {
        job_id,
        target_id: failed_id.clone(),
        status: ExecutionTargetStatus::Failed,
        message: "npm cache clean: failed".to_string(),
        detail: "Access denied".to_string(),
        completed: 2,
        total: 3,
    }));
    app.update(UiEvent::Worker(WorkerEvent::CleanProgress {
        job_id,
        target_id: skipped_id,
        status: ExecutionTargetStatus::Skipped,
        message: "cargo home: skipped".to_string(),
        detail: "inspect-only target has no executable cleanup action".to_string(),
        completed: 3,
        total: 3,
    }));
    app.update(UiEvent::Worker(WorkerEvent::CleanFinished {
        job_id,
        report: ExecutionReport {
            dry_run: false,
            selected: 3,
            attempted: 2,
            succeeded: 1,
            failed: 1,
            skipped: 1,
            failures: vec![crate::execution::ActionFailure {
                target_id: failed_id,
                message: "Access denied: file is locked".to_string(),
            }],
            outcomes: Vec::new(),
            notes: Vec::new(),
            estimated_recoverable: Default::default(),
            confirmation_digest: crate::execution::ConfirmationDigest::new("test-digest"),
            audit_log: Some(PathBuf::from("audit.jsonl")),
        },
    }));

    let rendered = render_text(&app);
    assert!(rendered.contains("OK"));
    assert!(rendered.contains("FAILED"));
    assert!(rendered.contains("SKIPPED"));
    assert!(rendered.contains("Access denied"));

    app.active_tab = ActiveTab::JobsLogs;
    app.cleanup_progress = None;
    let jobs_logs = render_text(&app);
    assert!(jobs_logs.contains("ERR"));
    assert!(jobs_logs.contains("Audit"));
    assert!(jobs_logs.contains("audit.jsonl"));
}

#[test]
fn footer_renders_contextual_key_actions() {
    let mut app = App::with_plan(representative_plan());

    let normal = render_text(&app);
    assert!(normal.contains("NORMAL"));
    assert!(normal.contains("[s] Scan"));
    assert!(normal.contains("[c] Clean"));

    app.update(key(KeyCode::Char('c')));
    let confirm = render_text(&app);
    assert!(confirm.contains("CONFIRM"));
    assert!(confirm.contains("[Enter] Run"));
    assert!(confirm.contains("[Esc] Cancel"));
    assert!(!confirm.contains("[s] Scan"));
    assert!(!confirm.contains("[/] Filter"));

    app.overlay = Overlay::None;
    app.cleanup_progress = Some(CleanupProgress {
        job_id: 1,
        completed: 1,
        total: 1,
        summary: "finished: 1 succeeded, 0 failed, 0 skipped".to_string(),
        finished: true,
        items: Vec::new(),
    });
    let done = render_text(&app);
    assert!(done.contains("DONE"));
    assert!(done.contains("[Enter] Close"));
    assert!(done.contains("[Esc] Close"));
}

#[test]
fn narrow_footer_keeps_primary_actions_visible() {
    let app = App::with_plan(representative_plan());
    let rendered = render_text_with_size(&app, 80, 24);

    assert!(rendered.contains("NORMAL"));
    assert!(rendered.contains("[s] Scan"));
    assert!(rendered.contains("[c] Clean"));
    assert!(rendered.contains("[q] Quit"));
}

#[test]
fn render_path_does_not_mutate_app_state() {
    let mut app = App::with_plan(representative_plan());
    app.overlay = Overlay::DryRun;
    let before = app.clone();
    let backend = TestBackend::new(100, 28);
    let mut terminal = Terminal::new(backend).expect("test terminal");

    terminal
        .draw(|frame| render_app(frame, &app))
        .expect("dry-run preview renders");

    assert_eq!(app, before);
}

#[test]
fn display_path_removes_windows_verbatim_prefixes() {
    assert_eq!(
        display_path(Path::new("\\\\?\\D:\\code\\devsweep\\target")),
        "D:\\code\\devsweep\\target"
    );
    assert_eq!(
        display_path(Path::new("\\\\?\\UNC\\server\\share\\cache")),
        "\\\\server\\share\\cache"
    );
    assert_eq!(
        display_path(Path::new("D:\\code\\devsweep\\target")),
        "D:\\code\\devsweep\\target"
    );
}

#[test]
fn rendered_paths_hide_windows_verbatim_prefixes() {
    let target_path = PathBuf::from("\\\\?\\D:\\code\\devsweep\\target");
    let marker_path = PathBuf::from("\\\\?\\D:\\code\\devsweep\\Cargo.toml");
    let plan = CleanupPlan {
        version: CLEANUP_PLAN_VERSION,
        targets: vec![target(
            "rust.target",
            Scope::Project {
                root: PathBuf::from("\\\\?\\D:\\code\\devsweep"),
            },
            Ecosystem::Rust,
            TargetKind::BuildArtifacts,
            Some(target_path.clone()),
            1024,
            RiskLevel::Low,
            true,
            false,
            CleanAction::Command {
                program: "cargo".to_string(),
                args: vec![
                    "clean".to_string(),
                    "--manifest-path".to_string(),
                    marker_path.display().to_string(),
                ],
                cwd: Some(PathBuf::from("\\\\?\\D:\\code\\devsweep")),
                irreversible: true,
            },
        )],
    };
    let mut app = App::with_plan(plan);
    app.overlay = Overlay::Details;

    let rendered = render_text(&app);

    assert!(!rendered.contains("\\\\?\\"));
    assert!(rendered.contains("D:\\code\\devsweep\\target"));
    assert!(rendered.contains("cwd: D:\\code\\devsweep"));
}
