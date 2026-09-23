//! Binds the desktop fixtures to the real Tauri and core wire types.
//!
//! The React contract decoders read `desktop/src/api/fixtures/`. Without this
//! module those files are only a frontend mock: nothing proves the Rust side
//! produces or accepts the same JSON. Every fixture below is deserialized into
//! the type the shipped command actually returns and serialized back, so a
//! field added, removed, or renamed on either side fails here.

use devsweep_core::{
    analysis::{AnalyzeSnapshotV1, AnalyzeTrashPreviewV1, AnalyzeTrashReportV1},
    execution::ExecutionReport,
    history::CleanMovedTotalsV1,
    model::ScanReport,
    optimize::{OPTIMIZE_CATALOGUE_VERSION, catalogue_entries},
    software::{SoftwareInventoryV1, SoftwareStartupListV1, SoftwareStartupToggleReportV1},
    status::{StatusEventV1, StatusSnapshotV1},
};
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    analyze::{DesktopAnalyzeProgress, DesktopAnalyzeResult},
    clean::DryRunOutcome,
    error::CommandError,
    hud::HudStatusEvent,
    optimize::{
        DesktopOptimizeAuditResult, DesktopOptimizePreviewResult, DesktopOptimizeRunResult,
    },
    scan::DesktopScanProgress,
    software::{
        DesktopSoftwareAuditResult, DesktopSoftwareLeftoversPreviewResult,
        DesktopSoftwareLeftoversResult, DesktopSoftwarePreviewResult,
        DesktopSoftwareUninstallResult, DesktopSoftwareUpdatesResult,
    },
    status::{DesktopStatusLiveResult, DesktopStatusSnapshotResult},
};

/// Decodes one fixture into `T` and serializes it back.
///
/// The deserialize step fails when the fixture carries a field the Rust type
/// does not declare. The comparison fails when the Rust type emits a field the
/// fixture does not carry.
fn assert_round_trip<T>(label: &str, raw: &str)
where
    T: DeserializeOwned + Serialize,
{
    let fixture: serde_json::Value =
        serde_json::from_str(raw).unwrap_or_else(|error| panic!("{label}: invalid JSON: {error}"));
    let decoded: T = serde_json::from_value(fixture.clone())
        .unwrap_or_else(|error| panic!("{label}: fixture does not match the Rust type: {error}"));
    let encoded = serde_json::to_value(&decoded)
        .unwrap_or_else(|error| panic!("{label}: Rust value does not serialize: {error}"));
    assert_eq!(
        encoded, fixture,
        "{label}: Rust serialization differs from the desktop fixture"
    );
}

#[test]
fn clean_fixtures_match_the_clean_command_wire_types() {
    assert_round_trip::<ScanReport>(
        "clean/scan-report.json",
        include_str!("../../src/api/fixtures/clean/scan-report.json"),
    );
    assert_round_trip::<ScanReport>(
        "scan-report.json",
        include_str!("../../src/api/fixtures/scan-report.json"),
    );
    assert_round_trip::<ScanReport>(
        "scan-report.real.json",
        include_str!("../../src/api/fixtures/scan-report.real.json"),
    );
    assert_round_trip::<DesktopScanProgress>(
        "clean/scan-progress.json",
        include_str!("../../src/api/fixtures/clean/scan-progress.json"),
    );
    assert_round_trip::<DesktopScanProgress>(
        "scan-progress.json",
        include_str!("../../src/api/fixtures/scan-progress.json"),
    );
    assert_round_trip::<DryRunOutcome>(
        "clean/dry-run-outcome.json",
        include_str!("../../src/api/fixtures/clean/dry-run-outcome.json"),
    );
    assert_round_trip::<DryRunOutcome>(
        "dry-run-outcome.json",
        include_str!("../../src/api/fixtures/dry-run-outcome.json"),
    );
    assert_round_trip::<DryRunOutcome>(
        "dry-run-outcome-two-targets.json",
        include_str!("../../src/api/fixtures/dry-run-outcome-two-targets.json"),
    );
    assert_round_trip::<ExecutionReport>(
        "clean/execution-report.json",
        include_str!("../../src/api/fixtures/clean/execution-report.json"),
    );
    assert_round_trip::<ExecutionReport>(
        "execution-report.json",
        include_str!("../../src/api/fixtures/execution-report.json"),
    );
}

