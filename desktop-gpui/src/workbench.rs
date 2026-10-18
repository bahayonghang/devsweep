//! Pure software and Optimize workbench identity mapping.
//!
//! The shell classifies a request into a review argv or an unavailable
//! explanation. This module does not spawn a process, scan, or mutate.

use devsweep_core::optimize::{MaintenanceActionClass, published_entries};

/// Software workbench segment. Leftovers is requestable only so it can be refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoftwareSegment {
    Uninstall,
    Updates,
    Startup,
    Leftovers,
}

/// A workbench request the native shell may classify.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkbenchAction<'a> {
    /// One published Optimize catalogue identity, or an unknown id.
    Catalogue(&'a str),
    /// One software segment, including leftovers which is not a landing segment.
    Software(SoftwareSegment),
}

/// Review argv for an accepted identity, or a non-empty capability explanation.
///
/// Unavailable never carries argv and is never a success outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkbenchDecision {
    Review {
        id: &'static str,
        argv: Vec<&'static str>,
    },
    Unavailable {
        explanation: &'static str,
    },
}

const UNINSTALL_ID: &str = "uninstall";
const UPDATES_ID: &str = "updates";
const STARTUP_ID: &str = "startup";

const GUIDANCE_EXPLANATION: &str =
    "This catalogue row is guidance only. It has no dispatch and cannot run.";
const OBSERVE_EXPLANATION: &str =
    "This catalogue row is observation only. It has no dispatch and cannot run.";
const UNKNOWN_EXPLANATION: &str = "This identity is outside the published Optimize catalogue.";
const LEFTOVERS_EXPLANATION: &str =
    "Leftovers is not a software workbench segment. It is unavailable here.";

/// Map a workbench action to an accepted CLI identity, or refuse it.
pub fn resolve(action: WorkbenchAction<'_>) -> WorkbenchDecision {
    match action {
        WorkbenchAction::Catalogue(id) => resolve_catalogue(id),
        WorkbenchAction::Software(segment) => resolve_software(segment),
    }
}

/// Classify the four software landing identities. Leftovers stays unavailable.
/// This does not spawn a process or record a success report.
pub fn software_landing() -> [WorkbenchDecision; 4] {
    [
        resolve(WorkbenchAction::Software(SoftwareSegment::Uninstall)),
        resolve(WorkbenchAction::Software(SoftwareSegment::Updates)),
        resolve(WorkbenchAction::Software(SoftwareSegment::Startup)),
        resolve(WorkbenchAction::Software(SoftwareSegment::Leftovers)),
    ]
}

fn resolve_catalogue(id: &str) -> WorkbenchDecision {
    let Some(entry) = published_entries().iter().find(|entry| entry.id == id) else {
        return WorkbenchDecision::Unavailable {
            explanation: UNKNOWN_EXPLANATION,
        };
    };
    match entry.action_class {
        MaintenanceActionClass::Execute | MaintenanceActionClass::SettingsHandoff => {
            WorkbenchDecision::Review {
                id: entry.id,
                argv: optimize_plan_review_argv(entry.id),
            }
        }
        MaintenanceActionClass::Guidance => WorkbenchDecision::Unavailable {
            explanation: GUIDANCE_EXPLANATION,
        },
        MaintenanceActionClass::Observe => WorkbenchDecision::Unavailable {
            explanation: OBSERVE_EXPLANATION,
        },
    }
}

fn resolve_software(segment: SoftwareSegment) -> WorkbenchDecision {
    match segment {
        SoftwareSegment::Uninstall => WorkbenchDecision::Review {
            id: UNINSTALL_ID,
            argv: vec!["software", "plan"],
        },
        SoftwareSegment::Updates => WorkbenchDecision::Review {
            id: UPDATES_ID,
            argv: vec!["software", "updates", "check"],
        },
        SoftwareSegment::Startup => WorkbenchDecision::Review {
            id: STARTUP_ID,
            argv: vec!["software", "startup", "list"],
        },
        SoftwareSegment::Leftovers => WorkbenchDecision::Unavailable {
            explanation: LEFTOVERS_EXPLANATION,
        },
    }
}

