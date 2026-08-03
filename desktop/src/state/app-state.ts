import type { CommandError, DryRunOutcome, ExecutionReport, ScanProgress, ScanReport, UntrustedTarget } from "../api/types.gen";
import { reportMatchesSelection } from "../api/contract";

export type Phase = "idle" | "scanning" | "reviewed" | "dry_run" | "confirming" | "executing" | "reported";
export type PendingOperation = "dry_run" | null;

export interface AppState {
  phase: Phase;
  pending: PendingOperation;
  cancelRequested: boolean;
  progress: ScanProgress | null;
  scan: ScanReport | null;
  selectedIds: Set<string>;
  dryRun: DryRunOutcome | null;
  execution: ExecutionReport | null;
  error: CommandError | null;
}

export type AppAction =
  | { type: "scan_requested" }
  | { type: "scan_progressed"; progress: ScanProgress }
  | { type: "scan_cancel_requested" }
  | { type: "scan_succeeded"; report: ScanReport }
  | { type: "command_failed"; error: CommandError }
  | { type: "selection_changed"; targetId: string; selected: boolean }
  | { type: "select_all_changed"; selected: boolean }
  | { type: "dry_run_requested" }
  | { type: "dry_run_succeeded"; outcome: DryRunOutcome }
  | { type: "confirmation_opened" }
  | { type: "confirmation_closed" }
  | { type: "execute_requested" }
  | { type: "execute_succeeded"; report: ExecutionReport }
  | { type: "review_requested" }
  | { type: "error_dismissed" };

export const initialState: AppState = {
  phase: "idle", pending: null, cancelRequested: false, progress: null, scan: null,
  selectedIds: new Set(), dryRun: null, execution: null, error: null,
};

export function isExecutable(target: UntrustedTarget): boolean {
  return target.intent.type !== "inspect_only";
}

function executableIds(state: AppState): Set<string> {
  return new Set((state.scan?.plan.targets ?? []).filter(isExecutable).map((target) => target.id));
}

function invalidatePreview(state: AppState, selectedIds: Set<string>): AppState {
  return { ...state, phase: state.scan ? "reviewed" : "idle", pending: null, selectedIds, dryRun: null, execution: null, error: null };
}

export function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "scan_requested":
      return { ...initialState, phase: "scanning" };
    case "scan_progressed":
      return state.phase === "scanning" ? { ...state, progress: action.progress } : state;
    case "scan_cancel_requested":
      return state.phase === "scanning" ? { ...state, cancelRequested: true } : state;
    case "scan_succeeded":
      return {
        ...initialState,
        phase: "reviewed",
        scan: action.report,
        selectedIds: new Set(action.report.plan.targets.filter((target) => target.selected_by_default && isExecutable(target)).map((target) => target.id)),
      };
    case "selection_changed": {
      const allowed = executableIds(state);
      if (!allowed.has(action.targetId)) return state;
      const selectedIds = new Set(state.selectedIds);
      if (action.selected) selectedIds.add(action.targetId); else selectedIds.delete(action.targetId);
      return invalidatePreview(state, selectedIds);
    }
    case "select_all_changed":
      return invalidatePreview(state, action.selected ? executableIds(state) : new Set());
    case "dry_run_requested":
      return state.scan && state.selectedIds.size > 0 && state.phase !== "scanning" && state.phase !== "executing"
        ? { ...state, pending: "dry_run", error: null, execution: null }
        : state;
    case "dry_run_succeeded":
      return state.pending === "dry_run" && state.selectedIds.size > 0 && reportMatchesSelection(action.outcome.report, [...state.selectedIds])
        ? { ...state, phase: "dry_run", pending: null, dryRun: action.outcome, execution: null, error: null }
        : state;
    case "confirmation_opened":
      return state.phase === "dry_run" && state.dryRun && state.selectedIds.size > 0 ? { ...state, phase: "confirming" } : state;
    case "confirmation_closed":
      return state.phase === "confirming" ? { ...state, phase: "dry_run" } : state;
    case "execute_requested":
      return state.phase === "confirming" && state.dryRun && state.selectedIds.size > 0 ? { ...state, phase: "executing", error: null } : state;
    case "execute_succeeded":
      return state.phase === "executing" && state.dryRun && !action.report.dry_run && action.report.confirmation_digest === state.dryRun.digest && reportMatchesSelection(action.report, [...state.selectedIds])
        ? { ...state, phase: "reported", execution: action.report, error: null }
        : state;
    case "review_requested":
      return invalidatePreview(state, state.selectedIds);
    case "command_failed":
      if (action.error.code === "stale_confirmation") {
        return { ...state, phase: "reviewed", pending: null, dryRun: null, execution: null, error: action.error };
      }
      return { ...state, phase: state.phase === "executing" ? "dry_run" : state.phase === "scanning" ? (state.scan ? "reviewed" : "idle") : state.phase, pending: null, cancelRequested: false, error: action.error };
    case "error_dismissed":
      return { ...state, error: null };
  }
}
