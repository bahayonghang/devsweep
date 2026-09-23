import type {
  CommandError,
  DesktopSoftwareAuditResult,
  DesktopSoftwarePreviewResult,
  DesktopSoftwareLeftoversPreviewResult,
  DesktopSoftwareLeftoversResult,
  DesktopSoftwareUninstallResult,
  DesktopSoftwareUpdatesResult,
  SoftwareInventoryV1,
  SoftwareLeftoverPlanPreviewV1,
  SoftwareLeftoverPlanV1,
  SoftwareLeftoverPreviewV1,
  SoftwarePreviewV1,
  SoftwareSelectionPlanV1,
  SoftwareStartupListV1,
  SoftwareStartupToggleReportV1,
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

export type SoftwareOperation = "inventory" | "preview" | "uninstall" | "audit" | "updates" | "leftover_plan" | "leftover_move";
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
  readonly updates: DesktopSoftwareUpdatesResult["updates"] | null;
  readonly startup: SoftwareStartupListV1 | null;
  readonly leftovers: SoftwareLeftoverPreviewV1 | null;
  readonly leftoverSelection: ReadonlySet<string>;
  readonly leftoverPlan: { readonly plan: SoftwareLeftoverPlanV1; readonly preview: SoftwareLeftoverPlanPreviewV1 } | null;
  readonly leftoverConfirming: boolean;
  readonly leftoverReport: DesktopSoftwareLeftoversResult["report"] | null;
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
  updates: null,
  startup: null,
  leftovers: null,
  leftoverSelection: new Set(),
  leftoverPlan: null,
  leftoverConfirming: false,
  leftoverReport: null,
  query: "",
  sort: "name",
  expandedId: null,
  error: null,
};

type PlannedLeftovers = Extract<DesktopSoftwareLeftoversPreviewResult, { type: "planned" }>;

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
  | { readonly type: "preview_completed"; readonly operationId: string; readonly result: DesktopSoftwarePreviewResult; readonly leftovers?: SoftwareLeftoverPreviewV1 }
  | { readonly type: "confirmation_opened" }
  | { readonly type: "confirmation_closed" }
  | { readonly type: "uninstall_completed"; readonly operationId: string; readonly result: DesktopSoftwareUninstallResult }
  | { readonly type: "audit_completed"; readonly operationId: string; readonly result: DesktopSoftwareAuditResult }
  | { readonly type: "updates_completed"; readonly operationId: string; readonly result: DesktopSoftwareUpdatesResult }
  | { readonly type: "startup_listed"; readonly list: SoftwareStartupListV1 }
  | { readonly type: "startup_toggled"; readonly report: SoftwareStartupToggleReportV1 }
  | { readonly type: "leftover_selection_changed"; readonly candidateId: string; readonly selected: boolean }
  | { readonly type: "leftovers_planned"; readonly operationId: string; readonly result: PlannedLeftovers }
  | { readonly type: "leftover_confirmation_opened" }
  | { readonly type: "leftover_confirmation_closed" }
  | { readonly type: "leftovers_moved"; readonly operationId: string; readonly result: DesktopSoftwareLeftoversResult }
  | { readonly type: "support_failed"; readonly error: CommandError }
  | { readonly type: "operation_failed"; readonly operationId: string; readonly error: CommandError }
  | { readonly type: "error_dismissed" }
  | { readonly type: "released" };

/** Operations that leave the uninstall state machine where it was. */
const RESUMING_OPERATIONS: ReadonlySet<SoftwareOperation> = new Set(["audit", "updates", "leftover_plan", "leftover_move"]);

/** Removal of leftovers requires a removed or reboot-required uninstall outcome. */
export function uninstallSucceeded(state: SoftwareState, softwareId: string): string | null {
  const outcome = state.report?.outcomes.find((item) => item.software_id === softwareId
    && (item.outcome === "removed" || item.outcome === "reboot_required"));
  return outcome?.operation_id ?? null;
}

const LEFTOVERS_CLEARED = {
  leftovers: null,
  leftoverSelection: new Set<string>(),
  leftoverPlan: null,
  leftoverConfirming: false,
  leftoverReport: null,
} as const;

