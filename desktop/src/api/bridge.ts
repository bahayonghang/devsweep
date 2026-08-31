import { Channel, invoke } from "@tauri-apps/api/core";
import { decodeCommandError, decodeDesktopAnalyzeProgress, decodeDesktopAnalyzeResult, decodeDesktopOptimizeAuditResult, decodeDesktopOptimizeListResult, decodeDesktopOptimizePreviewResult, decodeDesktopOptimizeRunResult, decodeDesktopScanProgress, decodeDesktopScanResult, decodeDesktopSoftwareAuditResult, decodeDesktopSoftwareInventoryResult, decodeDesktopSoftwarePreviewResult, decodeDesktopSoftwareUninstallResult, decodeDesktopStatusLiveResult, decodeDesktopStatusSnapshotResult, decodeDryRunOutcome, decodeExecutedReport, decodeStatusEvent, reportMatchesSelection } from "./contract";
import type { DesktopAnalyzeProgress, DesktopAnalyzeResult, DesktopOptimizeAuditResult, DesktopOptimizeListResult, DesktopOptimizePreviewResult, DesktopOptimizeRunResult, DesktopScanProgress, DesktopScanResult, DesktopSoftwareAuditResult, DesktopSoftwareInventoryResult, DesktopSoftwarePreviewResult, DesktopSoftwareUninstallResult, DesktopStatusLiveResult, DesktopStatusSnapshotResult, DryRunOutcome, ExecutionReport, MaintenancePlanV1, ScanOptions, SoftwareInventoryV1, SoftwareSelectionPlanV1, StatusEventV1, UntrustedPlan } from "./types.gen";

export interface DesktopBridge {
  analyzeStart(operationId: string, root: string, onProgress: (progress: DesktopAnalyzeProgress) => void, onProgressError: (error: unknown) => void): Promise<DesktopAnalyzeResult>;
  analyzeCancel(operationId: string): Promise<void>;
  softwareInventoryStart(operationId: string): Promise<DesktopSoftwareInventoryResult>;
  softwarePreview(operationId: string, inventory: SoftwareInventoryV1, selectedIds: string[]): Promise<DesktopSoftwarePreviewResult>;
  softwareUninstall(operationId: string, plan: SoftwareSelectionPlanV1, previewDigest: string, confirmed: boolean): Promise<DesktopSoftwareUninstallResult>;
  softwareAudit(operationId: string): Promise<DesktopSoftwareAuditResult>;
  softwareCancel(operationId: string): Promise<void>;
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
};
