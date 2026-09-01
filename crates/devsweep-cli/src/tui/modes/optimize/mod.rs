//! Optimize V1 presentation state. Authority remains in `devsweep_core::optimize`.

use devsweep_core::optimize::{
    MaintenanceActionClass, MaintenanceActionOutcomeV1, MaintenanceCatalogueEntryV1,
    MaintenanceExecutionOutcome, MaintenanceExecutionReportV1, MaintenancePlanV1,
    MaintenancePreviewV1, OptimizeAuditRecordV1, catalogue_entries,
};

use crate::tui::app::JobId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::tui) enum OptimizePhase {
    Checking,
    Ready,
    Selected,
    Previewing,
    PreviewReady,
    Confirming,
    Running,
    Launching,
    Terminal,
    Unknown,
    Canceling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::tui) enum OptimizeOperation {
    List,
    Preview,
    Run,
    Audit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::tui) struct OptimizeModeState {
    pub(in crate::tui) phase: OptimizePhase,
    pub(in crate::tui) resume_phase: OptimizePhase,
    pub(in crate::tui) operation_id: Option<JobId>,
    pub(in crate::tui) operation: Option<OptimizeOperation>,
    pub(in crate::tui) entries: Option<Vec<MaintenanceCatalogueEntryV1>>,
    pub(in crate::tui) selected_id: Option<String>,
    pub(in crate::tui) plan: Option<MaintenancePlanV1>,
    pub(in crate::tui) preview: Option<MaintenancePreviewV1>,
    pub(in crate::tui) report: Option<MaintenanceExecutionReportV1>,
    pub(in crate::tui) recovered: Vec<MaintenanceActionOutcomeV1>,
    pub(in crate::tui) audit: Vec<OptimizeAuditRecordV1>,
    pub(in crate::tui) cursor: usize,
    pub(in crate::tui) error: Option<String>,
}

