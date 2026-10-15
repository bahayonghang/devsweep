import { describe, expect, it } from "vitest";
import fixture from "../../api/fixtures/scan-report.json";
import previewFixture from "../../api/fixtures/dry-run-outcome.json";
import { decodeDryRunOutcome, decodeScanReport } from "../../api/contract";
import type { CleanAction } from "./reducer";
import { cleanSessionReducer, initialCleanSession, type CleanScope } from "./session-state";

const report = decodeScanReport(fixture);
function completed(scope: CleanScope) {
  let state = cleanSessionReducer(initialCleanSession, { type: "update", scope, revision: 0, change: { type: "scan_requested", scanId: "scan" } });
  state = cleanSessionReducer(state, { type: "update", scope, revision: 1, completedAt: "2026-10-14T12:00:00Z", change: { type: "scan_completed", scanId: "scan", report } });
  return state;
}
describe("Clean scope authority", () => {
  it("invalidates project roots without touching the global report or selection", () => {
    const state = { ...completed("global"), project: completed("project").project };
    const next = cleanSessionReducer(state, { type: "roots_changed" });
    expect(next.global).toBe(state.global);
    expect(next.project.state.scan).toBeNull();
    expect(next.project.state.selectedIds.size).toBe(0);
    expect(next.project.revision).toBe(state.project.revision + 1);
  });
  it("fences stale scan completions after roots change or route hide", () => {
    const state = completed("project");
    for (const type of ["roots_changed", "hidden"] as const) {
      const next = cleanSessionReducer(state, { type });
      expect(cleanSessionReducer(next, { type: "update", scope: "project", revision: state.project.revision, change: { type: "scan_completed", scanId: "scan", report } })).toBe(next);
    }
  });
  it("keeps observations but invalidates all plans after protection changes without claiming all targets are protected", () => {
    const next = cleanSessionReducer(completed("project"), { type: "protections_changed" });
    expect(next.project.stale).toBe(true);
    expect(next.project.state.scan).toBe(report);
    expect(next.project.state.selectedIds.size).toBe(0);
    expect(next.project.state.protectedIds.size).toBe(0);
    expect(next.project.state.dryRun).toBeNull();
  });
  it("settles hidden pending state and never retains confirmation authority", () => {
    const scanned = completed("project");
    const pending = { ...scanned, project: { ...scanned.project, state: { ...scanned.project.state, pending: "dry_run" as const, phase: "confirming" as const } } };
    const next = cleanSessionReducer(pending, { type: "hidden" });
    expect(next.project.state.pending).toBeNull();
    expect(next.project.state.activeScan).toBeNull();
    expect(next.project.state.phase).toBe("reviewed");
    expect(next.project.state.scan).toBe(report);
  });
  it("rejects a superseded preview even when a later request has the same selection", () => {
    const outcome = decodeDryRunOutcome(previewFixture);
    let state = completed("project");
    const update = (change: CleanAction, revision = state.project.revision) => {
      state = cleanSessionReducer(state, { type: "update", scope: "project", revision, change });
    };
    update({ type: "select_all_changed", selected: false });
    update({ type: "selection_changed", targetId: outcome.report.outcomes[0].target_id, selected: true });
    update({ type: "dry_run_requested" });
    const oldRevision = state.project.revision;
    state = cleanSessionReducer(state, { type: "hidden" });
    update({ type: "dry_run_requested" });
    const awaitingCurrent = state;
    update({ type: "dry_run_succeeded", outcome }, oldRevision);
    expect(state).toBe(awaitingCurrent);
    expect(state.project.state.dryRun).toBeNull();
    update({ type: "dry_run_succeeded", outcome });
    update({ type: "confirmation_opened" });
    expect(state.project.state.phase).toBe("confirming");
    state = cleanSessionReducer(state, { type: "hidden" });
    expect(state.project.state.dryRun).toBe(outcome);
    expect(state.project.state.phase).toBe("dry_run");
    for (const type of ["roots_changed", "protections_changed"] as const) {
      const invalidated = cleanSessionReducer(state, { type });
      expect(invalidated.project.state.dryRun).toBeNull();
      expect(invalidated.project.state.selectedIds.size).toBe(0);
    }
  });
});
