import type {
  CommandError,
  DesktopOptimizeAuditResult,
  DesktopOptimizePreviewResult,
  DesktopOptimizeRunResult,
  MaintenanceActionClass,
  MaintenanceCatalogueEntryV1,
  MaintenanceExecutionReportV1,
  MaintenancePlanV1,
  MaintenancePreviewV1,
} from "../../api/types.gen";

export type OptimizeStatus =
  | "idle"
  | "checking"
  | "ready"
  | "selected"
  | "previewing"
  | "preview_ready"
  | "confirming"
  | "running"
  | "launching"
  | "terminal"
  | "unknown"
  | "canceling";

export type OptimizeOperation = "list" | "preview" | "run" | "audit";

export interface OptimizeState {
  readonly status: OptimizeStatus;
  readonly resumeStatus: OptimizeStatus;
  readonly operationId: string | null;
  readonly operation: OptimizeOperation | null;
  readonly entries: readonly MaintenanceCatalogueEntryV1[] | null;
  readonly selectedId: string | null;
  readonly plan: MaintenancePlanV1 | null;
  readonly preview: MaintenancePreviewV1 | null;
  readonly report: MaintenanceExecutionReportV1 | null;
  readonly audit: DesktopOptimizeAuditResult | null;
  readonly expandedId: string | null;
  readonly error: CommandError | null;
}

export const initialOptimizeState: OptimizeState = {
  status: "idle",
  resumeStatus: "idle",
  operationId: null,
  operation: null,
  entries: null,
  selectedId: null,
  plan: null,
  preview: null,
  report: null,
  audit: null,
  expandedId: null,
  error: null,
};

export type OptimizeAction =
  | { readonly type: "operation_requested"; readonly operationId: string; readonly operation: OptimizeOperation }
  | { readonly type: "cancel_requested"; readonly operationId: string }
  | { readonly type: "list_completed"; readonly operationId: string; readonly entries: readonly MaintenanceCatalogueEntryV1[] }
  | { readonly type: "operation_canceled"; readonly operationId: string }
  | { readonly type: "selection_changed"; readonly catalogueId: string }
  | { readonly type: "expanded_changed"; readonly catalogueId: string | null }
  | { readonly type: "preview_completed"; readonly operationId: string; readonly result: DesktopOptimizePreviewResult }
  | { readonly type: "confirmation_opened" }
  | { readonly type: "confirmation_closed" }
  | { readonly type: "run_completed"; readonly operationId: string; readonly result: DesktopOptimizeRunResult }
  | { readonly type: "audit_completed"; readonly operationId: string; readonly result: DesktopOptimizeAuditResult }
  | { readonly type: "operation_failed"; readonly operationId: string; readonly error: CommandError }
  | { readonly type: "error_dismissed" }
  | { readonly type: "released" };

const CLOSED_IDS = [
  "dns.flush",
  "settings.storage_recommendations",
  "settings.search",
  "settings.energy_recommendations",
  "guidance.drive_optimize",
  "guidance.system_integrity",
  "guidance.filesystem_check",
  "guidance.network_reset",
] as const;

export function dispatchable(actionClass: MaintenanceActionClass): boolean {
  return actionClass === "execute" || actionClass === "settings_handoff";
}

export function selectedEntry(state: OptimizeState): MaintenanceCatalogueEntryV1 | null {
  if (!state.entries || !state.selectedId) return null;
  return state.entries.find((entry) => entry.id === state.selectedId) ?? null;
}

function catalogueMatchesClosedSet(entries: readonly MaintenanceCatalogueEntryV1[]): boolean {
  return entries.length === CLOSED_IDS.length
    && entries.every((entry, index) => entry.id === CLOSED_IDS[index]);
}

function statusForOperation(state: OptimizeState, operation: OptimizeOperation): OptimizeStatus {
  if (operation === "list" || operation === "audit") return "checking";
  if (operation === "preview") return "previewing";
  if (operation === "run") {
    return state.preview?.action_class === "settings_handoff" ? "launching" : "running";
  }
  return state.status;
}

function idleAfterCancel(state: OptimizeState): OptimizeStatus {
  if (state.preview) return "preview_ready";
  if (state.selectedId) return "selected";
  if (state.entries) return "ready";
  return "idle";
}

