import { describe, expect, it } from "vitest";
import snapshotFixture from "../../api/fixtures/analyze/snapshot.json";
import progressFixture from "../../api/fixtures/analyze/progress.json";
import { decodeAnalyzeSnapshot, decodeDesktopAnalyzeProgress } from "../../api/contract";
import { analyzeReducer, initialAnalyzeState, type AnalyzeState } from "./state";

const snapshot = decodeAnalyzeSnapshot(snapshotFixture);
const progress = decodeDesktopAnalyzeProgress(progressFixture);

describe("Analyze reducer", () => {
  it("rejects stale operation ids and non-monotonic sequences", () => {
    let state = analyzeReducer(initialAnalyzeState, { type: "analysis_requested", operationId: "current" });
    state = analyzeReducer(state, { type: "analysis_progressed", progress: { ...progress, operation_id: "stale" } });
    expect(state.lastSequence).toBe(0);
    state = analyzeReducer(state, { type: "analysis_progressed", progress: { ...progress, operation_id: "current", sequence: 2 } });
    state = analyzeReducer(state, { type: "analysis_progressed", progress: { ...progress, operation_id: "current", sequence: 1 } });
    expect(state.lastSequence).toBe(2);
    expect(analyzeReducer(state, { type: "analysis_completed", operationId: "stale", snapshot })).toBe(state);
  });

  it("preserves focus anchors across directory drill-down and up", () => {
    let state: AnalyzeState = { ...initialAnalyzeState, snapshot, status: "complete" };
    state = analyzeReducer(state, { type: "node_focused", nodeId: 1, page: 0 });
    state = analyzeReducer(state, { type: "directory_opened", nodeId: 1, page: 0 });
    state = analyzeReducer(state, { type: "directory_up", parentId: 0, childId: 1, page: 2 });
    expect(state.currentNodeId).toBe(0);
    expect(state.page).toBe(2);
    expect(state.focusedNodeId).toBe(1);
    expect(state.focusAnchors["0"]).toBe(1);
  });
});
