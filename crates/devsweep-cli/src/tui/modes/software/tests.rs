use devsweep_core::software::{
    SoftwareActionClass, SoftwareActionOutcomeV1, SoftwareAuditErrorCode, SoftwareEligibility,
    SoftwareInstalledState, SoftwareLastUsedEvidence, SoftwareLastUsedReason,
    SoftwarePreviewItemV1, SoftwareRebootEvidence, SoftwareSizeEvidence,
};

use super::*;
use crate::{
    i18n::Locale,
    tui::{app::App, shell::ShellComposition, test_support::render_text_with_size},
};

fn entry(id: &str, selectable_entry: bool) -> SoftwareEntryV1 {
    SoftwareEntryV1 {
        id: id.to_string(),
        identity: if selectable_entry {
            SoftwareIdentity::Msix {
                package_full_name: format!("{id}_1.0"),
            }
        } else {
            SoftwareIdentity::Msi {
                product_code: "{11111111-1111-1111-1111-111111111111}".to_string(),
                context: devsweep_core::software::MsiContext::Machine,
            }
        },
        scope: if selectable_entry {
            SoftwareScope::CurrentUser
        } else {
            SoftwareScope::Machine
        },
        display_name: Some(id.to_string()),
        publisher: None,
        version: None,
        provenance: Vec::new(),
        eligibility: SoftwareEligibility {
            state: if selectable_entry {
                SoftwareEligibilityState::Selectable
            } else {
                SoftwareEligibilityState::Manual
            },
            reason: if selectable_entry {
                SoftwareEligibilityReason::EligibleCurrentUserMsix
            } else {
                SoftwareEligibilityReason::MsiExecutionNotSupportedV1
            },
        },
        size: SoftwareSizeEvidence::Unknown {
            reason_code: "not_reported".to_string(),
        },
        last_used: SoftwareLastUsedEvidence::Unknown {
            reason_code: SoftwareLastUsedReason::NoSupportedExactSource,
        },
    }
}

fn inventory() -> SoftwareInventoryV1 {
    SoftwareInventoryV1 {
        version: 1,
        observed_at_unix_ms: 1,
        sources: Vec::new(),
        entries: vec![entry("eligible", true), entry("manual", false)],
        fingerprint: format!("sha256:{}", "1".repeat(64)),
    }
}

#[test]
fn exact_manual_identity_never_enters_selection() {
    let mut state = SoftwareModeState::default();
    state.reduce(SoftwareAction::InventoryStarted(1));
    state.reduce(SoftwareAction::InventoryFinished {
        job_id: 1,
        inventory: inventory(),
    });
    state.reduce(SoftwareAction::MoveCursor(1));
    state.reduce(SoftwareAction::ToggleFocused);
    assert!(state.selected_ids.is_empty());
    state.reduce(SoftwareAction::SelectAll);
    assert_eq!(state.selected_ids, BTreeSet::from(["eligible".to_string()]));
}

#[test]
fn stale_preview_is_rejected_and_selection_invalidates_authority() {
    let mut state = SoftwareModeState::default();
    state.reduce(SoftwareAction::InventoryStarted(1));
    state.reduce(SoftwareAction::InventoryFinished {
        job_id: 1,
        inventory: inventory(),
    });
    state.reduce(SoftwareAction::ToggleFocused);
    state.reduce(SoftwareAction::PreviewStarted(2));
    let plan = SoftwareSelectionPlanV1 {
        version: 1,
        inventory_fingerprint: format!("sha256:{}", "1".repeat(64)),
        inventory_observed_at_unix_ms: 1,
        expires_at_unix_ms: 2,
        selected_ids: vec!["eligible".to_string()],
    };
    let preview = SoftwarePreviewV1 {
        version: 1,
        inventory_fingerprint: plan.inventory_fingerprint.clone(),
        irreversible: true,
        selected: vec![SoftwarePreviewItemV1 {
            id: "eligible".to_string(),
            identity: SoftwareIdentity::Msix {
                package_full_name: "eligible_1.0".to_string(),
            },
            action_class: SoftwareActionClass::RemoveCurrentUserMsix,
            scope: SoftwareScope::CurrentUser,
            eligibility: SoftwareEligibilityReason::EligibleCurrentUserMsix,
            strategy_token: "remove_package_current_user".to_string(),
        }],
        digest: format!("sha256:{}", "2".repeat(64)),
    };
    state.reduce(SoftwareAction::PreviewFinished {
        job_id: 99,
        plan: plan.clone(),
        preview: preview.clone(),
    });
    assert_eq!(state.phase, SoftwarePhase::Previewing);
    state.reduce(SoftwareAction::PreviewFinished {
        job_id: 2,
        plan,
        preview,
    });
    assert_eq!(state.phase, SoftwarePhase::PreviewReady);
    state.reduce(SoftwareAction::ToggleFocused);
    assert!(state.preview.is_none());
    assert!(state.plan.is_none());
}

#[test]
fn restart_audit_accepts_only_matching_operation_and_preserves_unknown() {
    let report = SoftwareExecutionReportV1 {
        version: 1,
        irreversible: true,
        outcomes: vec![SoftwareActionOutcomeV1 {
            operation_id: "core-op".to_string(),
            software_id: "eligible".to_string(),
            outcome: SoftwareExecutionOutcome::UnknownAfterDispatch,
            installed_state: Some(SoftwareInstalledState::Unavailable),
            reboot_evidence: SoftwareRebootEvidence::None,
            error_code: Some(SoftwareAuditErrorCode::RequeryUnavailable),
            irreversible: true,
        }],
    };
    let mut state = SoftwareModeState::default();
    state.reduce(SoftwareAction::AuditStarted(7));
    state.reduce(SoftwareAction::AuditFinished {
        job_id: 8,
        report: report.clone(),
    });
    assert_eq!(state.phase, SoftwarePhase::Loading);
    state.reduce(SoftwareAction::AuditFinished { job_id: 7, report });
    assert_eq!(state.phase, SoftwarePhase::Unknown);
    assert_eq!(state.report.as_ref().unwrap().outcomes.len(), 1);
}

#[test]
fn bilingual_test_backend_keeps_manual_unknown_and_irreversible_copy_visible() {
    for (locale, expected_manual, expected_unknown, expected_irreversible) in [
        (
            Locale::En,
            "MSI uninstall is manual",
            "unknown",
            "DevSweep cannot restore or reinstall software",
        ),
        (
            Locale::ZhCn,
            "Software V1 仅手动处理 MSI 卸载",
            "未知",
            "DevSweep 无法还原或重新安装软件",
        ),
    ] {
        let mut shell = ShellComposition::for_locale(locale).expect("shell");
        assert!(shell.activate(crate::tui::shell::ModeId::Software));
        let mut app = App::with_shell(shell);
        app.software.reduce(SoftwareAction::InventoryStarted(1));
        app.software.reduce(SoftwareAction::InventoryFinished {
            job_id: 1,
            inventory: inventory(),
        });
        app.software.cursor = 1;
        let rendered = render_text_with_size(&app, 100, 30);
        assert!(rendered.contains(expected_manual), "{rendered}");
        assert!(rendered.contains(expected_unknown), "{rendered}");
        assert!(rendered.contains(expected_irreversible), "{rendered}");
        assert!(rendered.contains("MSI:Machine"), "{rendered}");
    }
}
