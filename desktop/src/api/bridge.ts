import { Channel, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { decodeAnalyzeTrashPreview, decodeAnalyzeTrashReport, decodeCommandError, decodeDesktopAnalyzeProgress, decodeDesktopAnalyzeResult, decodeDesktopOptimizeAuditResult, decodeDesktopOptimizeListResult, decodeDesktopOptimizePreviewResult, decodeDesktopOptimizeRunResult, decodeDesktopScanProgress, decodeDesktopScanResult, decodeDesktopSoftwareAuditResult, decodeDesktopSoftwareInventoryResult, decodeDesktopSoftwareLeftoversPreviewResult, decodeDesktopSoftwareLeftoversResult, decodeDesktopSoftwarePreviewResult, decodeDesktopSoftwareUninstallResult, decodeDesktopSoftwareUpdatesResult, decodeDesktopStatusLiveResult, decodeDesktopStatusSnapshotResult, decodeDryRunOutcome, decodeExecutedReport, decodeHudStatusEvent, decodeSoftwareStartupList, decodeSoftwareStartupToggleReport, decodeStatusEvent, reportMatchesSelection } from "./contract";
import type { AnalyzeTrashPreviewV1, AnalyzeTrashReportV1, DesktopAnalyzeProgress, DesktopAnalyzeResult, DesktopOptimizeAuditResult, DesktopOptimizeListResult, DesktopOptimizePreviewResult, DesktopOptimizeRunResult, DesktopScanProgress, DesktopScanResult, DesktopSoftwareAuditResult, DesktopSoftwareInventoryResult, DesktopSoftwareLeftoversPreviewResult, DesktopSoftwareLeftoversResult, DesktopSoftwarePreviewResult, DesktopSoftwareUninstallResult, DesktopSoftwareUpdatesResult, DesktopStatusLiveResult, DesktopStatusSnapshotResult, DryRunOutcome, ExecutionReport, CleanMovedTotalsV1, HudStatusEvent, MaintenancePlanV1, ScanOptions, SoftwareInventoryV1, SoftwareLeftoverPlanV1, SoftwareLeftoverSelectionV1, SoftwareSelectionPlanV1, SoftwareStartupListV1, SoftwareStartupToggleReportV1, StatusEventV1, UntrustedPlan } from "./types.gen";
import {
  decodeCleanMovedTotals,
  decodeHistoryDetail,
  decodeHistoryList,
  decodeProtectionMutation,
  decodeProtectionPaths,
  decodeRuleProjection,
  decodeRules,
} from "../support/decode";
import type {
  HistoryDetailV1,
  HistoryDomain,
  HistoryListV1,
  ProtectionMutationReport,
  RuleProjectionV1,
} from "../support/types";

export interface DesktopBridge {
  analyzeStart(operationId: string, root: string, onProgress: (progress: DesktopAnalyzeProgress) => void, onProgressError: (error: unknown) => void): Promise<DesktopAnalyzeResult>;
  analyzeCancel(operationId: string): Promise<void>;
  analyzeDefaultRoot(): Promise<string>;
  analyzeReveal(operationId: string, nodeId: number): Promise<void>;
  analyzeTrashPreview(operationId: string, nodeIds: number[]): Promise<AnalyzeTrashPreviewV1>;
  analyzeTrashExecute(operationId: string, nodeIds: number[], digest: string, confirmed: boolean): Promise<AnalyzeTrashReportV1>;
  softwareInventoryStart(operationId: string): Promise<DesktopSoftwareInventoryResult>;
  softwarePreview(operationId: string, inventory: SoftwareInventoryV1, selectedIds: string[]): Promise<DesktopSoftwarePreviewResult>;
  softwareUninstall(operationId: string, plan: SoftwareSelectionPlanV1, previewDigest: string, confirmed: boolean): Promise<DesktopSoftwareUninstallResult>;
  softwareAudit(operationId: string): Promise<DesktopSoftwareAuditResult>;
  softwareCancel(operationId: string): Promise<void>;
  softwareUpdatesCheck(operationId: string, inventory: SoftwareInventoryV1 | null): Promise<DesktopSoftwareUpdatesResult>;
  softwareStartupList(): Promise<SoftwareStartupListV1>;
  softwareStartupSet(entryId: string, enabled: boolean, confirmed: boolean): Promise<SoftwareStartupToggleReportV1>;
  softwareLeftoversPreview(operationId: string, inventory: SoftwareInventoryV1, selectedIds: string[], selection: SoftwareLeftoverSelectionV1 | null): Promise<DesktopSoftwareLeftoversPreviewResult>;
  softwareLeftoversExecute(operationId: string, plan: SoftwareLeftoverPlanV1, previewDigest: string, confirmed: boolean): Promise<DesktopSoftwareLeftoversResult>;
  optimizeListStart(operationId: string): Promise<DesktopOptimizeListResult>;
  optimizePreview(operationId: string, catalogueId: string): Promise<DesktopOptimizePreviewResult>;
  optimizeRun(operationId: string, plan: MaintenancePlanV1, previewDigest: string, confirmed: boolean): Promise<DesktopOptimizeRunResult>;
  optimizeAudit(operationId: string): Promise<DesktopOptimizeAuditResult>;
  optimizeCancel(operationId: string): Promise<void>;
  statusSnapshot(operationId: string, processLimit?: number): Promise<DesktopStatusSnapshotResult>;
  statusLiveStart(operationId: string, intervalMs: number, processLimit: number, onEvent: (event: StatusEventV1) => void, onEventError: (error: unknown) => void): Promise<DesktopStatusLiveResult>;
  statusCancel(operationId: string): Promise<void>;
  scanStart(scanId: string, options: ScanOptions, onProgress: (progress: DesktopScanProgress) => void, onProgressError: (error: unknown) => void): Promise<DesktopScanResult>;
  scanCancel(scanId: string): Promise<void>;
  planDryRun(plan: UntrustedPlan, selectedIds: string[]): Promise<DryRunOutcome>;
  planExecute(plan: UntrustedPlan, selectedIds: string[], digest: string): Promise<ExecutionReport>;
  protectionList(): Promise<string[]>;
  protectionAdd(path: string, confirm: boolean): Promise<ProtectionMutationReport>;
  protectionRemove(path: string, confirm: boolean): Promise<ProtectionMutationReport>;
  rulesList(): Promise<RuleProjectionV1[]>;
  rulesShow(id: string): Promise<RuleProjectionV1>;
  historyList(domain?: HistoryDomain, limit?: number): Promise<HistoryListV1>;
  historyShow(operationId: string): Promise<HistoryDetailV1>;
  historyCleanTotals(): Promise<CleanMovedTotalsV1>;
}

function bridgeError(error: unknown) {
  try { return decodeCommandError(error); }
  catch { return { code: "io" as const, message: error instanceof Error ? error.message : "Unexpected Tauri IPC error" }; }
}

async function call<T>(command: string, args: Record<string, unknown>, decode: (value: unknown) => T): Promise<T> {
  let value: unknown;
  try { value = await invoke<unknown>(command, args); }
  catch (error) { throw bridgeError(error); }
  return decode(value);
}

export const tauriBridge: DesktopBridge = {
  analyzeStart: async (operationId, root, onProgress, onProgressError) => {
    const channel = new Channel<unknown>((value) => {
      try { onProgress(decodeDesktopAnalyzeProgress(value)); }
      catch (error) { onProgressError(error); }
    });
    const result = await call("analyze_start", { operationId, root, onProgress: channel }, decodeDesktopAnalyzeResult);
    if (result.operation_id !== operationId) throw new Error("Analyze result does not match the active operation");
    return result;
  },
  analyzeCancel: async (operationId) => { try { await invoke("analyze_cancel", { operationId }); } catch (error) { throw bridgeError(error); } },
  analyzeDefaultRoot: () => call("analyze_default_root", {}, (value) => {
    if (typeof value !== "string" || value.length === 0) throw new Error("Invalid analyze default root");
    return value;
  }),
  analyzeReveal: async (operationId, nodeId) => { try { await invoke("analyze_reveal", { operationId, nodeId }); } catch (error) { throw bridgeError(error); } },
  analyzeTrashPreview: (operationId, nodeIds) => call("analyze_trash_preview", { operationId, nodeIds }, (value) => {
    const preview = decodeAnalyzeTrashPreview(value);
    const requested = new Set(nodeIds);
    if (preview.operation_id !== operationId || [...preview.items, ...preview.refused].some((entry) => !requested.has(entry.node_id))) {
      throw new Error("Analyze trash preview does not match the request");
    }
    return preview;
  }),
  analyzeTrashExecute: (operationId, nodeIds, digest, confirmed) => call("analyze_trash_execute", { operationId, nodeIds, digest, confirmed }, (value) => {
    const report = decodeAnalyzeTrashReport(value);
    const requested = new Set(nodeIds);
    if (report.operation_id !== operationId || report.report.confirmation_digest !== digest || report.moved_node_ids.some((nodeId) => !requested.has(nodeId))) {
      throw new Error("Analyze trash report does not match the confirmed request");
    }
    return report;
  }),
  softwareInventoryStart: async (operationId) => {
    const result = await call("software_inventory_start", { operationId }, decodeDesktopSoftwareInventoryResult);
    if (result.operation_id !== operationId) throw new Error("Software inventory result does not match the active operation");
    return result;
  },
  softwarePreview: async (operationId, inventory, selectedIds) => {
    const result = await call("software_preview", { operationId, inventory, selectedIds }, decodeDesktopSoftwarePreviewResult);
    if (result.operation_id !== operationId) throw new Error("Software preview result does not match the active operation");
    return result;
  },
  softwareUninstall: async (operationId, plan, previewDigest, confirmed) => {
    const result = await call("software_uninstall", { operationId, plan, previewDigest, confirmed }, decodeDesktopSoftwareUninstallResult);
    if (result.operation_id !== operationId) throw new Error("Software uninstall result does not match the active operation");
    return result;
  },
  softwareAudit: async (operationId) => {
    const result = await call("software_audit", { operationId }, decodeDesktopSoftwareAuditResult);
    if (result.operation_id !== operationId) throw new Error("Software audit result does not match the active operation");
    return result;
  },
  softwareCancel: async (operationId) => { try { await invoke("software_cancel", { operationId }); } catch (error) { throw bridgeError(error); } },
  softwareUpdatesCheck: async (operationId, inventory) => {
    const result = await call("software_updates_check", { operationId, inventory }, decodeDesktopSoftwareUpdatesResult);
    if (result.operation_id !== operationId) throw new Error("Software updates result does not match the active operation");
    return result;
  },
  softwareStartupList: () => call("software_startup_list", {}, decodeSoftwareStartupList),
  softwareStartupSet: (entryId, enabled, confirmed) => call("software_startup_set", { entryId, enabled, confirmed }, (value) => {
    const report = decodeSoftwareStartupToggleReport(value);
    if (report.entry.id !== entryId || report.requested_enabled !== enabled) throw new Error("Startup toggle report does not match the request");
    return report;
  }),
  softwareLeftoversPreview: async (operationId, inventory, selectedIds, selection) => {
    const result = await call("software_leftovers_preview", { operationId, inventory, selectedIds, selection }, decodeDesktopSoftwareLeftoversPreviewResult);
    if (result.operation_id !== operationId) throw new Error("Software leftovers result does not match the active operation");
    return result;
  },
  softwareLeftoversExecute: async (operationId, plan, previewDigest, confirmed) => {
    const result = await call("software_leftovers_execute", { operationId, plan, previewDigest, confirmed }, decodeDesktopSoftwareLeftoversResult);
    if (result.operation_id !== operationId) throw new Error("Software leftovers result does not match the active operation");
    return result;
  },
  optimizeListStart: async (operationId) => {
    const result = await call("optimize_list", { operationId }, decodeDesktopOptimizeListResult);
    if (result.operation_id !== operationId) throw new Error("Optimize list result does not match the active operation");
    return result;
  },
  optimizePreview: async (operationId, catalogueId) => {
    const result = await call("optimize_preview", { operationId, catalogueId }, decodeDesktopOptimizePreviewResult);
    if (result.operation_id !== operationId) throw new Error("Optimize preview result does not match the active operation");
    return result;
  },
  optimizeRun: async (operationId, plan, previewDigest, confirmed) => {
    const result = await call("optimize_run", { operationId, plan, previewDigest, confirmed }, decodeDesktopOptimizeRunResult);
    if (result.operation_id !== operationId) throw new Error("Optimize run result does not match the active operation");
    return result;
  },
  optimizeAudit: async (operationId) => {
    const result = await call("optimize_audit", { operationId }, decodeDesktopOptimizeAuditResult);
    if (result.operation_id !== operationId) throw new Error("Optimize audit result does not match the active operation");
    return result;
  },
  optimizeCancel: async (operationId) => { try { await invoke("optimize_cancel", { operationId }); } catch (error) { throw bridgeError(error); } },
  statusSnapshot: async (operationId, processLimit) => {
    const result = await call("status_snapshot", { operationId, processLimit: processLimit ?? null }, decodeDesktopStatusSnapshotResult);
    if (result.operation_id !== operationId) throw new Error("Status snapshot result does not match the active operation");
    return result;
  },
  statusLiveStart: async (operationId, intervalMs, processLimit, onEvent, onEventError) => {
    const channel = new Channel<unknown>((value) => {
      try { onEvent(decodeStatusEvent(value)); }
      catch (error) { onEventError(error); }
    });
    const result = await call("status_live_start", { operationId, intervalMs, processLimit, onEvent: channel }, decodeDesktopStatusLiveResult);
    if (result.operation_id !== operationId) throw new Error("Status live result does not match the active operation");
    return result;
  },
  statusCancel: async (operationId) => { try { await invoke("status_cancel", { operationId }); } catch (error) { throw bridgeError(error); } },
  scanStart: async (scanId, options, onProgress, onProgressError) => {
    const channel = new Channel<unknown>((value) => {
      try { onProgress(decodeDesktopScanProgress(value)); }
      catch (error) { onProgressError(error); }
    });
    const result = await call("scan_start", { scanId, options, onProgress: channel }, decodeDesktopScanResult);
    if (result.scan_id !== scanId) throw new Error("Scan result does not match the active scan");
    return result;
  },
  scanCancel: async (scanId) => { try { await invoke("scan_cancel", { scanId }); } catch (error) { throw bridgeError(error); } },
  planDryRun: (plan, selectedIds) => call("plan_dry_run", { plan, selectedIds }, (value) => {
    const outcome = decodeDryRunOutcome(value);
    if (!reportMatchesSelection(outcome.report, selectedIds)) throw new Error("Dry-run response does not match the requested selection");
    return outcome;
  }),
  planExecute: (plan, selectedIds, digest) => call("plan_execute", { plan, selectedIds, digest }, (value) => {
    const report = decodeExecutedReport(value);
    if (!reportMatchesSelection(report, selectedIds) || report.confirmation_digest !== digest) throw new Error("Execution response does not match the confirmed request");
    return report;
  }),
  protectionList: () => call("protection_list", {}, decodeProtectionPaths),
  protectionAdd: (path, confirm) => call("protection_add", { path, confirm }, decodeProtectionMutation),
  protectionRemove: (path, confirm) => call("protection_remove", { path, confirm }, decodeProtectionMutation),
  rulesList: () => call("rules_list", {}, decodeRules),
  rulesShow: (id) => call("rules_show", { id }, decodeRuleProjection),
  historyList: (domain, limit) => call("history_list", { domain: domain ?? null, limit: limit ?? null }, decodeHistoryList),
  historyShow: (operationId) => call("history_show", { operationId }, decodeHistoryDetail),
  historyCleanTotals: () => call("history_clean_totals", {}, decodeCleanMovedTotals),
};

/** The HUD window event name. The backend emits it only to the HUD window. */
export const HUD_STATUS_EVENT = "hud-status";

/**
 * Subscribe the HUD window to backend samples. The HUD sends no command; each
 * payload is decoded closed before the handler sees it.
 */
export function listenHudStatus(onEvent: (event: HudStatusEvent) => void, onEventError: (error: unknown) => void): Promise<() => void> {
  return listen<unknown>(HUD_STATUS_EVENT, (event) => {
    try { onEvent(decodeHudStatusEvent(event.payload)); }
    catch (error) { onEventError(error); }
  });
}
