use super::*;
use crate::{
    i18n::Locale,
    tui::{app::App, shell::ShellComposition, test_support::render_text_with_size},
};

fn entries() -> Vec<MaintenanceCatalogueEntryV1> {
    catalogue_entries().to_vec()
}

fn digest() -> String {
    format!("sha256:{}", "a".repeat(64))
}

fn preview_for(
    id: &str,
    action_class: MaintenanceActionClass,
) -> (MaintenancePlanV1, MaintenancePreviewV1) {
    (
        MaintenancePlanV1 {
            version: 1,
            catalogue_version: 1,
            operation_id: id.to_string(),
        },
        MaintenancePreviewV1 {
            version: 1,
            catalogue_version: 1,
            operation_id: id.to_string(),
            action_class,
            digest: digest(),
        },
    )
}

fn report(
    id: &str,
    action_class: MaintenanceActionClass,
    outcome: MaintenanceExecutionOutcome,
) -> MaintenanceExecutionReportV1 {
    MaintenanceExecutionReportV1 {
        version: 1,
        catalogue_version: 1,
        outcomes: vec![MaintenanceActionOutcomeV1 {
            operation_id: "optimize-op-fixture".to_string(),
            catalogue_id: id.to_string(),
            action_class,
            outcome,
            error_code: None,
        }],
    }
}

fn loaded() -> OptimizeModeState {
    let mut state = OptimizeModeState::default();
    state.reduce(OptimizeAction::ListStarted(1));
    state.reduce(OptimizeAction::ListFinished {
        job_id: 1,
        entries: entries(),
    });
    state
}

#[test]
fn catalogue_rows_are_the_closed_eight_and_guidance_cannot_preview() {
    let mut state = loaded();
    assert_eq!(state.visible_entries().len(), 8);
    assert_eq!(state.visible_entries()[0].id, "dns.flush");
    state.cursor = 4;
    state.reduce(OptimizeAction::SelectFocused);
    assert_eq!(
        state.selected_id.as_deref(),
        Some("guidance.drive_optimize")
    );
    assert_eq!(state.phase, OptimizePhase::Selected);
    state.reduce(OptimizeAction::PreviewStarted(2));
    assert_eq!(state.phase, OptimizePhase::Selected);
    assert!(state.operation_id.is_none());
}

#[test]
fn stale_preview_and_stale_run_are_rejected() {
    let mut state = loaded();
    state.reduce(OptimizeAction::SelectFocused);
    state.reduce(OptimizeAction::PreviewStarted(2));
    let (plan, preview) = preview_for("dns.flush", MaintenanceActionClass::Execute);
    state.reduce(OptimizeAction::PreviewFinished {
        job_id: 99,
        plan: plan.clone(),
        preview: preview.clone(),
    });
    assert_eq!(state.phase, OptimizePhase::Previewing);
    state.reduce(OptimizeAction::PreviewFinished {
        job_id: 2,
        plan,
        preview,
    });
    assert_eq!(state.phase, OptimizePhase::PreviewReady);
    state.reduce(OptimizeAction::OpenConfirmation);
    state.reduce(OptimizeAction::RunStarted(3));
    state.reduce(OptimizeAction::RunFinished {
        job_id: 3,
        report: report(
            "settings.search",
            MaintenanceActionClass::SettingsHandoff,
            MaintenanceExecutionOutcome::Launched,
        ),
    });
    assert_eq!(state.phase, OptimizePhase::Running);
    assert!(state.report.is_none());
}

#[test]
fn confirmation_is_required_and_settings_launch_is_not_completion() {
    let mut state = loaded();
    state.cursor = 2;
    state.reduce(OptimizeAction::SelectFocused);
    assert_eq!(state.selected_id.as_deref(), Some("settings.search"));
    state.reduce(OptimizeAction::PreviewStarted(2));
    let (plan, preview) = preview_for("settings.search", MaintenanceActionClass::SettingsHandoff);
    state.reduce(OptimizeAction::PreviewFinished {
        job_id: 2,
        plan,
        preview,
    });
    state.reduce(OptimizeAction::RunStarted(3));
    assert_eq!(state.phase, OptimizePhase::PreviewReady);
    state.reduce(OptimizeAction::OpenConfirmation);
    state.reduce(OptimizeAction::RunStarted(3));
    assert_eq!(state.phase, OptimizePhase::Launching);
    state.reduce(OptimizeAction::RunFinished {
        job_id: 3,
        report: report(
            "settings.search",
            MaintenanceActionClass::SettingsHandoff,
            MaintenanceExecutionOutcome::Launched,
        ),
    });
    assert_eq!(state.phase, OptimizePhase::Terminal);
    assert_eq!(
        state.report.as_ref().unwrap().outcomes[0].outcome,
        MaintenanceExecutionOutcome::Launched
    );
}