#[test]
fn analyze_fixtures_match_the_analyze_command_wire_types() {
    assert_round_trip::<DesktopAnalyzeResult>(
        "analyze/complete.json",
        include_str!("../../src/api/fixtures/analyze/complete.json"),
    );
    assert_round_trip::<DesktopAnalyzeProgress>(
        "analyze/progress.json",
        include_str!("../../src/api/fixtures/analyze/progress.json"),
    );
    assert_round_trip::<AnalyzeSnapshotV1>(
        "analyze/snapshot.json",
        include_str!("../../src/api/fixtures/analyze/snapshot.json"),
    );
    assert_round_trip::<CommandError>(
        "analyze/error.json",
        include_str!("../../src/api/fixtures/analyze/error.json"),
    );
    assert_round_trip::<AnalyzeTrashPreviewV1>(
        "analyze/trash-preview.json",
        include_str!("../../src/api/fixtures/analyze/trash-preview.json"),
    );
    assert_round_trip::<AnalyzeTrashReportV1>(
        "analyze/trash-report.json",
        include_str!("../../src/api/fixtures/analyze/trash-report.json"),
    );
}

#[test]
fn software_fixtures_match_the_software_command_wire_types() {
    assert_round_trip::<SoftwareInventoryV1>(
        "software/inventory.json",
        include_str!("../../src/api/fixtures/software/inventory.json"),
    );
    assert_round_trip::<SoftwareInventoryV1>(
        "software/inventory-partial.json",
        include_str!("../../src/api/fixtures/software/inventory-partial.json"),
    );
    assert_round_trip::<SoftwareInventoryV1>(
        "software/all-msi-manual.json",
        include_str!("../../src/api/fixtures/software/all-msi-manual.json"),
    );
    assert_round_trip::<DesktopSoftwarePreviewResult>(
        "software/preview.json",
        include_str!("../../src/api/fixtures/software/preview.json"),
    );
    assert_round_trip::<DesktopSoftwareUninstallResult>(
        "software/execution-five-terminal.json",
        include_str!("../../src/api/fixtures/software/execution-five-terminal.json"),
    );
    assert_round_trip::<DesktopSoftwareAuditResult>(
        "software/audit-restart.json",
        include_str!("../../src/api/fixtures/software/audit-restart.json"),
    );
}

#[test]
fn software_support_fixtures_match_the_support_command_wire_types() {
    assert_round_trip::<DesktopSoftwareUpdatesResult>(
        "software/updates-available.json",
        include_str!("../../src/api/fixtures/software/updates-available.json"),
    );
    assert_round_trip::<DesktopSoftwareUpdatesResult>(
        "software/updates-unavailable.json",
        include_str!("../../src/api/fixtures/software/updates-unavailable.json"),
    );
    assert_round_trip::<SoftwareStartupListV1>(
        "software/startup-list.json",
        include_str!("../../src/api/fixtures/software/startup-list.json"),
    );
    assert_round_trip::<SoftwareStartupToggleReportV1>(
        "software/startup-toggle.json",
        include_str!("../../src/api/fixtures/software/startup-toggle.json"),
    );
    assert_round_trip::<DesktopSoftwareLeftoversPreviewResult>(
        "software/leftovers-discovered.json",
        include_str!("../../src/api/fixtures/software/leftovers-discovered.json"),
    );
    assert_round_trip::<DesktopSoftwareLeftoversPreviewResult>(
        "software/leftovers-planned.json",
        include_str!("../../src/api/fixtures/software/leftovers-planned.json"),
    );
    assert_round_trip::<DesktopSoftwareLeftoversResult>(
        "software/leftovers-report.json",
        include_str!("../../src/api/fixtures/software/leftovers-report.json"),
    );
}

