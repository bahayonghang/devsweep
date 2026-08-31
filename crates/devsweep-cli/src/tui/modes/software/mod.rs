//! Software V1 presentation state. Authority remains in `devsweep_core::software`.

use std::collections::BTreeSet;

use devsweep_core::software::{
    SoftwareEligibilityReason, SoftwareEligibilityState, SoftwareEntryV1, SoftwareExecutionOutcome,
    SoftwareExecutionReportV1, SoftwareIdentity, SoftwareInventoryV1, SoftwarePreviewV1,
    SoftwareScope, SoftwareSelectionPlanV1,
};

use crate::tui::app::JobId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::tui) enum SoftwarePhase {
    Loading,
    Ready,
    Selecting,
    Previewing,
    PreviewReady,
    Confirming,
    Uninstalling,
    Terminal,
    Unknown,
    Canceling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::tui) enum SoftwareOperation {
    Inventory,
    Preview,
    Uninstall,
    Audit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::tui) struct SoftwareModeState {
    pub(in crate::tui) phase: SoftwarePhase,
    pub(in crate::tui) operation_id: Option<JobId>,
    pub(in crate::tui) operation: Option<SoftwareOperation>,
    pub(in crate::tui) inventory: Option<SoftwareInventoryV1>,
    pub(in crate::tui) selected_ids: BTreeSet<String>,
    pub(in crate::tui) plan: Option<SoftwareSelectionPlanV1>,
    pub(in crate::tui) preview: Option<SoftwarePreviewV1>,
    pub(in crate::tui) report: Option<SoftwareExecutionReportV1>,
    pub(in crate::tui) cursor: usize,
    pub(in crate::tui) query: String,
    pub(in crate::tui) error: Option<String>,
}