fn optimize_plan_review_argv(id: &'static str) -> Vec<&'static str> {
    vec!["optimize", "plan", "--operation", id]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims_operation_succeeded(text: &str) -> bool {
        let lower = text.to_ascii_lowercase();
        lower.contains("succeed") || lower.contains("success")
    }

    fn unavailable_explanation(decision: &WorkbenchDecision) -> &str {
        match decision {
            WorkbenchDecision::Unavailable { explanation } => explanation,
            WorkbenchDecision::Review { id, argv } => {
                panic!("expected unavailable, got review id={id} argv={argv:?}")
            }
        }
    }

    fn review_parts(decision: WorkbenchDecision) -> (&'static str, Vec<&'static str>) {
        match decision {
            WorkbenchDecision::Review { id, argv } => (id, argv),
            WorkbenchDecision::Unavailable { explanation } => {
                panic!("expected review, got unavailable: {explanation}")
            }
        }
    }

    #[test]
    fn every_published_catalogue_id_is_classified() {
        let entries = published_entries();
        assert!(
            !entries.is_empty(),
            "published_entries must expose the real catalogue"
        );
        for entry in entries {
            let decision = resolve(WorkbenchAction::Catalogue(entry.id));
            match entry.action_class {
                MaintenanceActionClass::Execute | MaintenanceActionClass::SettingsHandoff => {
                    let (id, argv) = review_parts(decision);
                    assert_eq!(id, entry.id);
                    assert_eq!(argv, ["optimize", "plan", "--operation", entry.id]);
                    assert!(
                        !argv.contains(&"run"),
                        "{} must not emit optimize run",
                        entry.id
                    );
                }
                MaintenanceActionClass::Guidance | MaintenanceActionClass::Observe => {
                    let explanation = unavailable_explanation(&decision);
                    assert!(!explanation.is_empty(), "{} explanation is empty", entry.id);
                    assert!(
                        !claims_operation_succeeded(explanation),
                        "{} explanation claims success: {explanation}",
                        entry.id
                    );
                }
            }
        }
    }

    #[test]
    fn guidance_and_observe_are_unavailable_explanations() {
        for entry in published_entries() {
            if !matches!(
                entry.action_class,
                MaintenanceActionClass::Guidance | MaintenanceActionClass::Observe
            ) {
                continue;
            }
            let decision = resolve(WorkbenchAction::Catalogue(entry.id));
            let explanation = unavailable_explanation(&decision);
            assert!(!explanation.is_empty());
            assert!(!claims_operation_succeeded(explanation));
        }
    }

    #[test]
    fn execute_and_settings_handoff_map_to_optimize_plan_review() {
        for entry in published_entries() {
            if !matches!(
                entry.action_class,
                MaintenanceActionClass::Execute | MaintenanceActionClass::SettingsHandoff
            ) {
                continue;
            }
            let (id, argv) = review_parts(resolve(WorkbenchAction::Catalogue(entry.id)));
            assert_eq!(id, entry.id);
            assert_eq!(argv, ["optimize", "plan", "--operation", entry.id]);
        }
    }

    #[test]
    fn image_scan_does_not_resolve_image_repair() {
        let (id, argv) = review_parts(resolve(WorkbenchAction::Catalogue(
            "maintenance.image_scan",
        )));
        assert_eq!(id, "maintenance.image_scan");
        assert_ne!(id, "maintenance.image_repair");
        assert!(!id.contains("maintenance.image_repair"));
        assert!(
            !argv
                .iter()
                .any(|word| word.contains("maintenance.image_repair")),
            "image scan argv contains image repair: {argv:?}"
        );
        assert_eq!(
            argv,
            ["optimize", "plan", "--operation", "maintenance.image_scan"]
        );
    }

    #[test]
    fn unknown_id_and_leftovers_are_unavailable() {
        let unknown = resolve(WorkbenchAction::Catalogue("not.a.catalogue.id"));
        let unknown_explanation = unavailable_explanation(&unknown);
        assert!(!unknown_explanation.is_empty());
        assert!(!claims_operation_succeeded(unknown_explanation));

        let leftovers = resolve(WorkbenchAction::Software(SoftwareSegment::Leftovers));
        let leftovers_explanation = unavailable_explanation(&leftovers);
        assert!(!leftovers_explanation.is_empty());
        assert!(!claims_operation_succeeded(leftovers_explanation));
    }

    #[test]
    fn software_segments_map_to_review_argv() {
        let (_, uninstall) = review_parts(resolve(WorkbenchAction::Software(
            SoftwareSegment::Uninstall,
        )));
        assert_eq!(uninstall, ["software", "plan"]);

        let (_, updates) =
            review_parts(resolve(WorkbenchAction::Software(SoftwareSegment::Updates)));
        assert_eq!(updates, ["software", "updates", "check"]);

        let (_, startup) =
            review_parts(resolve(WorkbenchAction::Software(SoftwareSegment::Startup)));
        assert_eq!(startup, ["software", "startup", "list"]);
    }

    #[test]
    fn software_landing_has_four_classified_identities() {
        let landing = software_landing();
        assert_eq!(landing.len(), 4);

        let (_, uninstall) = review_parts(landing[0].clone());
        assert_eq!(uninstall, ["software", "plan"]);

        let (_, updates) = review_parts(landing[1].clone());
        assert_eq!(updates, ["software", "updates", "check"]);

        let (_, startup) = review_parts(landing[2].clone());
        assert_eq!(startup, ["software", "startup", "list"]);

        let leftovers = unavailable_explanation(&landing[3]);
        assert_eq!(leftovers, LEFTOVERS_EXPLANATION);
        assert!(!leftovers.is_empty());
        assert!(!claims_operation_succeeded(leftovers));
    }
}