/** Accepts discovery only for the selected apps; certain candidates start selected. */
function discoveredLeftovers(state: SoftwareState, preview: SoftwareLeftoverPreviewV1 | undefined) {
  if (!preview || !preview.apps.every((app) => state.selectedIds.has(app.software_id))) return {};
  return {
    leftovers: preview,
    leftoverSelection: new Set(preview.apps.flatMap((app) => app.candidates
      .filter((candidate) => candidate.selected_by_default)
      .map((candidate) => candidate.id))),
  };
}

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
      if (action.operation === "leftover_move" && (!state.leftoverConfirming || !state.leftoverPlan)) return state;
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
        ...LEFTOVERS_CLEARED,
        expandedId: null,
        error: null,
      };
    case "operation_canceled": {
      if (state.operationId !== action.operationId) return state;
      const resume = state.operation !== null && RESUMING_OPERATIONS.has(state.operation);
      return {
        ...state,
        status: resume ? state.resumeStatus : state.inventory ? "ready" : "idle",
        resumeStatus: resume ? state.resumeStatus : state.inventory ? "ready" : "idle",
        operationId: null,
        operation: null,
      };
    }
    case "selection_changed": {
      if (!state.inventory || !["ready", "selecting", "preview_ready"].includes(state.status)
        || !selectableIds(state.inventory).has(action.softwareId)) return state;
      const selectedIds = new Set(state.selectedIds);
      if (action.selected) selectedIds.add(action.softwareId);
      else selectedIds.delete(action.softwareId);
      return { ...state, status: "selecting", selectedIds, plan: null, preview: null, report: null, ...LEFTOVERS_CLEARED };
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
        ...LEFTOVERS_CLEARED,
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
        ...LEFTOVERS_CLEARED,
        ...discoveredLeftovers(state, action.leftovers),
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
    case "updates_completed":
      if (state.operationId !== action.operationId || state.operation !== "updates"
        || action.result.operation_id !== action.operationId) return state;
      return { ...state, status: state.resumeStatus, operationId: null, operation: null, updates: action.result.updates, error: null };
    case "startup_listed":
      return { ...state, startup: action.list };
    case "startup_toggled":
      if (!state.startup) return state;
      return {
        ...state,
        startup: {
          ...state.startup,
          entries: state.startup.entries.map((entry) => entry.id === action.report.entry.id ? action.report.entry : entry),
        },
      };
    case "leftover_selection_changed": {
      if (!state.leftovers || state.operationId !== null
        || !state.leftovers.apps.some((app) => app.candidates.some((candidate) => candidate.id === action.candidateId))) return state;
      const leftoverSelection = new Set(state.leftoverSelection);
      if (action.selected) leftoverSelection.add(action.candidateId);
      else leftoverSelection.delete(action.candidateId);
      return { ...state, leftoverSelection, leftoverPlan: null, leftoverConfirming: false };
    }
    case "leftovers_planned":
      if (state.operationId !== action.operationId || state.operation !== "leftover_plan"
        || action.result.operation_id !== action.operationId
        || uninstallSucceeded(state, action.result.plan.software_id) !== action.result.plan.uninstall_operation_id) return state;
      return {
        ...state,
        status: state.resumeStatus,
        operationId: null,
        operation: null,
        leftoverPlan: { plan: action.result.plan, preview: action.result.preview },
        leftoverConfirming: false,
        leftoverReport: null,
        error: null,
      };
    case "leftover_confirmation_opened":
      return state.leftoverPlan && state.operationId === null ? { ...state, leftoverConfirming: true } : state;
    case "leftover_confirmation_closed":
      return { ...state, leftoverConfirming: false };
    case "leftovers_moved":
      if (state.operationId !== action.operationId || state.operation !== "leftover_move"
        || action.result.operation_id !== action.operationId
        || action.result.report.software_id !== state.leftoverPlan?.plan.software_id) return state;
      return {
        ...state,
        status: state.resumeStatus,
        operationId: null,
        operation: null,
        leftoverPlan: null,
        leftoverConfirming: false,
        leftoverReport: action.result.report,
        error: null,
      };
    case "support_failed":
      return { ...state, error: action.error };
    case "operation_failed": {
      if (state.operationId !== action.operationId) return state;
      const resume = state.operation !== null && RESUMING_OPERATIONS.has(state.operation);
      return {
        ...state,
        status: resume ? state.resumeStatus : "unknown",
        resumeStatus: resume ? state.resumeStatus : "unknown",
        operationId: null,
        operation: null,
        leftoverConfirming: false,
        error: action.error,
      };
    }
    case "error_dismissed":
      return { ...state, error: null };
    case "released":
      return { ...initialSoftwareState, query: state.query, sort: state.sort, updates: state.updates, startup: state.startup };
  }
}