impl Default for SoftwareModeState {
    fn default() -> Self {
        Self {
            phase: SoftwarePhase::Ready,
            operation_id: None,
            operation: None,
            inventory: None,
            selected_ids: BTreeSet::new(),
            plan: None,
            preview: None,
            report: None,
            cursor: 0,
            query: String::new(),
            error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::tui) enum SoftwareAction {
    InventoryStarted(JobId),
    InventoryFinished {
        job_id: JobId,
        inventory: SoftwareInventoryV1,
    },
    ToggleFocused,
    SelectAll,
    MoveCursor(isize),
    QueryChanged(String),
    PreviewStarted(JobId),
    PreviewFinished {
        job_id: JobId,
        plan: SoftwareSelectionPlanV1,
        preview: SoftwarePreviewV1,
    },
    OpenConfirmation,
    CloseConfirmation,
    UninstallStarted(JobId),
    UninstallFinished {
        job_id: JobId,
        report: SoftwareExecutionReportV1,
    },
    AuditStarted(JobId),
    AuditFinished {
        job_id: JobId,
        report: SoftwareExecutionReportV1,
    },
    CancelRequested(JobId),
    Canceled(JobId),
    Failed {
        job_id: JobId,
        message: String,
    },
    Released,
}

impl SoftwareModeState {
    pub(in crate::tui) fn reduce(&mut self, action: SoftwareAction) {
        match action {
            SoftwareAction::InventoryStarted(job_id) if self.operation_id.is_none() => {
                self.phase = SoftwarePhase::Loading;
                self.operation_id = Some(job_id);
                self.operation = Some(SoftwareOperation::Inventory);
                self.error = None;
            }
            SoftwareAction::InventoryFinished { job_id, inventory }
                if self.matches(job_id, SoftwareOperation::Inventory) =>
            {
                self.phase = SoftwarePhase::Ready;
                self.operation_id = None;
                self.operation = None;
                self.inventory = Some(inventory);
                self.selected_ids.clear();
                self.plan = None;
                self.preview = None;
                self.report = None;
                self.cursor = 0;
                self.error = None;
            }
            SoftwareAction::ToggleFocused
                if matches!(
                    self.phase,
                    SoftwarePhase::Ready | SoftwarePhase::Selecting | SoftwarePhase::PreviewReady
                ) =>
            {
                let Some(id) = self
                    .focused_entry()
                    .filter(|entry| selectable(entry))
                    .map(|entry| entry.id.clone())
                else {
                    return;
                };
                if !self.selected_ids.remove(&id) {
                    self.selected_ids.insert(id);
                }
                self.invalidate_preview();
            }
            SoftwareAction::SelectAll
                if matches!(
                    self.phase,
                    SoftwarePhase::Ready | SoftwarePhase::Selecting | SoftwarePhase::PreviewReady
                ) =>
            {
                let selectable = self
                    .inventory
                    .iter()
                    .flat_map(|inventory| &inventory.entries)
                    .filter(|entry| selectable(entry))
                    .map(|entry| entry.id.clone())
                    .collect::<BTreeSet<_>>();
                if self.selected_ids == selectable {
                    self.selected_ids.clear();
                } else {
                    self.selected_ids = selectable;
                }
                self.invalidate_preview();
            }
            SoftwareAction::MoveCursor(delta) => {
                let count = self.visible_entries().len();
                if count > 0 {
                    self.cursor = self.cursor.saturating_add_signed(delta).min(count - 1);
                }
            }
            SoftwareAction::QueryChanged(query) => {
                self.query = query;
                self.cursor = 0;
            }
            SoftwareAction::PreviewStarted(job_id)
                if self.operation_id.is_none()
                    && self.inventory.is_some()
                    && !self.selected_ids.is_empty() =>
            {
                self.phase = SoftwarePhase::Previewing;
                self.operation_id = Some(job_id);
                self.operation = Some(SoftwareOperation::Preview);
                self.error = None;
            }
            SoftwareAction::PreviewFinished {
                job_id,
                plan,
                preview,
            } if self.matches(job_id, SoftwareOperation::Preview)
                && same_ids(&self.selected_ids, &plan.selected_ids)
                && same_ids(
                    &self.selected_ids,
                    &preview
                        .selected
                        .iter()
                        .map(|item| item.id.clone())
                        .collect::<Vec<_>>(),
                ) =>
            {
                self.phase = SoftwarePhase::PreviewReady;
                self.operation_id = None;
                self.operation = None;
                self.plan = Some(plan);
                self.preview = Some(preview);
                self.report = None;
                self.error = None;
            }
            SoftwareAction::OpenConfirmation
                if self.phase == SoftwarePhase::PreviewReady
                    && self.plan.is_some()
                    && self.preview.is_some() =>
            {
                self.phase = SoftwarePhase::Confirming;
            }
            SoftwareAction::CloseConfirmation if self.phase == SoftwarePhase::Confirming => {
                self.phase = SoftwarePhase::PreviewReady;
            }
            SoftwareAction::UninstallStarted(job_id)
                if self.phase == SoftwarePhase::Confirming
                    && self.operation_id.is_none()
                    && self.plan.is_some()
                    && self.preview.is_some() =>
            {
                self.phase = SoftwarePhase::Uninstalling;
                self.operation_id = Some(job_id);
                self.operation = Some(SoftwareOperation::Uninstall);
                self.error = None;
            }
            SoftwareAction::UninstallFinished { job_id, report }
                if self.matches(job_id, SoftwareOperation::Uninstall)
                    && self.preview.as_ref().is_some_and(|preview| {
                        same_ids(
                            &preview
                                .selected
                                .iter()
                                .map(|item| item.id.clone())
                                .collect::<BTreeSet<_>>(),
                            &report
                                .outcomes
                                .iter()
                                .map(|item| item.software_id.clone())
                                .collect::<Vec<_>>(),
                        )
                    }) =>
            {
                self.phase = if report.outcomes.iter().any(|outcome| {
                    outcome.outcome == SoftwareExecutionOutcome::UnknownAfterDispatch
                }) {
                    SoftwarePhase::Unknown
                } else {
                    SoftwarePhase::Terminal
                };
                self.operation_id = None;
                self.operation = None;
                self.report = Some(report);
                self.error = None;
            }
            SoftwareAction::AuditStarted(job_id) if self.operation_id.is_none() => {
                self.phase = SoftwarePhase::Loading;
                self.operation_id = Some(job_id);
                self.operation = Some(SoftwareOperation::Audit);
                self.error = None;
            }
            SoftwareAction::AuditFinished { job_id, report }
                if self.matches(job_id, SoftwareOperation::Audit) =>
            {
                self.phase = if report.outcomes.is_empty() {
                    SoftwarePhase::Ready
                } else if report.outcomes.iter().any(|outcome| {
                    outcome.outcome == SoftwareExecutionOutcome::UnknownAfterDispatch
                }) {
                    SoftwarePhase::Unknown
                } else {
                    SoftwarePhase::Terminal
                };
                self.operation_id = None;
                self.operation = None;
                self.report = Some(report);
                self.error = None;
            }
            SoftwareAction::CancelRequested(job_id) if self.operation_id == Some(job_id) => {
                self.phase = SoftwarePhase::Canceling;
            }
            SoftwareAction::Canceled(job_id) if self.operation_id == Some(job_id) => {
                self.operation_id = None;
                self.operation = None;
                self.phase = if self.preview.is_some() {
                    SoftwarePhase::PreviewReady
                } else {
                    SoftwarePhase::Ready
                };
            }
            SoftwareAction::Failed { job_id, message } if self.operation_id == Some(job_id) => {
                self.operation_id = None;
                self.operation = None;
                self.phase = SoftwarePhase::Unknown;
                self.error = Some(message);
            }
            SoftwareAction::Released => *self = Self::default(),
            _ => {}
        }
    }

    pub(in crate::tui) fn visible_entries(&self) -> Vec<&SoftwareEntryV1> {
        let query = self.query.trim().to_lowercase();
        self.inventory
            .iter()
            .flat_map(|inventory| &inventory.entries)
            .filter(|entry| {
                query.is_empty()
                    || entry.id.to_lowercase().contains(&query)
                    || entry
                        .display_name
                        .as_deref()
                        .is_some_and(|name| name.to_lowercase().contains(&query))
            })
            .collect()
    }

    pub(in crate::tui) fn focused_entry(&self) -> Option<&SoftwareEntryV1> {
        self.visible_entries().get(self.cursor).copied()
    }

    fn matches(&self, job_id: JobId, operation: SoftwareOperation) -> bool {
        self.operation_id == Some(job_id) && self.operation == Some(operation)
    }

    fn invalidate_preview(&mut self) {
        self.phase = SoftwarePhase::Selecting;
        self.plan = None;
        self.preview = None;
        self.report = None;
    }
}

pub(in crate::tui) fn selectable(entry: &SoftwareEntryV1) -> bool {
    entry.eligibility.state == SoftwareEligibilityState::Selectable
        && entry.eligibility.reason == SoftwareEligibilityReason::EligibleCurrentUserMsix
        && entry.scope == SoftwareScope::CurrentUser
        && matches!(entry.identity, SoftwareIdentity::Msix { .. })
}

fn same_ids(left: &BTreeSet<String>, right: &[String]) -> bool {
    left.len() == right.len() && right.iter().all(|id| left.contains(id))
}

#[cfg(test)]
mod tests;
