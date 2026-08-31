import scanJson from "./fixtures/scan-report.json";
import progressJson from "./fixtures/scan-progress.json";
import dryRunJson from "./fixtures/dry-run-outcome.json";
import twoTargetDryRunJson from "./fixtures/dry-run-outcome-two-targets.json";
import executionJson from "./fixtures/execution-report.json";
import analyzeCompleteJson from "./fixtures/analyze/complete.json";
import analyzeProgressJson from "./fixtures/analyze/progress.json";
import softwareInventoryJson from "./fixtures/software/inventory.json";
import softwarePreviewJson from "./fixtures/software/preview.json";
import softwareExecutionJson from "./fixtures/software/execution-five-terminal.json";
import softwareAuditJson from "./fixtures/software/audit-restart.json";
import optimizeCatalogueJson from "./fixtures/optimize/catalogue.json";
import optimizePreviewDnsJson from "./fixtures/optimize/preview-dns.json";
import optimizePreviewSearchJson from "./fixtures/optimize/preview-settings-search.json";
import optimizePreviewStorageJson from "./fixtures/optimize/preview-settings-storage.json";
import optimizePreviewEnergyJson from "./fixtures/optimize/preview-settings-energy.json";
import optimizeExecutionDnsJson from "./fixtures/optimize/execution-dns-succeeded.json";
import optimizeExecutionSettingsJson from "./fixtures/optimize/execution-settings-launched.json";
import optimizeAuditJson from "./fixtures/optimize/audit.json";
import optimizeRefusalGuidanceJson from "./fixtures/optimize/refusal-guidance.json";
import optimizeRefusalStaleJson from "./fixtures/optimize/refusal-stale.json";
import { decodeCommandError, decodeDesktopAnalyzeProgress, decodeDesktopAnalyzeResult, decodeDesktopOptimizeAuditResult, decodeDesktopOptimizeListResult, decodeDesktopOptimizePreviewResult, decodeDesktopOptimizeRunResult, decodeDesktopScanProgress, decodeDesktopSoftwareAuditResult, decodeDesktopSoftwarePreviewResult, decodeDesktopSoftwareUninstallResult, decodeDryRunOutcome, decodeExecutionReport, decodeScanReport, decodeSoftwareInventory } from "./contract";
import type { DesktopBridge } from "./bridge";
import type { DesktopAnalyzeProgress, DesktopAnalyzeResult, DesktopScanProgress, DesktopScanResult } from "./types.gen";

const scanReport = decodeScanReport(scanJson);
const projectProgress = decodeDesktopScanProgress(progressJson);
const dryRun = decodeDryRunOutcome(dryRunJson);
const twoTargetDryRun = decodeDryRunOutcome(twoTargetDryRunJson);
const execution = decodeExecutionReport(executionJson);
const analyzeComplete = decodeDesktopAnalyzeResult(analyzeCompleteJson);
const analyzeProgress = decodeDesktopAnalyzeProgress(analyzeProgressJson);
const softwareInventory = decodeSoftwareInventory(softwareInventoryJson);
const softwarePreview = decodeDesktopSoftwarePreviewResult(softwarePreviewJson);
const softwareExecution = decodeDesktopSoftwareUninstallResult(softwareExecutionJson);
const softwareAudit = decodeDesktopSoftwareAuditResult(softwareAuditJson);
const optimizeCatalogue = decodeDesktopOptimizeListResult(optimizeCatalogueJson);
const optimizePreviewById = {
  "dns.flush": decodeDesktopOptimizePreviewResult(optimizePreviewDnsJson),
  "settings.search": decodeDesktopOptimizePreviewResult(optimizePreviewSearchJson),
  "settings.storage_recommendations": decodeDesktopOptimizePreviewResult(optimizePreviewStorageJson),
  "settings.energy_recommendations": decodeDesktopOptimizePreviewResult(optimizePreviewEnergyJson),
} as const;
const optimizeExecutionDns = decodeDesktopOptimizeRunResult(optimizeExecutionDnsJson);
const optimizeExecutionSettings = decodeDesktopOptimizeRunResult(optimizeExecutionSettingsJson);
const optimizeAudit = decodeDesktopOptimizeAuditResult(optimizeAuditJson);
const optimizeRefusalGuidance = decodeCommandError(optimizeRefusalGuidanceJson);
const optimizeRefusalStale = decodeCommandError(optimizeRefusalStaleJson);
const cargoTargetId = "cargo.target:C:/work/app/target";
const npmTargetId = "npm.cache.clean:global";
const inspectOnlyTargetId = "cargo.home.inspect:C:/Users/dev/.cargo";
let scanCount = 0;
let pendingCancellation: { scanId: string; resolve: (result: DesktopScanResult) => void } | undefined;
let analyzeCount = 0;
let pendingAnalyzeCancellation: { operationId: string; resolve: (result: DesktopAnalyzeResult) => void } | undefined;

