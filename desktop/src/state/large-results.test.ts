import { describe, expect, it } from "vitest";
import reportFixture from "../api/fixtures/scan-report.json";
import progressFixture from "../api/fixtures/scan-progress.json";
import { decodeDesktopScanProgress, decodeScanReport } from "../api/contract";
import { appReducer, initialState } from "./app-state";
import { allExecutableSelected, selectedTargets, selectedTotals } from "./selectors";

function largeReport() {
  const base = reportFixture.plan.targets[0];
  const targets = Array.from({ length: 10_000 }, (_, index) => ({
    ...base,
    id: `cargo.target:C:/fixture/project-${index}/target`,
    path: `C:/fixture/project-${index}/target`,
    scope: { type: "project", root: `C:/fixture/project-${index}` },
    estimated_bytes: 1024,
    selected_by_default: false,
    intent: { type: index === 9999 ? "inspect_only" : "trash_project_artifact", rule_id: base.rule_id },
  }));
  return decodeScanReport({
    version: 1, plan: { version: 2, targets },
    health: { completeness: "complete", diagnostics: [], totals: {
      verified_bytes: 10_000 * 1024, partial_lower_bound_bytes: 0, unknown_target_count: 0,
    } },
  });
}

describe("large completed reports and replacement previews", () => {
  it("selects the full 10k report independently of rendered rows while retaining exclusions", () => {
    const report = largeReport();
    const scanning = appReducer(initialState, { type: "scan_requested", scanId: "large" });
    let state = appReducer(scanning, { type: "scan_completed", scanId: "large", report });
    const protectedId = report.plan.targets[5000].id;
    const skippedId = report.plan.targets[9998].id;
    const inspectId = report.plan.targets[9999].id;
    state = appReducer(state, { type: "target_protected", targetId: protectedId });
    state = appReducer(state, { type: "target_skipped", targetId: skippedId });
    state = appReducer(state, { type: "select_all_changed", selected: true });
    expect(state.scan).toBe(report);
    expect(state.selectedIds.size).toBe(9997);
    expect(selectedTargets(state)).toHaveLength(9997);
    expect(selectedTotals(state)).toEqual({ verified_bytes: 9997 * 1024, partial_lower_bound_bytes: 0, unknown_target_count: 0 });
    expect(allExecutableSelected(state)).toBe(true);
    for (const targetId of [protectedId, skippedId, inspectId, "missing-target"]) {
      expect(appReducer(state, { type: "selection_changed", targetId, selected: true })).toBe(state);
      expect(state.selectedIds.has(targetId)).toBe(false);
    }
    state = appReducer(state, { type: "selection_changed", targetId: report.plan.targets[9997].id, selected: false });
    expect(state.selectedIds.size).toBe(9996);
    expect(allExecutableSelected(state)).toBe(false);
    expect(state.scan?.plan.targets).toHaveLength(10_000);
  });

  it("replaces repeated cumulative previews without accumulating targets or selection authority", () => {
    let state = appReducer(initialState, { type: "scan_requested", scanId: progressFixture.scan_id });
    let latest = decodeDesktopScanProgress(progressFixture);
    for (let sequence = 1; sequence <= 20; sequence += 1) {
      const base = progressFixture.preview.targets[0];
      latest = decodeDesktopScanProgress({
        ...progressFixture, sequence,
        preview: { targets: Array.from({ length: 1000 }, (_, index) => ({
          ...base, id: `node.node_modules:C:/fixture/run-${sequence}/project-${index}/node_modules`,
          path: `C:/fixture/run-${sequence}/project-${index}/node_modules`,
        })), totals: { ...progressFixture.preview.totals, target_count: 1000, verified_bytes: 4096 * 1000 } },
      });
      state = appReducer(state, { type: "scan_progressed", progress: latest });
      expect(state.activeScan?.preview).toBe(latest.preview);
      expect(state.activeScan?.preview?.targets).toHaveLength(1000);
      expect(state.selectedIds.size).toBe(0);
      expect(state.scan).toBeNull();
    }
    expect(appReducer(state, { type: "select_all_changed", selected: true })).toBe(state);
    expect(appReducer(state, { type: "selection_changed", targetId: latest.preview!.targets[0].id, selected: true })).toBe(state);
    state = appReducer(state, { type: "scan_canceled", scanId: progressFixture.scan_id });
    expect(state.activeScan).toBeNull();
    expect(state.stoppedPreview?.preview).toBe(latest.preview);
    expect(state.stoppedPreview?.progress).toBe(latest);
    expect(state.selectedIds.size).toBe(0);
    expect(appReducer(state, { type: "dry_run_requested" })).toBe(state);
  });
});
