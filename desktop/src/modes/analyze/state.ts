import type { AnalyzeSnapshotV1, CommandError, DesktopAnalyzeProgress } from "../../api/types.gen";
import type { AnalyzeSort } from "./selectors";

export type AnalyzeStatus = "idle" | "loading" | "canceling" | "complete" | "partial" | "canceled" | "error";

export interface AnalyzeState {
  readonly status: AnalyzeStatus;
  readonly operationId: string | null;
  readonly lastSequence: number;
  readonly storedNodes: number;
  readonly accountedOwnedBytes: number;
  readonly snapshot: AnalyzeSnapshotV1 | null;
  readonly currentNodeId: number;
  readonly page: number;
  readonly query: string;
  readonly sort: AnalyzeSort;
  readonly focusedNodeId: number | null;
  readonly focusAnchors: Readonly<Record<string, number>>;
  readonly error: CommandError | null;
}

export const initialAnalyzeState: AnalyzeState = {
  status: "idle",
  operationId: null,
  lastSequence: 0,
  storedNodes: 0,
  accountedOwnedBytes: 0,
  snapshot: null,
  currentNodeId: 0,
  page: 0,
  query: "",
  sort: "size_desc",
  focusedNodeId: null,
  focusAnchors: {},
  error: null,
};

export type AnalyzeAction =
  | { readonly type: "analysis_requested"; readonly operationId: string }
  | { readonly type: "analysis_progressed"; readonly progress: DesktopAnalyzeProgress }
  | { readonly type: "analysis_cancel_requested"; readonly operationId: string }
  | { readonly type: "analysis_completed"; readonly operationId: string; readonly snapshot: AnalyzeSnapshotV1 }
  | { readonly type: "analysis_canceled"; readonly operationId: string; readonly snapshot: AnalyzeSnapshotV1 }
  | { readonly type: "analysis_failed"; readonly operationId: string; readonly error: CommandError }
  | { readonly type: "query_changed"; readonly query: string }
  | { readonly type: "sort_changed"; readonly sort: AnalyzeSort }
  | { readonly type: "page_changed"; readonly page: number }
  | { readonly type: "node_focused"; readonly nodeId: number; readonly page: number }
  | { readonly type: "directory_opened"; readonly nodeId: number; readonly page: number }
  | { readonly type: "directory_up"; readonly parentId: number; readonly childId: number; readonly page: number }
  | { readonly type: "error_dismissed" }
  | { readonly type: "released" };

function terminalStatus(snapshot: AnalyzeSnapshotV1): AnalyzeStatus {
  if (snapshot.completeness === "partial_budget") return "partial";
  if (snapshot.completeness === "canceled") return "canceled";
  return "complete";
}

export function analyzeReducer(state: AnalyzeState, action: AnalyzeAction): AnalyzeState {
  switch (action.type) {
    case "analysis_requested":
      return {
        ...initialAnalyzeState,
        status: "loading",
        operationId: action.operationId,
        query: state.query,
        sort: state.sort,
      };
    case "analysis_progressed":
      if (state.operationId !== action.progress.operation_id
        || !["loading", "canceling"].includes(state.status)
        || action.progress.sequence <= state.lastSequence) return state;
      return {
        ...state,
        lastSequence: action.progress.sequence,
        storedNodes: action.progress.stored_nodes,
        accountedOwnedBytes: action.progress.accounted_owned_bytes,
      };
    case "analysis_cancel_requested":
      return state.operationId === action.operationId && state.status === "loading"
        ? { ...state, status: "canceling" }
        : state;
    case "analysis_completed":
    case "analysis_canceled":
      if (state.operationId !== action.operationId) return state;
      return {
        ...state,
        status: terminalStatus(action.snapshot),
        operationId: null,
        snapshot: action.snapshot,
        storedNodes: action.snapshot.nodes.length,
        accountedOwnedBytes: action.snapshot.accounted_owned_bytes,
        currentNodeId: 0,
        page: 0,
        focusedNodeId: null,
        focusAnchors: {},
        error: null,
      };
    case "analysis_failed":
      return state.operationId === action.operationId
        ? { ...state, status: "error", operationId: null, error: action.error }
        : state;
    case "query_changed":
      return { ...state, query: action.query, page: 0, focusedNodeId: null };
    case "sort_changed":
      return { ...state, sort: action.sort, page: 0, focusedNodeId: null };
    case "page_changed":
      return { ...state, page: Math.max(0, Math.trunc(action.page)) };
    case "node_focused":
      return {
        ...state,
        focusedNodeId: action.nodeId,
        page: action.page,
        focusAnchors: { ...state.focusAnchors, [String(state.currentNodeId)]: action.nodeId },
      };
    case "directory_opened":
      return {
        ...state,
        currentNodeId: action.nodeId,
        page: Math.max(0, Math.trunc(action.page)),
        focusedNodeId: state.focusAnchors[String(action.nodeId)] ?? null,
      };
    case "directory_up":
      return {
        ...state,
        currentNodeId: action.parentId,
        page: Math.max(0, Math.trunc(action.page)),
        focusedNodeId: action.childId,
        focusAnchors: { ...state.focusAnchors, [String(action.parentId)]: action.childId },
      };
    case "error_dismissed":
      return { ...state, error: null, status: state.snapshot ? terminalStatus(state.snapshot) : "idle" };
    case "released":
      return { ...initialAnalyzeState, query: state.query, sort: state.sort };
  }
}
