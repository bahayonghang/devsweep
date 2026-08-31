import { Channel, invoke } from "@tauri-apps/api/core";
import { decodeCommandError, decodeDesktopAnalyzeProgress, decodeDesktopAnalyzeResult, decodeDesktopScanProgress, decodeDesktopScanResult, decodeDesktopSoftwareAuditResult, decodeDesktopSoftwareInventoryResult, decodeDesktopSoftwarePreviewResult, decodeDesktopSoftwareUninstallResult, decodeDryRunOutcome, decodeExecutedReport, reportMatchesSelection } from "./contract";
import type { DesktopAnalyzeProgress, DesktopAnalyzeResult, DesktopScanProgress, DesktopScanResult, DesktopSoftwareAuditResult, DesktopSoftwareInventoryResult, DesktopSoftwarePreviewResult, DesktopSoftwareUninstallResult, DryRunOutcome, ExecutionReport, ScanOptions, SoftwareInventoryV1, SoftwareSelectionPlanV1, UntrustedPlan } from "./types.gen";

export interface DesktopBridge {
  analyzeStart(operationId: string, root: string, onProgress: (progress: DesktopAnalyzeProgress) => void, onProgressError: (error: unknown) => void): Promise<DesktopAnalyzeResult>;
  analyzeCancel(operationId: string): Promise<void>;
  softwareInventoryStart(operationId: string): Promise<DesktopSoftwareInventoryResult>;
  softwarePreview(operationId: string, inventory: SoftwareInventoryV1, selectedIds: string[]): Promise<DesktopSoftwarePreviewResult>;
  softwareUninstall(operationId: string, plan: SoftwareSelectionPlanV1, previewDigest: string, confirmed: boolean): Promise<DesktopSoftwareUninstallResult>;
  softwareAudit(operationId: string): Promise<DesktopSoftwareAuditResult>;
  softwareCancel(operationId: string): Promise<void>;
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
