import { describe, expect, it } from "vitest";
import fixture from "../api/fixtures/scan-report.json";
import dryRunFixture from "../api/fixtures/dry-run-outcome.json";
import { decodeDryRunOutcome, decodeScanReport } from "../api/contract";
import { appReducer, initialState } from "./app-state";

const report = decodeScanReport(fixture);
const dryRun = decodeDryRunOutcome(dryRunFixture);

describe("appReducer", () => {
  it("projects defaults while rejecting inspect-only targets", () => {
    let state = appReducer(initialState, { type: "scan_succeeded", report });
    expect([...state.selectedIds]).toEqual(["cargo.target:C:/work/app/target"]);
    state = appReducer(state, { type: "selection_changed", targetId: "cargo.home.inspect:C:/Users/dev/.cargo", selected: true });
    expect([...state.selectedIds]).toEqual(["cargo.target:C:/work/app/target"]);
  });

  it("select all excludes inspect-only targets", () => {
    const reviewed = appReducer(initialState, { type: "scan_succeeded", report });
    const selected = appReducer(reviewed, { type: "select_all_changed", selected: true });
    expect([...selected.selectedIds]).toEqual(["cargo.target:C:/work/app/target", "npm.cache.clean:global"]);
  });

  it("invalidates the digest on selection and rescan", () => {
    let state = appReducer(initialState, { type: "scan_succeeded", report });
    state = appReducer(state, { type: "dry_run_requested" });
    state = appReducer(state, { type: "dry_run_succeeded", outcome: dryRun });
    state = appReducer(state, { type: "selection_changed", targetId: "npm.cache.clean:global", selected: true });
    expect(state.phase).toBe("reviewed");
    expect(state.dryRun).toBeNull();
    state = appReducer({ ...state, dryRun }, { type: "scan_requested" });
    expect(state.dryRun).toBeNull();
  });

  it("clears stale confirmation and returns to review", () => {
    let state = appReducer(initialState, { type: "scan_succeeded", report });
    state = appReducer(state, { type: "dry_run_requested" });
    state = appReducer(state, { type: "dry_run_succeeded", outcome: dryRun });
    state = appReducer(state, { type: "confirmation_opened" });
    state = appReducer(state, { type: "execute_requested" });
    state = appReducer(state, { type: "command_failed", error: { code: "stale_confirmation", expected_digest: "old", actual_digest: "new" } });
    expect(state.phase).toBe("reviewed");
    expect(state.dryRun).toBeNull();
  });

  it("gates execution behind current dry run and confirmation", () => {
    const reviewed = appReducer(initialState, { type: "scan_succeeded", report });
    expect(appReducer(reviewed, { type: "execute_requested" })).toBe(reviewed);
    let state = appReducer(reviewed, { type: "dry_run_requested" });
    state = appReducer(state, { type: "dry_run_succeeded", outcome: dryRun });
    expect(appReducer(state, { type: "execute_requested" })).toBe(state);
    state = appReducer(state, { type: "confirmation_opened" });
    expect(appReducer(state, { type: "execute_requested" }).phase).toBe("executing");
  });

  it("rejects dry-run and execution responses for a different selection", () => {
    let state = appReducer(initialState, { type: "scan_succeeded", report });
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
