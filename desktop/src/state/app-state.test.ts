import { describe, expect, it } from "vitest";
import fixture from "../api/fixtures/scan-report.json";
import progressFixture from "../api/fixtures/scan-progress.json";
import dryRunFixture from "../api/fixtures/dry-run-outcome.json";
import { decodeDesktopScanProgress, decodeDryRunOutcome, decodeScanReport } from "../api/contract";
import { appReducer, initialState } from "./app-state";

const report = decodeScanReport(fixture);
const progress = decodeDesktopScanProgress(progressFixture);
const dryRun = decodeDryRunOutcome(dryRunFixture);

function reviewedState() {
  let state = appReducer(initialState, { type: "scan_requested", scanId: "completed" });
  state = appReducer(state, { type: "scan_completed", scanId: "completed", report });
  return state;
}

describe("appReducer", () => {
  it("projects defaults only from a matching final report and rejects inspect-only targets", () => {
    let state = reviewedState();
    expect([...state.selectedIds]).toEqual(["cargo.target:C:/work/app/target"]);
    state = appReducer(state, { type: "selection_changed", targetId: "cargo.home.inspect:C:/Users/dev/.cargo", selected: true });
    expect([...state.selectedIds]).toEqual(["cargo.target:C:/work/app/target"]);
  });

  it("select all excludes inspect-only targets", () => {
    const selected = appReducer(reviewedState(), { type: "select_all_changed", selected: true });
    expect([...selected.selectedIds]).toEqual(["cargo.target:C:/work/app/target", "npm.cache.clean:global"]);
  });

  it("accepts only higher sequences for the active scan id", () => {
    let state = appReducer(initialState, { type: "scan_requested", scanId: progress.scan_id });
    state = appReducer(state, { type: "scan_progressed", progress });
    expect(state.activeScan?.preview?.targets).toHaveLength(1);
    expect(state.activeScan?.lastSequence).toBe(1);

    const duplicate = appReducer(state, { type: "scan_progressed", progress: { ...progress, message: "duplicate" } });
    expect(duplicate).toBe(state);
    const staleRun = appReducer(state, { type: "scan_progressed", progress: { ...progress, scan_id: "stale", sequence: 2 } });
    expect(staleRun).toBe(state);

    state = appReducer(state, { type: "scan_progressed", progress: { ...progress, sequence: 2, message: "phase boundary", preview: null } });
    expect(state.activeScan?.progress?.message).toBe("phase boundary");
    expect(state.activeScan?.preview?.targets).toHaveLength(1);
  });

  it("keeps previews read-only through cancel request and explicit cancellation", () => {
    let state = appReducer(initialState, { type: "scan_requested", scanId: progress.scan_id });
    state = appReducer(state, { type: "scan_progressed", progress });
    state = appReducer(state, { type: "scan_cancel_requested", scanId: progress.scan_id });
    expect(state.activeScan?.cancelRequested).toBe(true);
    expect(state.activeScan?.preview?.targets).toHaveLength(1);
    expect(appReducer(state, { type: "selection_changed", targetId: progress.preview!.targets[0].id, selected: true })).toBe(state);

    state = appReducer(state, { type: "scan_canceled", scanId: progress.scan_id });
    expect(state.activeScan).toBeNull();
    expect(state.stoppedPreview?.kind).toBe("canceled");
    expect(state.stoppedPreview?.preview?.targets).toHaveLength(1);
    expect(state.selectedIds.size).toBe(0);
    expect(appReducer(state, { type: "dry_run_requested" })).toBe(state);
  });

  it("retains the previous completed report after a failed rescan", () => {
    let state = reviewedState();
    state = appReducer(state, { type: "scan_requested", scanId: progress.scan_id });
    expect(state.scan).toBe(report);
    expect(state.selectedIds.size).toBe(0);
    state = appReducer(state, { type: "scan_progressed", progress });
    state = appReducer(state, { type: "scan_failed", scanId: progress.scan_id, error: { code: "scan_failed", message: "controlled" } });
    expect(state.scan).toBe(report);
    expect(state.stoppedPreview?.kind).toBe("failed");
    expect(state.stoppedPreview?.preview?.targets).toHaveLength(1);
    expect(state.selectedIds.size).toBe(0);

    state = appReducer(state, { type: "stopped_preview_dismissed" });
    expect(state.stoppedPreview).toBeNull();
    expect(state.phase).toBe("reviewed");
    expect([...state.selectedIds]).toEqual(["cargo.target:C:/work/app/target"]);
  });

  it("atomically replaces preview state on matching completion and ignores stale terminals", () => {
    let state = appReducer(initialState, { type: "scan_requested", scanId: progress.scan_id });
    state = appReducer(state, { type: "scan_progressed", progress });
    expect(appReducer(state, { type: "scan_completed", scanId: "stale", report })).toBe(state);
    state = appReducer(state, { type: "scan_completed", scanId: progress.scan_id, report });
    expect(state.activeScan).toBeNull();
    expect(state.stoppedPreview).toBeNull();
    expect(state.scan).toBe(report);
    expect([...state.selectedIds]).toEqual(["cargo.target:C:/work/app/target"]);
  });

  it("invalidates the digest on selection and rescan", () => {
    let state = reviewedState();
    state = appReducer(state, { type: "dry_run_requested" });
    state = appReducer(state, { type: "dry_run_succeeded", outcome: dryRun });
    state = appReducer(state, { type: "selection_changed", targetId: "npm.cache.clean:global", selected: true });
    expect(state.phase).toBe("reviewed");
    expect(state.dryRun).toBeNull();
    state = appReducer({ ...state, dryRun }, { type: "scan_requested", scanId: "rescan" });
    expect(state.dryRun).toBeNull();
  });

  it("clears stale confirmation and returns to review", () => {
    let state = reviewedState();
    state = appReducer(state, { type: "dry_run_requested" });
    state = appReducer(state, { type: "dry_run_succeeded", outcome: dryRun });
    state = appReducer(state, { type: "confirmation_opened" });
    state = appReducer(state, { type: "execute_requested" });
    state = appReducer(state, { type: "command_failed", error: { code: "stale_confirmation", expected_digest: "old", actual_digest: "new" } });
    expect(state.phase).toBe("reviewed");
    expect(state.dryRun).toBeNull();
  });

  it("gates execution behind current dry run and confirmation", () => {
    const reviewed = reviewedState();
    expect(appReducer(reviewed, { type: "execute_requested" })).toBe(reviewed);
    let state = appReducer(reviewed, { type: "dry_run_requested" });
    state = appReducer(state, { type: "dry_run_succeeded", outcome: dryRun });
    expect(appReducer(state, { type: "execute_requested" })).toBe(state);
    state = appReducer(state, { type: "confirmation_opened" });
    expect(appReducer(state, { type: "execute_requested" }).phase).toBe("executing");
  });

  it("rejects dry-run and execution responses for a different selection", () => {
    let state = reviewedState();
    state = appReducer(state, { type: "dry_run_requested" });
    const mismatched = { ...dryRun, report: { ...dryRun.report, outcomes: [{ ...dryRun.report.outcomes[0], target_id: "npm.cache.clean:global" }] } };
    expect(appReducer(state, { type: "dry_run_succeeded", outcome: mismatched })).toBe(state);

    state = appReducer(state, { type: "dry_run_succeeded", outcome: dryRun });
    state = appReducer(state, { type: "confirmation_opened" });
    state = appReducer(state, { type: "execute_requested" });
    const wrongMode = { ...dryRun.report, dry_run: true };
    expect(appReducer(state, { type: "execute_succeeded", report: wrongMode })).toBe(state);
  });
});