function correlatedAnalyzeProgress(operationId: string): DesktopAnalyzeProgress {
  return { ...analyzeProgress, operation_id: operationId };
}

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
  analyzeStart: async (operationId, _root, onProgress) => {
    analyzeCount += 1;
    onProgress(correlatedAnalyzeProgress(operationId));
    if (analyzeCount === 1) {
      return new Promise((resolve) => { pendingAnalyzeCancellation = { operationId, resolve }; });
    }
    await new Promise((resolve) => setTimeout(resolve, 120));
    return { ...analyzeComplete, operation_id: operationId };
  },
  analyzeCancel: async (operationId) => {
    if (pendingAnalyzeCancellation?.operationId === operationId) {
      pendingAnalyzeCancellation.resolve({
        ...analyzeComplete,
        type: "canceled",
        operation_id: operationId,
        snapshot: { ...analyzeComplete.snapshot, completeness: "canceled" },
      });
      pendingAnalyzeCancellation = undefined;
    }
  },
  softwareInventoryStart: async (operationId) => ({ type: "completed", operation_id: operationId, inventory: softwareInventory }),
  softwarePreview: async (operationId, _inventory, selectedIds) => {
    if (selectedIds.join("\n") !== softwarePreview.plan.selected_ids.join("\n")) {
      throw { code: "software_stale_authority", message: "Controlled fixture selection differs" };
    }
    return { ...softwarePreview, operation_id: operationId };
  },
  softwareUninstall: async (operationId, plan, previewDigest, confirmed) => {
    if (!confirmed || previewDigest !== softwarePreview.preview.digest
      || plan.selected_ids.join("\n") !== softwarePreview.plan.selected_ids.join("\n")) {
      throw { code: "software_stale_authority", message: "Controlled fixture preview is stale" };
    }
    return { ...softwareExecution, operation_id: operationId };
  },
  softwareAudit: async (operationId) => ({ ...softwareAudit, operation_id: operationId }),
  softwareCancel: async () => undefined,
  optimizeListStart: async (operationId) => (
    optimizeCatalogue.type === "completed"
      ? { ...optimizeCatalogue, operation_id: operationId }
      : { type: "canceled", operation_id: operationId }
  ),
  optimizePreview: async (operationId, catalogueId) => {
    if (catalogueId.startsWith("guidance.")) throw optimizeRefusalGuidance;
    const preview = optimizePreviewById[catalogueId as keyof typeof optimizePreviewById];
    if (!preview) throw optimizeRefusalGuidance;
    return { ...preview, operation_id: operationId };
  },
  optimizeRun: async (operationId, plan, previewDigest, confirmed) => {
    const preview = optimizePreviewById[plan.operation_id as keyof typeof optimizePreviewById];
    if (!confirmed || !preview || previewDigest !== preview.preview.digest) throw optimizeRefusalStale;
    if (plan.operation_id === "dns.flush") return { ...optimizeExecutionDns, operation_id: operationId };
    return {
      operation_id: operationId,
      report: {
        ...optimizeExecutionSettings.report,
        outcomes: [{
          ...optimizeExecutionSettings.report.outcomes[0],
          catalogue_id: plan.operation_id,
        }],
      },
    };
  },
  optimizeAudit: async (operationId) => ({ ...optimizeAudit, operation_id: operationId }),
  optimizeCancel: async () => undefined,
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
