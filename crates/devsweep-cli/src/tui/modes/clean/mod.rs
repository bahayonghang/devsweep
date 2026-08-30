//! Clean-local TUI reducer. Domain authority never enters the shell.
#![allow(dead_code)]
use std::collections::HashSet;

use devsweep_core::{
    execution::{ConfirmationDigest, ExecutionReport},
    model::{CleanupIntent, ScanReport, TargetId, UntrustedTarget},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CleanPhase {
    Idle,
    Scanning,
    ReportReady,
    Selecting,
    Previewing,
    PreviewReady,
    Confirming,
    Executing,
    Completed,
    Partial,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CleanModeState {
    pub(crate) phase: CleanPhase,
    pub(crate) operation_id: Option<String>,
    pub(crate) report: Option<ScanReport>,
    pub(crate) selected_ids: HashSet<TargetId>,
    pub(crate) preview: Option<ExecutionReport>,
    pub(crate) query: String,
    pub(crate) last_sequence: u64,
}

impl Default for CleanModeState {
    fn default() -> Self {
        Self {
            phase: CleanPhase::Idle,
            operation_id: None,
            report: None,
            selected_ids: HashSet::new(),
            preview: None,
            query: String::new(),
            last_sequence: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CleanAction {
    ScanRequested {
        operation_id: String,
    },
    ScanProgressed {
        operation_id: String,
        sequence: u64,
    },
    ScanCompleted {
        operation_id: String,
        report: ScanReport,
    },
    ScanCanceled {
        operation_id: String,
    },
    SelectionChanged {
        target_id: TargetId,
        selected: bool,
    },
    SelectAll {
        selected: bool,
    },
    QueryChanged {
        query: String,
    },
    PreviewRequested,
    PreviewReady {
        report: ExecutionReport,
    },
    ConfirmOpened,
    ConfirmClosed,
    ExecuteRequested,
    ExecuteCompleted {
        report: ExecutionReport,
    },
    StalePreview,
}

pub(crate) fn reduce(state: CleanModeState, action: CleanAction) -> CleanModeState {
    match action {
        CleanAction::ScanRequested { operation_id } => CleanModeState {
            phase: CleanPhase::Scanning,
            operation_id: Some(operation_id),
            selected_ids: HashSet::new(),
            preview: None,
            last_sequence: 0,
            ..state
        },
        CleanAction::ScanProgressed {
            operation_id,
            sequence,
        } => {
            if state.operation_id.as_deref() != Some(operation_id.as_str())
                || sequence <= state.last_sequence
                || state.phase != CleanPhase::Scanning
            {
                return state;
            }
            CleanModeState {
                last_sequence: sequence,
                ..state
            }
        }
        CleanAction::ScanCompleted {
            operation_id,
            report,
        } => {
            if state.operation_id.as_deref() != Some(operation_id.as_str()) {
                return state;
            }
            let selected_ids = default_selected(&report);
            CleanModeState {
                phase: CleanPhase::ReportReady,
                report: Some(report),
                selected_ids,
                preview: None,
                ..state
            }
        }
        CleanAction::ScanCanceled { operation_id } => {
            if state.operation_id.as_deref() != Some(operation_id.as_str()) {
                return state;
            }
            CleanModeState {
                phase: CleanPhase::Canceled,
                selected_ids: HashSet::new(),
                preview: None,
                ..state
            }
        }
        CleanAction::SelectionChanged {
            target_id,
            selected,
        } => {
            if !matches!(
                state.phase,
                CleanPhase::ReportReady | CleanPhase::Selecting | CleanPhase::PreviewReady
            ) {
                return state;
            }
            if !is_executable(state.report.as_ref(), &target_id) {
                return state;
            }
            let mut selected_ids = state.selected_ids.clone();
            if selected {
                selected_ids.insert(target_id);
            } else {
                selected_ids.remove(&target_id);
            }
            CleanModeState {
                phase: CleanPhase::Selecting,
                selected_ids,
                preview: None,
                ..state
            }
        }
        CleanAction::SelectAll { selected } => {
            if state.report.is_none() || state.phase == CleanPhase::Scanning {
                return state;
            }
            let selected_ids = if selected {
                executable_ids(state.report.as_ref())
            } else {
                HashSet::new()
            };
            CleanModeState {
                phase: CleanPhase::Selecting,
                selected_ids,
                preview: None,
                ..state
            }
        }
        CleanAction::QueryChanged { query } => CleanModeState { query, ..state },
        CleanAction::PreviewRequested => {
            if state.selected_ids.is_empty() || state.report.is_none() {
                return state;
            }
            CleanModeState {
                phase: CleanPhase::Previewing,
                ..state
            }
        }
        CleanAction::PreviewReady { report } => CleanModeState {
            phase: CleanPhase::PreviewReady,
            preview: Some(report),
            ..state
        },
        CleanAction::ConfirmOpened => {
            if state.preview.is_none() {
                return state;
            }
            CleanModeState {
                phase: CleanPhase::Confirming,
                ..state
            }
        }
        CleanAction::ConfirmClosed => CleanModeState {
            phase: CleanPhase::PreviewReady,
            ..state
        },
        CleanAction::ExecuteRequested => {
            if state.phase != CleanPhase::Confirming || state.preview.is_none() {
                return state;
            }
            CleanModeState {
                phase: CleanPhase::Executing,
                ..state
            }
        }
        CleanAction::ExecuteCompleted { report } => {
            let phase = if report.failed > 0 && report.succeeded > 0 {
                CleanPhase::Partial
            } else if report.failed > 0 {
                CleanPhase::Failed
            } else {
                CleanPhase::Completed
            };
            CleanModeState { phase, ..state }
        }
        CleanAction::StalePreview => CleanModeState {
            phase: CleanPhase::Selecting,
            preview: None,
            ..state
        },
    }
}

fn default_selected(report: &ScanReport) -> HashSet<TargetId> {
    report
        .plan
        .targets
        .iter()
        .filter(|target| target.selected_by_default && is_target_executable(target))
        .map(|target| target.id.clone())
        .collect()
}

fn executable_ids(report: Option<&ScanReport>) -> HashSet<TargetId> {
    report
        .map(|report| {
            report
                .plan
                .targets
                .iter()
                .filter(|target| is_target_executable(target))
                .map(|target| target.id.clone())
                .collect()
        })
        .unwrap_or_default()
}

fn is_executable(report: Option<&ScanReport>, target_id: &TargetId) -> bool {
    report
        .and_then(|report| {
            report
                .plan
                .targets
                .iter()
                .find(|target| &target.id == target_id)
        })
        .is_some_and(is_target_executable)
}

fn is_target_executable(target: &UntrustedTarget) -> bool {
    !matches!(target.intent, CleanupIntent::InspectOnly { .. })
}

pub(crate) fn current_digest(state: &CleanModeState) -> Option<&ConfirmationDigest> {
    state
        .preview
        .as_ref()
        .map(|report| &report.confirmation_digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use devsweep_core::model::{
        CLEANUP_PLAN_VERSION, Ecosystem, Evidence, RiskLevel, ScanHealth, Scope, TargetKind,
        UntrustedPlan,
    };

    fn report() -> ScanReport {
        ScanReport::new(
            UntrustedPlan {
                version: CLEANUP_PLAN_VERSION,
                targets: vec![target("ok", false), target("cargo.home.inspect:x", true)],
            },
            ScanHealth::complete(),
        )
    }

    fn target(id: &str, inspect_only: bool) -> UntrustedTarget {
        UntrustedTarget {
            id: TargetId::new(id),
            rule_id: "rust.target".to_string(),
            scope: Scope::Global,
            ecosystem: Ecosystem::Rust,
            kind: TargetKind::PackageCache,
            path: None,
            estimated_bytes: 1,
            size_complete: true,
            sizing_warnings: Vec::new(),
            last_modified: None,
            risk: RiskLevel::Low,
            reversible: true,
            selected_by_default: !inspect_only,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "rust.target".to_string(),
            }],
            intent: if inspect_only {
                CleanupIntent::InspectOnly {
                    rule_id: "cargo.home.inspect".to_string(),
                }
            } else {
                CleanupIntent::TrashProjectArtifact {
                    rule_id: "rust.target".to_string(),
                }
            },
        }
    }

    #[test]
    fn no_selection_during_scan_and_stale_events_are_ignored() {
        let mut state = reduce(
            CleanModeState::default(),
            CleanAction::ScanRequested {
                operation_id: "scan-1".into(),
            },
        );
        state = reduce(
            state,
            CleanAction::SelectionChanged {
                target_id: TargetId::new("ok"),
                selected: true,
            },
        );
        assert!(state.selected_ids.is_empty());
        state = reduce(
            state,
            CleanAction::ScanProgressed {
                operation_id: "scan-other".into(),
                sequence: 2,
            },
        );
        assert_eq!(state.last_sequence, 0);
        state = reduce(
            state,
            CleanAction::ScanCompleted {
                operation_id: "scan-1".into(),
                report: report(),
            },
        );
        assert_eq!(state.phase, CleanPhase::ReportReady);
        assert_eq!(state.selected_ids.len(), 1);
        assert!(
            !state
                .selected_ids
                .contains(&TargetId::new("cargo.home.inspect:x"))
        );
    }

    fn dry_run_report() -> ExecutionReport {
        ExecutionReport {
            dry_run: true,
            selected: 1,
            attempted: 1,
            succeeded: 0,
            failed: 0,
            skipped: 1,
            failures: Vec::new(),
            outcomes: Vec::new(),
            notes: Vec::new(),
            estimated_recoverable: Default::default(),
            confirmation_digest: ConfirmationDigest::new("abc"),
            audit_log: None,
        }
    }

    #[test]
    fn inspect_only_never_selects_and_selection_invalidates_preview() {
        let mut state = reduce(
            CleanModeState::default(),
            CleanAction::ScanRequested {
                operation_id: "scan-1".into(),
            },
        );
        state = reduce(
            state,
            CleanAction::ScanCompleted {
                operation_id: "scan-1".into(),
                report: report(),
            },
        );
        state = reduce(state, CleanAction::PreviewRequested);
        state = reduce(
            state,
            CleanAction::PreviewReady {
                report: dry_run_report(),
            },
        );
        assert_eq!(state.phase, CleanPhase::PreviewReady);
        state = reduce(
            state,
            CleanAction::SelectionChanged {
                target_id: TargetId::new("cargo.home.inspect:x"),
                selected: true,
            },
        );
        assert!(
            !state
                .selected_ids
                .contains(&TargetId::new("cargo.home.inspect:x"))
        );
        state = reduce(
            state,
            CleanAction::QueryChanged {
                query: "target".into(),
            },
        );
        assert_eq!(state.query, "target");
        state = reduce(
            state,
            CleanAction::SelectionChanged {
                target_id: TargetId::new("ok"),
                selected: false,
            },
        );
        assert!(state.preview.is_none());
    }

    #[test]
    fn execute_requires_preview_and_second_confirmation() {
        let mut state = reduce(
            CleanModeState::default(),
            CleanAction::ScanRequested {
                operation_id: "scan-1".into(),
            },
        );
        state = reduce(
            state,
            CleanAction::ScanCompleted {
                operation_id: "scan-1".into(),
                report: report(),
            },
        );
        state = reduce(state, CleanAction::ExecuteRequested);
        assert_eq!(state.phase, CleanPhase::ReportReady);
        state = reduce(state, CleanAction::PreviewRequested);
        state = reduce(
            state,
            CleanAction::PreviewReady {
                report: dry_run_report(),
            },
        );
        state = reduce(state, CleanAction::ConfirmOpened);
        state = reduce(state, CleanAction::ExecuteRequested);
        assert_eq!(state.phase, CleanPhase::Executing);
        state = reduce(state, CleanAction::StalePreview);
        assert_eq!(state.phase, CleanPhase::Selecting);
        assert!(state.preview.is_none());
    }
}