impl Default for OptimizeModeState {
    fn default() -> Self {
        Self {
            phase: OptimizePhase::Ready,
            resume_phase: OptimizePhase::Ready,
            operation_id: None,
            operation: None,
            entries: None,
            selected_id: None,
            plan: None,
            preview: None,
            report: None,
            recovered: Vec::new(),
            audit: Vec::new(),
            cursor: 0,
            error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::tui) enum OptimizeAction {
    ListStarted(JobId),
    ListFinished {
        job_id: JobId,
        entries: Vec<MaintenanceCatalogueEntryV1>,
    },
    SelectFocused,
    MoveCursor(isize),
    PreviewStarted(JobId),
    PreviewFinished {
        job_id: JobId,
        plan: MaintenancePlanV1,
        preview: MaintenancePreviewV1,
    },
    OpenConfirmation,
    CloseConfirmation,
    RunStarted(JobId),
    RunFinished {
        job_id: JobId,
        report: MaintenanceExecutionReportV1,
    },
    AuditStarted(JobId),
    AuditFinished {
        job_id: JobId,
        recovered: Vec<MaintenanceActionOutcomeV1>,
        records: Vec<OptimizeAuditRecordV1>,
    },
    CancelRequested(JobId),
    Canceled(JobId),
    Failed {
        job_id: JobId,
        message: String,
    },
    Released,
}

impl OptimizeModeState {
    pub(in crate::tui) fn reduce(&mut self, action: OptimizeAction) {
        match action {
            OptimizeAction::ListStarted(job_id) if self.operation_id.is_none() => {
                self.phase = OptimizePhase::Checking;
                self.operation_id = Some(job_id);
                self.operation = Some(OptimizeOperation::List);
                self.error = None;
            }
            OptimizeAction::ListFinished { job_id, entries }
                if self.matches(job_id, OptimizeOperation::List)
                    && catalogue_matches_closed_set(&entries) =>
            {
                self.phase = OptimizePhase::Ready;
                self.resume_phase = OptimizePhase::Ready;
                self.operation_id = None;
                self.operation = None;
                self.entries = Some(entries);
                self.selected_id = None;
                self.plan = None;
                self.preview = None;
                self.report = None;
                self.cursor = 0;
                self.error = None;
            }
            OptimizeAction::SelectFocused
                if matches!(
                    self.phase,
                    OptimizePhase::Ready
                        | OptimizePhase::Selected
                        | OptimizePhase::PreviewReady
                        | OptimizePhase::Terminal
                        | OptimizePhase::Unknown
                ) =>
            {
                let Some(id) = self.focused_entry().map(|entry| entry.id.to_string()) else {
                    return;
                };
                self.selected_id = Some(id);
                self.invalidate_preview();
            }
            OptimizeAction::MoveCursor(delta) => {
                let count = self.visible_entries().len();
                if count > 0 {
                    self.cursor = self.cursor.saturating_add_signed(delta).min(count - 1);
                }
            }
            OptimizeAction::PreviewStarted(job_id)
                if self.operation_id.is_none()
                    && self
                        .selected_entry()
                        .is_some_and(|entry| dispatchable(entry.action_class)) =>
            {
                self.phase = OptimizePhase::Previewing;
                self.operation_id = Some(job_id);
                self.operation = Some(OptimizeOperation::Preview);
                self.error = None;
            }
            OptimizeAction::PreviewFinished {
                job_id,
                plan,
                preview,
            } if self.matches(job_id, OptimizeOperation::Preview)
                && self.selected_id.as_deref() == Some(plan.operation_id.as_str())
                && self.selected_id.as_deref() == Some(preview.operation_id.as_str())
                && self
                    .selected_entry()
                    .is_some_and(|entry| entry.action_class == preview.action_class)
                && dispatchable(preview.action_class) =>
            {
                self.phase = OptimizePhase::PreviewReady;
                self.resume_phase = OptimizePhase::PreviewReady;
                self.operation_id = None;
                self.operation = None;
                self.plan = Some(plan);
                self.preview = Some(preview);
                self.report = None;
                self.error = None;
            }
            OptimizeAction::OpenConfirmation
                if self.phase == OptimizePhase::PreviewReady
                    && self.plan.is_some()
                    && self.preview.is_some() =>
            {
                self.phase = OptimizePhase::Confirming;
            }
            OptimizeAction::CloseConfirmation if self.phase == OptimizePhase::Confirming => {
                self.phase = OptimizePhase::PreviewReady;
            }
            OptimizeAction::RunStarted(job_id)
                if self.phase == OptimizePhase::Confirming
                    && self.operation_id.is_none()
                    && self.plan.is_some()
                    && self.preview.is_some() =>
            {
                self.phase = match self.preview.as_ref().map(|preview| preview.action_class) {
                    Some(MaintenanceActionClass::SettingsHandoff) => OptimizePhase::Launching,
                    _ => OptimizePhase::Running,
                };
                self.operation_id = Some(job_id);
                self.operation = Some(OptimizeOperation::Run);
                self.error = None;
            }
            OptimizeAction::RunFinished { job_id, report }
                if self.matches(job_id, OptimizeOperation::Run)
                    && self.preview.as_ref().is_some_and(|preview| {
                        report.outcomes.len() == 1
                            && report.outcomes[0].catalogue_id == preview.operation_id
                            && report.outcomes[0].action_class == preview.action_class
                    }) =>
            {
                let unknown = report.outcomes.iter().any(|outcome| {
                    outcome.outcome == MaintenanceExecutionOutcome::UnknownAfterDispatch
                });
                self.phase = if unknown {
                    OptimizePhase::Unknown
                } else {
                    OptimizePhase::Terminal
                };
                self.resume_phase = self.phase;
                self.operation_id = None;
                self.operation = None;
                self.report = Some(report);
                self.error = None;
            }
            OptimizeAction::AuditStarted(job_id) if self.operation_id.is_none() => {
                self.resume_phase = self.phase;
                self.phase = OptimizePhase::Checking;
                self.operation_id = Some(job_id);
                self.operation = Some(OptimizeOperation::Audit);
                self.error = None;
            }
            OptimizeAction::AuditFinished {
                job_id,
                recovered,
                records,
            } if self.matches(job_id, OptimizeOperation::Audit) => {
                self.phase = self.resume_phase;
                self.operation_id = None;
                self.operation = None;
                self.recovered = recovered;
                self.audit = records;
                self.error = None;
            }
            OptimizeAction::CancelRequested(job_id) if self.operation_id == Some(job_id) => {
                self.phase = OptimizePhase::Canceling;
            }
            OptimizeAction::Canceled(job_id) if self.operation_id == Some(job_id) => {
                self.operation_id = None;
                self.operation = None;
                self.phase = if self.preview.is_some() {
                    OptimizePhase::PreviewReady
                } else if self.selected_id.is_some() {
                    OptimizePhase::Selected
                } else {
                    OptimizePhase::Ready
                };
                self.resume_phase = self.phase;
            }
            OptimizeAction::Failed { job_id, message } if self.operation_id == Some(job_id) => {
                let audit = self.operation == Some(OptimizeOperation::Audit);
                self.operation_id = None;
                self.operation = None;
                self.phase = if audit {
                    self.resume_phase
                } else {
                    OptimizePhase::Unknown
                };
                self.error = Some(message);
            }
            OptimizeAction::Released => *self = Self::default(),
            _ => {}
        }
    }

    pub(in crate::tui) fn visible_entries(&self) -> &[MaintenanceCatalogueEntryV1] {
        self.entries.as_deref().unwrap_or(&[])
    }

    pub(in crate::tui) fn focused_entry(&self) -> Option<&MaintenanceCatalogueEntryV1> {
        self.visible_entries().get(self.cursor)
    }

    pub(in crate::tui) fn selected_entry(&self) -> Option<&MaintenanceCatalogueEntryV1> {
        let selected = self.selected_id.as_deref()?;
        self.visible_entries()
            .iter()
            .find(|entry| entry.id == selected)
    }

    fn matches(&self, job_id: JobId, operation: OptimizeOperation) -> bool {
        self.operation_id == Some(job_id) && self.operation == Some(operation)
    }

    fn invalidate_preview(&mut self) {
        self.phase = OptimizePhase::Selected;
        self.resume_phase = OptimizePhase::Selected;
        self.plan = None;
        self.preview = None;
        self.report = None;
    }
}

pub(in crate::tui) fn dispatchable(action_class: MaintenanceActionClass) -> bool {
    matches!(
        action_class,
        MaintenanceActionClass::Execute | MaintenanceActionClass::SettingsHandoff
    )
}

fn catalogue_matches_closed_set(entries: &[MaintenanceCatalogueEntryV1]) -> bool {
    let expected = catalogue_entries();
    entries.len() == expected.len()
        && entries.iter().zip(expected.iter()).all(|(actual, wanted)| {
            actual.id == wanted.id && actual.action_class == wanted.action_class
        })
}

#[cfg(test)]
mod tests;