#[test]
fn only_dns_enters_running_and_unknown_after_dispatch_is_preserved() {
    let mut state = loaded();
    state.reduce(OptimizeAction::SelectFocused);
    state.reduce(OptimizeAction::PreviewStarted(2));
    let (plan, preview) = preview_for("dns.flush", MaintenanceActionClass::Execute);
    state.reduce(OptimizeAction::PreviewFinished {
        job_id: 2,
        plan,
        preview,
    });
    state.reduce(OptimizeAction::OpenConfirmation);
    state.reduce(OptimizeAction::RunStarted(3));
    assert_eq!(state.phase, OptimizePhase::Running);
    state.reduce(OptimizeAction::RunFinished {
        job_id: 3,
        report: report(
            "dns.flush",
            MaintenanceActionClass::Execute,
            MaintenanceExecutionOutcome::UnknownAfterDispatch,
        ),
    });
    assert_eq!(state.phase, OptimizePhase::Unknown);
}

#[test]
fn cancel_and_failed_events_reject_mismatched_ids() {
    let mut state = loaded();
    state.reduce(OptimizeAction::SelectFocused);
    state.reduce(OptimizeAction::PreviewStarted(2));
    state.reduce(OptimizeAction::Canceled(9));
    assert_eq!(state.phase, OptimizePhase::Previewing);
    state.reduce(OptimizeAction::Failed {
        job_id: 9,
        message: "stale".to_string(),
    });
    assert_eq!(state.phase, OptimizePhase::Previewing);
    state.reduce(OptimizeAction::Canceled(2));
    assert_eq!(state.phase, OptimizePhase::Selected);
}

#[test]
fn bilingual_test_backend_keeps_badges_and_never_claims_settings_completion() {
    for (locale, badge, launched, guidance) in [
        (
            Locale::En,
            "Runs here",
            "Settings launched. This is not a completed optimization.",
            "No run action",
        ),
        (
            Locale::ZhCn,
            "在此运行",
            "设置已启动。这不是已完成的优化。",
            "无运行操作",
        ),
    ] {
        let mut shell = ShellComposition::for_locale(locale).expect("shell");
        assert!(shell.activate(crate::tui::shell::ModeId::Optimize));
        let mut app = App::with_shell(shell);
        app.optimize.reduce(OptimizeAction::ListStarted(1));
        app.optimize.reduce(OptimizeAction::ListFinished {
            job_id: 1,
            entries: entries(),
        });
        app.optimize.cursor = 2;
        app.optimize.reduce(OptimizeAction::SelectFocused);
        let (plan, preview) =
            preview_for("settings.search", MaintenanceActionClass::SettingsHandoff);
        app.optimize.reduce(OptimizeAction::PreviewStarted(2));
        app.optimize.reduce(OptimizeAction::PreviewFinished {
            job_id: 2,
            plan,
            preview,
        });
        app.optimize.reduce(OptimizeAction::OpenConfirmation);
        app.optimize.reduce(OptimizeAction::RunStarted(3));
        app.optimize.reduce(OptimizeAction::RunFinished {
            job_id: 3,
            report: report(
                "settings.search",
                MaintenanceActionClass::SettingsHandoff,
                MaintenanceExecutionOutcome::Launched,
            ),
        });
        let rendered = render_text_with_size(&app, 100, 32);
        assert!(rendered.contains(badge), "{rendered}");
        assert!(rendered.contains(launched), "{rendered}");
        assert!(rendered.contains("settings.search"), "{rendered}");
        assert!(rendered.contains("guidance.drive_optimize"), "{rendered}");
        assert!(rendered.contains(guidance), "{rendered}");
        assert!(
            !rendered.to_lowercase().contains("optimization complete"),
            "{rendered}"
        );
        for (width, height) in [(40, 24), (80, 28), (120, 36)] {
            let scaled = render_text_with_size(&app, width, height);
            assert!(scaled.contains(badge), "width {width}: {scaled}");
            assert!(
                scaled.contains("Settings launched") || scaled.contains("设置已启动"),
                "width {width}: {scaled}"
            );
            assert!(
                scaled.contains("completed optimization") || scaled.contains("已完成的优化"),
                "width {width}: {scaled}"
            );
            assert!(
                scaled.contains("No run") || scaled.contains("无运行"),
                "width {width}: {scaled}"
            );
            assert!(
                !scaled.to_lowercase().contains("optimization complete"),
                "width {width}: {scaled}"
            );
        }
    }
}
