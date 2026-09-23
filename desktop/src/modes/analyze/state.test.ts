import { describe, expect, it } from "vitest";
import snapshotFixture from "../../api/fixtures/analyze/snapshot.json";
import progressFixture from "../../api/fixtures/analyze/progress.json";
import trashPreviewFixture from "../../api/fixtures/analyze/trash-preview.json";
import trashReportFixture from "../../api/fixtures/analyze/trash-report.json";
import { decodeAnalyzeSnapshot, decodeAnalyzeTrashPreview, decodeAnalyzeTrashReport, decodeDesktopAnalyzeProgress } from "../../api/contract";
import { analyzeReducer, initialAnalyzeState, type AnalyzeState } from "./state";

const snapshot = decodeAnalyzeSnapshot(snapshotFixture);
const progress = decodeDesktopAnalyzeProgress(progressFixture);
const trashPreview = decodeAnalyzeTrashPreview(trashPreviewFixture);
const trashReport = decodeAnalyzeTrashReport(trashReportFixture);

describe("Analyze reducer", () => {
  it("keeps the snapshot operation id and walks the Recycle Bin flow only in order", () => {
    let state = analyzeReducer(initialAnalyzeState, { type: "analysis_requested", operationId: "analyze-op-1" });
    state = analyzeReducer(state, { type: "analysis_completed", operationId: "analyze-op-1", snapshot });
    expect(state.operationId).toBeNull();
    expect(state.snapshotOperationId).toBe("analyze-op-1");

    expect(analyzeReducer(state, { type: "trash_preview_requested", operationId: "stale", nodeIds: [1] })).toBe(state);
    expect(analyzeReducer(state, { type: "trash_confirm_opened" })).toBe(state);
    state = analyzeReducer(state, { type: "trash_preview_requested", operationId: "analyze-op-1", nodeIds: [0, 1] });
    expect(state.trash.phase).toBe("previewing");
    expect(analyzeReducer(state, { type: "trash_preview_loaded", preview: { ...trashPreview, operation_id: "stale" } })).toBe(state);
    state = analyzeReducer(state, { type: "trash_preview_loaded", preview: trashPreview });
    expect(state.trash.phase).toBe("reviewed");
    expect(state.refusalHints).toEqual({ 0: "analysis_root" });
    expect(analyzeReducer(state, { type: "trash_execute_requested" })).toBe(state);
    state = analyzeReducer(state, { type: "trash_confirm_opened" });
    state = analyzeReducer(state, { type: "trash_execute_requested" });
    expect(state.trash.phase).toBe("executing");
    expect(analyzeReducer(state, { type: "trash_dismissed" })).toBe(state);
    expect(analyzeReducer(state, { type: "trash_executed", report: { ...trashReport, operation_id: "stale" } })).toBe(state);
    state = analyzeReducer(state, { type: "trash_executed", report: trashReport });
    expect(state.trash.phase).toBe("reported");
    expect(state.movedIds).toEqual([1]);
    state = analyzeReducer(state, { type: "trash_dismissed" });
    expect(state.trash.phase).toBe("idle");
    expect(state.movedIds).toEqual([1]);

    state = analyzeReducer(state, { type: "analysis_requested", operationId: "next" });
    expect(state.movedIds).toEqual([]);
    expect(state.refusalHints).toEqual({});
    expect(state.snapshotOperationId).toBeNull();
  });

  it("returns a reviewed preview on cancel and reports a stale execution", () => {
    let state: AnalyzeState = { ...initialAnalyzeState, snapshot, status: "complete", snapshotOperationId: "analyze-op-1" };
    state = analyzeReducer(state, { type: "trash_preview_requested", operationId: "analyze-op-1", nodeIds: [1] });
    state = analyzeReducer(state, { type: "trash_preview_loaded", preview: { ...trashPreview, refused: [] } });
    state = analyzeReducer(state, { type: "trash_confirm_opened" });
    state = analyzeReducer(state, { type: "trash_confirm_closed" });
    expect(state.trash.phase).toBe("reviewed");
    state = analyzeReducer(state, { type: "trash_confirm_opened" });
    state = analyzeReducer(state, { type: "trash_execute_requested" });
    const error = { code: "stale_confirmation" as const, expected_digest: "a", actual_digest: "b" };
    state = analyzeReducer(state, { type: "trash_execute_failed", operationId: "analyze-op-1", error });
    expect(state.trash.phase).toBe("reported");
    expect(state.trash.error).toEqual(error);
    expect(state.movedIds).toEqual([]);
  });


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
