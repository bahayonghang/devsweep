import type {
  CommandError,
  DesktopScanProgress,
  DryRunOutcome,
  ExecutionReport,
  ScanPreviewSnapshot,
  ScanReport,
  UntrustedTarget,
} from "../api/types.gen";
import { reportMatchesSelection } from "../api/contract";

export type Phase = "idle" | "scanning" | "reviewed" | "dry_run" | "confirming" | "executing" | "reported";
export type PendingOperation = "dry_run" | null;

export interface ActiveScan {
  scanId: string;
  lastSequence: number;
  progress: DesktopScanProgress | null;
  preview: ScanPreviewSnapshot | null;
  cancelRequested: boolean;
}

export interface StoppedPreview {
  kind: "canceled" | "failed";
  progress: DesktopScanProgress | null;
  preview: ScanPreviewSnapshot | null;
}

export interface AppState {
  phase: Phase;
  pending: PendingOperation;
  activeScan: ActiveScan | null;
  stoppedPreview: StoppedPreview | null;
  scan: ScanReport | null;
  selectedIds: Set<string>;
  dryRun: DryRunOutcome | null;
  execution: ExecutionReport | null;
  error: CommandError | null;
}

export type AppAction =
  | { type: "scan_requested"; scanId: string }
  | { type: "scan_progressed"; progress: DesktopScanProgress }
  | { type: "scan_cancel_requested"; scanId: string }
  | { type: "scan_completed"; scanId: string; report: ScanReport }
  | { type: "scan_canceled"; scanId: string }
  | { type: "scan_failed"; scanId: string; error: CommandError }
  | { type: "stopped_preview_dismissed" }
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
  phase: "idle",
  pending: null,
  activeScan: null,
  stoppedPreview: null,
  scan: null,
  selectedIds: new Set(),
  dryRun: null,
  execution: null,
  error: null,
};

export function isExecutable(target: UntrustedTarget): boolean {
  return target.intent.type !== "inspect_only";
}

function executableIds(state: AppState): Set<string> {
  return new Set((state.scan?.plan.targets ?? []).filter(isExecutable).map((target) => target.id));
}

function defaultSelectedIds(report: ScanReport | null): Set<string> {
  return new Set(
    (report?.plan.targets ?? [])
      .filter((target) => target.selected_by_default && isExecutable(target))
      .map((target) => target.id),
  );
}

function invalidatePreview(state: AppState, selectedIds: Set<string>): AppState {
  return {
    ...state,
    phase: state.scan ? "reviewed" : "idle",
    pending: null,
    selectedIds,
    dryRun: null,
    execution: null,
    error: null,
  };
}

function stoppedPreview(state: AppState, kind: StoppedPreview["kind"]): StoppedPreview {
  const active = state.activeScan;
  return {
    kind,
    progress: active?.progress ?? null,
    preview: active?.preview ?? null,
  };
}

export function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "scan_requested":
      if (!action.scanId) return state;
      return {
        ...state,
        phase: "scanning",
        pending: null,
        activeScan: {
          scanId: action.scanId,
          lastSequence: 0,
          progress: null,
          preview: null,
          cancelRequested: false,
        },
        stoppedPreview: null,
        selectedIds: new Set(),
        dryRun: null,
        execution: null,
        error: null,
      };
    case "scan_progressed": {
      const active = state.activeScan;
      if (!active || action.progress.scan_id !== active.scanId || action.progress.sequence <= active.lastSequence) return state;
      return {
        ...state,
        activeScan: {
          ...active,
          lastSequence: action.progress.sequence,
          progress: action.progress,
          preview: action.progress.preview ?? active.preview,
        },
      };
    }
    case "scan_cancel_requested":
      return state.activeScan?.scanId === action.scanId
        ? { ...state, activeScan: { ...state.activeScan, cancelRequested: true } }
        : state;
    case "scan_completed":
      if (state.activeScan?.scanId !== action.scanId) return state;
      return {
        ...initialState,
        phase: "reviewed",
        scan: action.report,
        selectedIds: defaultSelectedIds(action.report),
      };
    case "scan_canceled":
      if (state.activeScan?.scanId !== action.scanId) return state;
      return {
        ...state,
        phase: state.scan ? "reviewed" : "idle",
        pending: null,
        activeScan: null,
        stoppedPreview: stoppedPreview(state, "canceled"),
        selectedIds: new Set(),
        dryRun: null,
        execution: null,
        error: null,
      };
    case "scan_failed":
      if (state.activeScan?.scanId !== action.scanId) return state;
      return {
        ...state,
        phase: state.scan ? "reviewed" : "idle",
        pending: null,
        activeScan: null,
        stoppedPreview: stoppedPreview(state, "failed"),
        selectedIds: new Set(),
        dryRun: null,
        execution: null,
        error: action.error,
      };
    case "stopped_preview_dismissed":
      if (!state.stoppedPreview) return state;
      return {
        ...state,
        phase: state.scan ? "reviewed" : "idle",
        stoppedPreview: null,
        selectedIds: defaultSelectedIds(state.scan),
        dryRun: null,
        execution: null,
        error: null,
      };
    case "selection_changed": {
      if (state.activeScan || state.stoppedPreview) return state;
      const allowed = executableIds(state);
      if (!allowed.has(action.targetId)) return state;
      const selectedIds = new Set(state.selectedIds);
      if (action.selected) selectedIds.add(action.targetId); else selectedIds.delete(action.targetId);
      return invalidatePreview(state, selectedIds);
    }
    case "select_all_changed":
      if (state.activeScan || state.stoppedPreview) return state;
      return invalidatePreview(state, action.selected ? executableIds(state) : new Set());
    case "dry_run_requested":
      return state.scan && !state.activeScan && !state.stoppedPreview && state.selectedIds.size > 0 && state.phase !== "executing"
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
      return {
        ...state,
        phase: state.phase === "executing" ? "dry_run" : state.phase,
        pending: null,
        error: action.error,
      };
    case "error_dismissed":
      return { ...state, error: null };
  }
}
