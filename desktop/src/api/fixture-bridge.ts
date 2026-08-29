import scanJson from "./fixtures/scan-report.json";
import progressJson from "./fixtures/scan-progress.json";
import dryRunJson from "./fixtures/dry-run-outcome.json";
import twoTargetDryRunJson from "./fixtures/dry-run-outcome-two-targets.json";
import executionJson from "./fixtures/execution-report.json";
import { decodeDesktopScanProgress, decodeDryRunOutcome, decodeExecutionReport, decodeScanReport } from "./contract";
import type { DesktopBridge } from "./bridge";
import type { DesktopScanProgress, DesktopScanResult } from "./types.gen";

const scanReport = decodeScanReport(scanJson);
const projectProgress = decodeDesktopScanProgress(progressJson);
const dryRun = decodeDryRunOutcome(dryRunJson);
const twoTargetDryRun = decodeDryRunOutcome(twoTargetDryRunJson);
const execution = decodeExecutionReport(executionJson);
const cargoTargetId = "cargo.target:C:/work/app/target";
const npmTargetId = "npm.cache.clean:global";
const inspectOnlyTargetId = "cargo.home.inspect:C:/Users/dev/.cargo";
let scanCount = 0;
let pendingCancellation: { scanId: string; resolve: (result: DesktopScanResult) => void } | undefined;

function correlatedProjectProgress(scanId: string): DesktopScanProgress {
  return { ...projectProgress, scan_id: scanId, sequence: 1 };
}

function mixedProgress(scanId: string): DesktopScanProgress {
  const projectTarget = projectProgress.preview!.targets[0];
  const globalTarget = {
    ...projectTarget,
    id: "fixture.npm.cache:global",
    scope: { type: "global" as const },
    ecosystem: "node" as const,
    kind: "package_cache" as const,
    path: "C:/Users/dev/AppData/Local/npm-cache",
    estimated_bytes: 8192,
  };
  return {
    scan_id: scanId,
    sequence: 3,
    phase: "global",
    message: "Reviewing controlled provider fixture",
    preview: {
      targets: [globalTarget, projectTarget],
      totals: {
        target_count: 2,
        verified_bytes: 12288,
        partial_lower_bound_bytes: 0,
        unknown_target_count: 0,
      },
    },
  };
}

export const fixtureBridge: DesktopBridge = {
  scanStart: async (scanId, _options, onProgress) => {
    scanCount += 1;
    onProgress(correlatedProjectProgress(scanId));
    if (scanCount === 1) {
      return new Promise((resolve) => { pendingCancellation = { scanId, resolve }; });
    }
    await new Promise((resolve) => setTimeout(resolve, 350));
    onProgress(mixedProgress(scanId));
    onProgress({ ...correlatedProjectProgress(scanId), sequence: 2, message: "Controlled stale progress" });
    await new Promise((resolve) => setTimeout(resolve, 350));
    return { type: "completed", scan_id: scanId, report: scanReport };
  },
  scanCancel: async (scanId) => {
    if (pendingCancellation?.scanId === scanId) {
      pendingCancellation.resolve({ type: "canceled", scan_id: scanId });
      pendingCancellation = undefined;
    }
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
};