export function optimizeReducer(state: OptimizeState, action: OptimizeAction): OptimizeState {
  switch (action.type) {
    case "operation_requested":
      if (state.operationId !== null) return state;
      if (action.operation === "preview" && !selectedEntry(state)?.action_class) return state;
      if (action.operation === "preview" && !dispatchable(selectedEntry(state)!.action_class)) return state;
      if (action.operation === "run" && (state.status !== "confirming" || !state.plan || !state.preview)) return state;
      return {
        ...state,
        resumeStatus: state.status,
        status: statusForOperation(state, action.operation),
        operationId: action.operationId,
        operation: action.operation,
        error: null,
      };
    case "cancel_requested":
      return state.operationId === action.operationId ? { ...state, status: "canceling" } : state;
    case "list_completed":
      if (state.operationId !== action.operationId || state.operation !== "list"
        || !catalogueMatchesClosedSet(action.entries)) return state;
      return {
        ...state,
        status: "ready",
        resumeStatus: "ready",
        operationId: null,
        operation: null,
        entries: action.entries,
        selectedId: null,
        plan: null,
        preview: null,
        report: null,
        expandedId: null,
        error: null,
      };
    case "operation_canceled":
      if (state.operationId !== action.operationId) return state;
      return {
        ...state,
        status: state.operation === "audit" ? state.resumeStatus : idleAfterCancel(state),
        resumeStatus: state.operation === "audit" ? state.resumeStatus : idleAfterCancel(state),
        operationId: null,
        operation: null,
      };
    case "selection_changed": {
      if (!state.entries || !["ready", "selected", "preview_ready", "terminal", "unknown"].includes(state.status)
        || !state.entries.some((entry) => entry.id === action.catalogueId)) return state;
      return {
        ...state,
        status: "selected",
        resumeStatus: "selected",
        selectedId: action.catalogueId,
        plan: null,
        preview: null,
        report: null,
      };
    }
    case "expanded_changed":
      return { ...state, expandedId: action.catalogueId };
    case "preview_completed": {
      const selected = selectedEntry(state);
      if (state.operationId !== action.operationId || state.operation !== "preview"
        || action.result.operation_id !== action.operationId
        || !selected
        || selected.id !== action.result.plan.operation_id
        || selected.id !== action.result.preview.operation_id
        || selected.action_class !== action.result.preview.action_class
        || !dispatchable(action.result.preview.action_class)) return state;
      return {
        ...state,
        status: "preview_ready",
        resumeStatus: "preview_ready",
        operationId: null,
        operation: null,
        plan: action.result.plan,
        preview: action.result.preview,
        report: null,
        error: null,
      };
    }
    case "confirmation_opened":
      return state.status === "preview_ready" && state.plan && state.preview
        ? { ...state, status: "confirming" }
        : state;
    case "confirmation_closed":
      return state.status === "confirming" ? { ...state, status: "preview_ready" } : state;
    case "run_completed": {
      if (state.operationId !== action.operationId || state.operation !== "run"
        || action.result.operation_id !== action.operationId
        || !state.preview
        || action.result.report.outcomes.length !== 1
        || action.result.report.outcomes[0].catalogue_id !== state.preview.operation_id
        || action.result.report.outcomes[0].action_class !== state.preview.action_class) return state;
      const unknown = action.result.report.outcomes.some((item) => item.outcome === "unknown_after_dispatch");
      return {
        ...state,
        status: unknown ? "unknown" : "terminal",
        resumeStatus: unknown ? "unknown" : "terminal",
        operationId: null,
        operation: null,
        report: action.result.report,
        error: null,
      };
    }
    case "audit_completed":
      if (state.operationId !== action.operationId || state.operation !== "audit"
        || action.result.operation_id !== action.operationId) return state;
      return {
        ...state,
        status: state.resumeStatus,
        operationId: null,
        operation: null,
        audit: action.result,
        error: null,
      };
    case "operation_failed":
      if (state.operationId !== action.operationId) return state;
      return {
        ...state,
        status: state.operation === "audit" ? state.resumeStatus : "unknown",
        resumeStatus: state.operation === "audit" ? state.resumeStatus : "unknown",
        operationId: null,
        operation: null,
        error: action.error,
      };
    case "error_dismissed":
      return { ...state, error: null };
    case "released":
      return initialOptimizeState;
  }
}
