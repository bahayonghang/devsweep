import scanJson from "./fixtures/scan-report.json";
import dryRunJson from "./fixtures/dry-run-outcome.json";
import twoTargetDryRunJson from "./fixtures/dry-run-outcome-two-targets.json";
import executionJson from "./fixtures/execution-report.json";
import { decodeDryRunOutcome, decodeExecutionReport, decodeScanReport } from "./contract";
import type { DesktopBridge } from "./bridge";
import type { ScanProgress } from "./types.gen";

const scanReport = decodeScanReport(scanJson);
const dryRun = decodeDryRunOutcome(dryRunJson);
const twoTargetDryRun = decodeDryRunOutcome(twoTargetDryRunJson);
const execution = decodeExecutionReport(executionJson);
const cargoTargetId = "cargo.target:C:/work/app/target";
const npmTargetId = "npm.cache.clean:global";
const inspectOnlyTargetId = "cargo.home.inspect:C:/Users/dev/.cargo";
let progressHandler: ((progress: ScanProgress) => void) | undefined;
let scanCount = 0;
let cancelFirstScan: ((error: unknown) => void) | undefined;

export const fixtureBridge: DesktopBridge = {
  scanStart: async () => {
    scanCount += 1;
    progressHandler?.({ phase: "projects", message: "Scanning controlled project fixture", partial: null });
    if (scanCount === 1) {
      return new Promise((_, reject) => { cancelFirstScan = reject; });
    }
    await new Promise((resolve) => setTimeout(resolve, 350));
    progressHandler?.({ phase: "global", message: "Reviewing controlled provider fixture", partial: null });
    await new Promise((resolve) => setTimeout(resolve, 350));
    return scanReport;
  },
  scanCancel: async () => {
    cancelFirstScan?.({ code: "scan_failed", message: "Controlled fixture scan canceled" });
    cancelFirstScan = undefined;
  },
  planDryRun: async (_plan, selectedIds) => {
    const selected = new Set(selectedIds);
    if (selected.has(inspectOnlyTargetId)) throw { code: "inspect_only_target", target_id: inspectOnlyTargetId };
    const unknown = selectedIds.find((id) => id !== cargoTargetId && id !== npmTargetId);
    if (unknown) throw { code: "unknown_target", target_id: unknown };
    if (selected.size === 1 && selected.has(cargoTargetId)) return dryRun;
    if (selected.size === 2 && selected.has(cargoTargetId) && selected.has(npmTargetId)) return twoTargetDryRun;
    throw { code: "invalid_plan", issues: ["Controlled fixture supports the cargo target alone or cargo plus npm"] };
  },
  planExecute: async (_plan, selectedIds, digest) => {
    const selected = new Set(selectedIds);
    if (selected.size === 2 && selected.has(cargoTargetId) && selected.has(npmTargetId) && digest === twoTargetDryRun.digest) return execution;
    if (selected.size === 1 && selected.has(cargoTargetId) && digest === dryRun.digest) {
      return { ...dryRun.report, dry_run: false, audit_log: "C:/Users/dev/AppData/Roaming/devsweep/audit/devsweep-audit.jsonl" };
    }
    throw { code: "stale_confirmation", expected_digest: digest, actual_digest: selected.size === 2 ? twoTargetDryRun.digest : dryRun.digest };
  },
  onScanProgress: async (handler) => {
    progressHandler = handler;
    return () => { if (progressHandler === handler) progressHandler = undefined; };
  },
};