#[test]
fn optimize_fixtures_match_the_optimize_command_wire_types() {
    assert_round_trip::<DesktopOptimizePreviewResult>(
        "optimize/preview-dns.json",
        include_str!("../../src/api/fixtures/optimize/preview-dns.json"),
    );
    assert_round_trip::<DesktopOptimizePreviewResult>(
        "optimize/preview-settings-search.json",
        include_str!("../../src/api/fixtures/optimize/preview-settings-search.json"),
    );
    assert_round_trip::<DesktopOptimizePreviewResult>(
        "optimize/preview-settings-storage.json",
        include_str!("../../src/api/fixtures/optimize/preview-settings-storage.json"),
    );
    assert_round_trip::<DesktopOptimizePreviewResult>(
        "optimize/preview-settings-energy.json",
        include_str!("../../src/api/fixtures/optimize/preview-settings-energy.json"),
    );
    assert_round_trip::<DesktopOptimizeRunResult>(
        "optimize/execution-dns-succeeded.json",
        include_str!("../../src/api/fixtures/optimize/execution-dns-succeeded.json"),
    );
    assert_round_trip::<DesktopOptimizeRunResult>(
        "optimize/execution-settings-launched.json",
        include_str!("../../src/api/fixtures/optimize/execution-settings-launched.json"),
    );
    assert_round_trip::<DesktopOptimizeRunResult>(
        "optimize/execution-five-terminal.json",
        include_str!("../../src/api/fixtures/optimize/execution-five-terminal.json"),
    );
    assert_round_trip::<DesktopOptimizeAuditResult>(
        "optimize/audit.json",
        include_str!("../../src/api/fixtures/optimize/audit.json"),
    );
    for (label, raw) in [
        (
            "optimize/refusal-stale.json",
            include_str!("../../src/api/fixtures/optimize/refusal-stale.json"),
        ),
        (
            "optimize/refusal-guidance.json",
            include_str!("../../src/api/fixtures/optimize/refusal-guidance.json"),
        ),
        (
            "optimize/refusal-build-unsupported.json",
            include_str!("../../src/api/fixtures/optimize/refusal-build-unsupported.json"),
        ),
        (
            "optimize/refusal-os-build-unavailable.json",
            include_str!("../../src/api/fixtures/optimize/refusal-os-build-unavailable.json"),
        ),
    ] {
        assert_round_trip::<CommandError>(label, raw);
    }
}

#[test]
fn optimize_catalogue_fixture_matches_the_shipped_closed_catalogue() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../src/api/fixtures/optimize/catalogue.json"
    ))
    .expect("optimize/catalogue.json is valid JSON");
    let shipped = serde_json::to_value(catalogue_entries()).expect("catalogue entries serialize");

    assert_eq!(fixture["type"], serde_json::json!("completed"));
    assert_eq!(
        fixture["catalogue_version"],
        serde_json::json!(OPTIMIZE_CATALOGUE_VERSION)
    );
    assert_eq!(
        fixture["entries"], shipped,
        "optimize/catalogue.json must list exactly the shipped closed catalogue"
    );
}

#[test]
fn status_fixtures_match_the_status_command_wire_types() {
    assert_round_trip::<StatusSnapshotV1>(
        "status/snapshot.json",
        include_str!("../../src/api/fixtures/status/snapshot.json"),
    );
    assert_round_trip::<StatusSnapshotV1>(
        "status/live-snapshot.json",
        include_str!("../../src/api/fixtures/status/live-snapshot.json"),
    );
    assert_round_trip::<DesktopStatusSnapshotResult>(
        "status/snapshot-completed.json",
        include_str!("../../src/api/fixtures/status/snapshot-completed.json"),
    );
    assert_round_trip::<DesktopStatusLiveResult>(
        "status/live-completed.json",
        include_str!("../../src/api/fixtures/status/live-completed.json"),
    );
    for (label, raw) in [
        (
            "status/event-started.json",
            include_str!("../../src/api/fixtures/status/event-started.json"),
        ),
        (
            "status/event-snapshot.json",
            include_str!("../../src/api/fixtures/status/event-snapshot.json"),
        ),
        (
            "status/event-skipped.json",
            include_str!("../../src/api/fixtures/status/event-skipped.json"),
        ),
        (
            "status/event-terminal.json",
            include_str!("../../src/api/fixtures/status/event-terminal.json"),
        ),
    ] {
        assert_round_trip::<StatusEventV1>(label, raw);
    }
}

#[test]
fn hud_fixtures_match_the_hud_event_wire_type() {
    assert_round_trip::<HudStatusEvent>(
        "status/hud-sampling.json",
        include_str!("../../src/api/fixtures/status/hud-sampling.json"),
    );
    assert_round_trip::<HudStatusEvent>(
        "status/hud-snapshot.json",
        include_str!("../../src/api/fixtures/status/hud-snapshot.json"),
    );
}

