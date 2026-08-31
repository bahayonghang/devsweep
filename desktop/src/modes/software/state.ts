import type {
  CommandError,
  DesktopSoftwareAuditResult,
  DesktopSoftwarePreviewResult,
  DesktopSoftwareUninstallResult,
  SoftwareInventoryV1,
  SoftwarePreviewV1,
  SoftwareSelectionPlanV1,
} from "../../api/types.gen";

export type SoftwareStatus =
  | "idle"
  | "loading"
  | "ready"
  | "selecting"
  | "previewing"
  | "preview_ready"
  | "confirming"
  | "uninstalling"
  | "terminal"
  | "unknown"
  | "canceling";

export type SoftwareOperation = "inventory" | "preview" | "uninstall" | "audit";
export type SoftwareSort = "name" | "size" | "eligibility" | "source";

export interface SoftwareState {
  readonly status: SoftwareStatus;
  readonly resumeStatus: SoftwareStatus;
  readonly operationId: string | null;
  readonly operation: SoftwareOperation | null;
  readonly inventory: SoftwareInventoryV1 | null;
  readonly selectedIds: ReadonlySet<string>;
  readonly plan: SoftwareSelectionPlanV1 | null;
  readonly preview: SoftwarePreviewV1 | null;
  readonly report: DesktopSoftwareUninstallResult["report"] | null;
  readonly audit: DesktopSoftwareAuditResult | null;
  readonly query: string;
  readonly sort: SoftwareSort;
  readonly expandedId: string | null;
  readonly error: CommandError | null;
}

export const initialSoftwareState: SoftwareState = {
  status: "idle",
  resumeStatus: "idle",
  operationId: null,
  operation: null,
  inventory: null,
  selectedIds: new Set(),
  plan: null,
  preview: null,
  report: null,
  audit: null,
  query: "",
  sort: "name",
  expandedId: null,
  error: null,
};

export type SoftwareAction =
  | { readonly type: "operation_requested"; readonly operationId: string; readonly operation: SoftwareOperation }
  | { readonly type: "cancel_requested"; readonly operationId: string }
  | { readonly type: "inventory_completed"; readonly operationId: string; readonly inventory: SoftwareInventoryV1 }
  | { readonly type: "operation_canceled"; readonly operationId: string }
  | { readonly type: "selection_changed"; readonly softwareId: string; readonly selected: boolean }
  | { readonly type: "select_all"; readonly selected: boolean }
  | { readonly type: "query_changed"; readonly query: string }
  | { readonly type: "sort_changed"; readonly sort: SoftwareSort }
  | { readonly type: "expanded_changed"; readonly softwareId: string | null }
  | { readonly type: "preview_completed"; readonly operationId: string; readonly result: DesktopSoftwarePreviewResult }
  | { readonly type: "confirmation_opened" }
  | { readonly type: "confirmation_closed" }
  | { readonly type: "uninstall_completed"; readonly operationId: string; readonly result: DesktopSoftwareUninstallResult }
  | { readonly type: "audit_completed"; readonly operationId: string; readonly result: DesktopSoftwareAuditResult }
  | { readonly type: "operation_failed"; readonly operationId: string; readonly error: CommandError }
  | { readonly type: "error_dismissed" }
  | { readonly type: "released" };

function statusForOperation(state: SoftwareState, operation: SoftwareOperation): SoftwareStatus {
  if (operation === "inventory") return "loading";
  if (operation === "preview") return "previewing";
  if (operation === "uninstall") return "uninstalling";
  return state.status;
}

function selectableIds(inventory: SoftwareInventoryV1 | null): Set<string> {
  return new Set(inventory?.entries
    .filter((entry) => entry.eligibility.state === "selectable"
      && entry.eligibility.reason === "eligible_current_user_msix"
      && entry.identity.source === "msix"
      && entry.scope === "current_user")
    .map((entry) => entry.id) ?? []);
}

function sameIdentity(left: ReadonlySet<string>, right: readonly string[]): boolean {
  return left.size === right.length && right.every((id) => left.has(id));
}

export function softwareReducer(state: SoftwareState, action: SoftwareAction): SoftwareState {
  switch (action.type) {
    case "operation_requested":
      if (state.operationId !== null) return state;
      if (action.operation === "preview" && (!state.inventory || state.selectedIds.size === 0)) return state;
      if (action.operation === "uninstall" && (state.status !== "confirming" || !state.plan || !state.preview)) return state;
      return {
        ...state,
        resumeStatus: state.status,
        status: statusForOperation(state, action.operation),
        operationId: action.operationId,
        operation: action.operation,
        error: null,
      };
    case "cancel_requested":
      return state.operationId === action.operationId
        ? { ...state, status: "canceling" }
        : state;
    case "inventory_completed":
      if (state.operationId !== action.operationId || state.operation !== "inventory") return state;
      return {
        ...state,
        status: "ready",
        resumeStatus: "ready",
        operationId: null,
        operation: null,
        inventory: action.inventory,
        selectedIds: new Set(),
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
        status: state.operation === "audit" ? state.resumeStatus : state.inventory ? "ready" : "idle",
        resumeStatus: state.operation === "audit" ? state.resumeStatus : state.inventory ? "ready" : "idle",
        operationId: null,
        operation: null,
      };
    case "selection_changed": {
      if (!state.inventory || !["ready", "selecting", "preview_ready"].includes(state.status)
        || !selectableIds(state.inventory).has(action.softwareId)) return state;
      const selectedIds = new Set(state.selectedIds);
      if (action.selected) selectedIds.add(action.softwareId);
      else selectedIds.delete(action.softwareId);
      return { ...state, status: "selecting", selectedIds, plan: null, preview: null, report: null };
    }
    case "select_all": {
      if (!state.inventory || !["ready", "selecting", "preview_ready"].includes(state.status)) return state;
      return {
        ...state,
        status: "selecting",
        selectedIds: action.selected ? selectableIds(state.inventory) : new Set(),
        plan: null,
        preview: null,
        report: null,
      };
    }
    case "query_changed":
      return { ...state, query: action.query };
    case "sort_changed":
      return { ...state, sort: action.sort };
    case "expanded_changed":
      return { ...state, expandedId: action.softwareId };
    case "preview_completed":
      if (state.operationId !== action.operationId || state.operation !== "preview"
        || action.result.operation_id !== action.operationId
        || !sameIdentity(state.selectedIds, action.result.plan.selected_ids)) return state;
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
    case "confirmation_opened":
      return state.status === "preview_ready" && state.plan && state.preview
        ? { ...state, status: "confirming" }
        : state;
    case "confirmation_closed":
      return state.status === "confirming" ? { ...state, status: "preview_ready" } : state;
    case "uninstall_completed": {
      if (state.operationId !== action.operationId || state.operation !== "uninstall"
        || action.result.operation_id !== action.operationId) return state;
      const expected = state.preview?.selected.map((item) => item.id) ?? [];
      if (!sameIdentity(new Set(action.result.report.outcomes.map((item) => item.software_id)), expected)) return state;
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
      return { ...initialSoftwareState, query: state.query, sort: state.sort };
  }
}
