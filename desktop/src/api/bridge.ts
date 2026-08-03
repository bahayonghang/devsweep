import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { decodeCommandError, decodeDryRunOutcome, decodeExecutedReport, decodeScanProgress, decodeScanReport, reportMatchesSelection } from "./contract";
import type { DryRunOutcome, ExecutionReport, ScanOptions, ScanProgress, ScanReport, UntrustedPlan } from "./types.gen";

export interface DesktopBridge {
  scanStart(options: ScanOptions): Promise<ScanReport>;
  scanCancel(): Promise<void>;
  planDryRun(plan: UntrustedPlan, selectedIds: string[]): Promise<DryRunOutcome>;
  planExecute(plan: UntrustedPlan, selectedIds: string[], digest: string): Promise<ExecutionReport>;
  onScanProgress(handler: (progress: ScanProgress) => void, onError: (error: unknown) => void): Promise<() => void>;
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
  scanStart: (options) => call("scan_start", { options }, decodeScanReport),
  scanCancel: async () => { try { await invoke("scan_cancel"); } catch (error) { throw bridgeError(error); } },
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
  onScanProgress: (handler, onError) => listen<unknown>("scan://progress", (event) => {
    try { handler(decodeScanProgress(event.payload)); }
    catch (error) { onError(error); }
  }),
};
