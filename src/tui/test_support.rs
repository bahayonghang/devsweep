use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};

use crate::model::{
    CLEANUP_PLAN_VERSION, CleanAction, CleanTarget, CleanupPlan, Ecosystem, Evidence, RiskLevel,
    Scope, TargetId, TargetKind,
};

use super::{
    app::{App, UiEvent},
    render::{action_summary, render_app},
};

pub(super) fn key(code: KeyCode) -> UiEvent {
    UiEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

pub(super) fn render_text(app: &App) -> String {
    render_text_with_size(app, 120, 32)
}

pub(super) fn render_text_with_size(app: &App, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| render_app(frame, app))
        .expect("app renders");
    format!("{}", terminal.backend())
}

pub(super) fn representative_plan() -> CleanupPlan {
    CleanupPlan {
        version: CLEANUP_PLAN_VERSION,
        targets: vec![
            target(
                "node.next_cache",
                Scope::Project {
                    root: PathBuf::from("D:/code/web"),
                },
                Ecosystem::Node,
                TargetKind::BuildArtifacts,
                Some(PathBuf::from("D:/code/web/.next/cache")),
                1024,
                RiskLevel::Low,
                true,
                true,
                CleanAction::MoveToTrash {
                    path: PathBuf::from("D:/code/web/.next/cache"),
                },
            ),
            target(
                "npm.cache.clean",
                Scope::Global,
                Ecosystem::Node,
                TargetKind::PackageCache,
                Some(PathBuf::from("C:/Users/me/AppData/Local/npm-cache")),
                2048,
                RiskLevel::Medium,
                false,
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
            target(
                "cargo.home.inspect",
                Scope::Global,
                Ecosystem::Rust,
                TargetKind::PackageCache,
                Some(PathBuf::from("C:/Users/me/.cargo")),
                0,
                RiskLevel::High,
                false,
                true,
                CleanAction::NoopInspectOnly,
            ),
        ],
    }
}

pub(super) fn plan_with_targets(targets: Vec<CleanTarget>) -> CleanupPlan {
    CleanupPlan {
        version: CLEANUP_PLAN_VERSION,
        targets,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn target(
    rule_id: &str,
    scope: Scope,
    ecosystem: Ecosystem,
    kind: TargetKind,
    path: Option<PathBuf>,
    estimated_bytes: u64,
    risk: RiskLevel,
    selected_by_default: bool,
    reversible: bool,
    action: CleanAction,
) -> CleanTarget {
    CleanTarget {
        id: TargetId::new(format!(
            "{rule_id}:{}",
            path.as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_string())
        )),
        scope,
        ecosystem,
        kind,
        path: path.clone(),
        estimated_bytes,
        last_modified: None,
        risk,
        reversible,
        selected_by_default,
        evidence: vec![
            Evidence::RuleMatched {
                rule_id: rule_id.to_string(),
            },
            Evidence::OfficialCommand {
                command: action_summary(&action),
            },
        ],
        action,
    }
}
