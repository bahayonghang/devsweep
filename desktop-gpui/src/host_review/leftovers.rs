//! A completed uninstall identifies the audit record. Discovery supplies choices only.
use super::{ActionResult, Intent, Plan, Prepared, UninstallSelection};
use crate::copy::{self, Locale};
use devsweep_core::{process::FlagCancelObserver, software::*};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Leftovers {
    pub(super) source: Arc<SoftwareInventoryV1>,
    pub id: String,
    pub operation: String,
    pub name: String,
    pub candidate: Option<Box<SoftwareLeftoverCandidateV1>>,
}

impl Leftovers {
    pub(crate) fn after(result: &ActionResult, inventory: &SoftwareInventoryV1) -> Option<Self> {
        let Intent::Uninstall(row) = &result.intent else {
            return None;
        };
        let outcome = result.uninstall.as_ref()?;
        if !row.allows_leftovers(outcome) {
            return None;
        }
        let entry = inventory.entries.iter().find(|entry| entry.id == row.id)?;
        if UninstallSelection::from_entry(entry).ok()?.ne(row) {
            return None;
        }
        let source = inventory.clone();
        Some(Self {
            source: Arc::new(source),
            id: row.id.clone(),
            name: row.name.clone(),
            operation: outcome.operation_id.clone(),
            candidate: None,
        })
    }

    pub(super) fn label(&self, locale: Locale) -> String {
        match &self.candidate {
            None => format!(
                "{} · {}",
                copy::pick(locale, "Review leftovers", "审阅卸载残留"),
                self.name
            ),
            Some(item) => format!(
                "{} · {}",
                copy::pick(locale, "Move to Recycle Bin", "移入回收站"),
                item.path.display()
            ),
        }
    }

    pub(super) fn prepare(&self) -> Result<Prepared, String> {
        let (plan, digest) = if let Some(candidate) = &self.candidate {
            let selection = SoftwareLeftoverSelectionV1 {
                software_id: self.id.clone(),
                uninstall_operation_id: self.operation.clone(),
                selected_candidate_ids: vec![candidate.id.clone()],
            };
            let (plan, _) =
                plan_software_leftovers(&self.source, &selection, None).map_err(code)?;
            let preview =
                preview_software_leftover_plan(&self.source, &plan, None).map_err(code)?;
            let [live] = preview.items.as_slice() else {
                return Err("stale_candidate".into());
            };
            if live.id != candidate.id
                || live.path != candidate.path
                || live.origin != candidate.origin
                || live.certainty != candidate.certainty
            {
                return Err("stale_candidate".into());
            }
            (Plan::Leftover(plan), preview.digest)
        } else {
            let preview =
                discover_software_leftovers(&self.source, std::slice::from_ref(&self.id), None)
                    .map_err(code)?;
            let app = preview
                .apps
                .into_iter()
                .find(|app| app.software_id == self.id)
                .ok_or("stale_candidate")?;
            (Plan::LeftoverChoices(app.candidates), String::new())
        };
        Ok(Prepared {
            intent: Intent::Leftovers(self.clone()),
            plan,
            digest,
        })
    }

    pub(crate) fn execute(
        &self,
        plan: &SoftwareLeftoverPlanV1,
        digest: &str,
        confirmed: bool,
        cancel: &Arc<FlagCancelObserver>,
        dispatch: impl FnOnce(
            &SoftwareInventoryV1,
            &SoftwareLeftoverPlanV1,
            &str,
            bool,
            Option<&Arc<FlagCancelObserver>>,
        ) -> Result<SoftwareLeftoverStrictReportV1, SoftwareLeftoverError>,
    ) -> Result<String, String> {
        let candidate = self.candidate.as_ref().ok_or("empty_selection")?;
        if !confirmed {
            return Err("confirmation_required".into());
        }
        if plan.software_id != self.id
            || plan.uninstall_operation_id != self.operation
            || plan.selected_candidate_ids.as_slice() != std::slice::from_ref(&candidate.id)
        {
            return Err("invalid_plan".into());
        }
        let report = dispatch(&self.source, plan, digest, confirmed, Some(cancel)).map_err(code)?;
        Ok(match report.outcome {
            SoftwareLeftoverStrictOutcome::Succeeded => match report.outcomes.as_slice() {
                [item] if candidate.id == item.candidate_id => match item.outcome {
                    SoftwareSupportOutcomeCode::Succeeded => "leftover_moved",
                    SoftwareSupportOutcomeCode::Failed => "leftover_failed",
                    SoftwareSupportOutcomeCode::Skipped => match item.error_code {
                        Some(SoftwareSupportErrorCode::NotPresent) => "leftover_missing",
                        Some(SoftwareSupportErrorCode::Protected) => "leftover_protected",
                        Some(SoftwareSupportErrorCode::UnsafePath) => "leftover_unsafe",
                        _ => "leftover_skipped",
                    },
                },
                _ => "leftover_audit_unknown",
            },
            SoftwareLeftoverStrictOutcome::Failed => "leftover_failed",
            SoftwareLeftoverStrictOutcome::Unknown => "leftover_audit_unknown",
            SoftwareLeftoverStrictOutcome::Refused => {
                report.error_code.as_deref().unwrap_or("leftover_skipped")
            }
        }
        .to_string())
    }
}

fn code(error: SoftwareLeftoverError) -> String {
    match error {
        SoftwareLeftoverError::IdentityUnavailable(_) => "candidate_changed",
        SoftwareLeftoverError::Canceled => "canceled_before_start",
        _ => error.code(),
    }
    .to_string()
}