#[test]
fn history_clean_totals_fixture_matches_the_core_wire_type() {
    assert_round_trip::<CleanMovedTotalsV1>(
        "history/clean-moved-totals.json",
        include_str!("../../src/api/fixtures/history/clean-moved-totals.json"),
    );
}

/// Names one `CommandError` variant.
///
/// The match is exhaustive on purpose: a new variant stops this test from
/// compiling until the shared error fixture carries its payload too.
fn error_code(error: &CommandError) -> &'static str {
    match error {
        CommandError::ScanAlreadyRunning => "scan_already_running",
        CommandError::ScanFailed { .. } => "scan_failed",
        CommandError::AnalyzeAlreadyRunning => "analyze_already_running",
        CommandError::AnalyzeFailed { .. } => "analyze_failed",
        CommandError::AnalyzeStaleOperation { .. } => "analyze_stale_operation",
        CommandError::SoftwareAlreadyRunning => "software_already_running",
        CommandError::SoftwareFailed { .. } => "software_failed",
        CommandError::SoftwareStaleAuthority { .. } => "software_stale_authority",
        CommandError::SoftwareAuditUnavailable { .. } => "software_audit_unavailable",
        CommandError::OptimizeAlreadyRunning => "optimize_already_running",
        CommandError::OptimizeFailed { .. } => "optimize_failed",
        CommandError::OptimizeStaleAuthority { .. } => "optimize_stale_authority",
        CommandError::OptimizeUnavailable { .. } => "optimize_unavailable",
        CommandError::OptimizeAuditUnavailable { .. } => "optimize_audit_unavailable",
        CommandError::StatusAlreadyRunning => "status_already_running",
        CommandError::StatusFailed { .. } => "status_failed",
        CommandError::InvalidPlan { .. } => "invalid_plan",
        CommandError::StaleConfirmation { .. } => "stale_confirmation",
        CommandError::UnknownTarget { .. } => "unknown_target",
        CommandError::InspectOnlyTarget { .. } => "inspect_only_target",
        CommandError::Io { .. } => "io",
        CommandError::ProtectionStoreUnavailable { .. } => "protection_store_unavailable",
        CommandError::ProtectionAuditUnknown { .. } => "protection_audit_unknown",
        CommandError::ProtectionPathMissing { .. } => "protection_path_missing",
        CommandError::ProtectionConfirmationRequired { .. } => "protection_confirmation_required",
        CommandError::HistoryStoreUnavailable { .. } => "history_store_unavailable",
        CommandError::HistoryNotFound { .. } => "history_not_found",
        CommandError::RuleNotFound { .. } => "rule_not_found",
    }
}

/// Count of `CommandError` variants the shared fixture must carry.
const COMMAND_ERROR_VARIANT_COUNT: usize = 28;

#[test]
fn shared_error_fixture_covers_every_command_error_variant() {
    let raw = include_str!("../../src/api/fixtures/errors/command-errors.json");
    let fixture: Vec<serde_json::Value> =
        serde_json::from_str(raw).expect("errors/command-errors.json is a JSON array");
    let decoded: Vec<CommandError> =
        serde_json::from_str(raw).expect("every payload matches the Rust command error");
    let encoded = serde_json::to_value(&decoded).expect("command errors serialize");

    assert_eq!(
        encoded,
        serde_json::Value::Array(fixture),
        "Rust command-error serialization differs from the desktop fixture"
    );

    let mut codes: Vec<&'static str> = decoded.iter().map(error_code).collect();
    codes.sort_unstable();
    let distinct = codes.len();
    codes.dedup();
    assert_eq!(codes.len(), distinct, "the fixture repeats a command error");
    assert_eq!(
        codes.len(),
        COMMAND_ERROR_VARIANT_COUNT,
        "the fixture must carry one payload for every command error variant"
    );
}

#[test]
fn status_unknown_event_fixture_stays_outside_the_closed_event_union() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../src/api/fixtures/status/unknown-event.json"
    ))
    .expect("status/unknown-event.json is valid JSON");

    serde_json::from_value::<StatusEventV1>(fixture)
        .expect_err("an unlisted Status event must not decode into the closed union");
}
