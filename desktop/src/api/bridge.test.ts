import { beforeEach, describe, expect, it, vi } from "vitest";
import scanJson from "./fixtures/scan-report.json";
import progressJson from "./fixtures/scan-progress.json";
import dryRunJson from "./fixtures/dry-run-outcome.json";
import executionJson from "./fixtures/execution-report.json";
import analyzeCompleteJson from "./fixtures/analyze/complete.json";
import analyzeProgressJson from "./fixtures/analyze/progress.json";
import cleanMovedTotalsJson from "./fixtures/history/clean-moved-totals.json";
import softwareInventoryJson from "./fixtures/software/inventory.json";
import softwareUpdatesJson from "./fixtures/software/updates-available.json";
import softwareStartupJson from "./fixtures/software/startup-list.json";
import softwareStartupToggleJson from "./fixtures/software/startup-toggle.json";
import softwareLeftoversPlanJson from "./fixtures/software/leftovers-planned.json";
import softwareLeftoversReportJson from "./fixtures/software/leftovers-report.json";

const mocks = vi.hoisted(() => {
  const channels: Array<{ onmessage: (value: unknown) => void }> = [];
  class Channel<T> {
    onmessage: (value: T) => void;
    constructor(onmessage: (value: T) => void) {
      this.onmessage = onmessage;
      channels.push(this as { onmessage: (value: unknown) => void });
    }
  }
  return { invoke: vi.fn(), channels, Channel };
});
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke, Channel: mocks.Channel }));

import { tauriBridge } from "./bridge";
import { decodeDesktopSoftwareLeftoversPreviewResult, decodeScanReport, decodeSoftwareInventory } from "./contract";

const plan = decodeScanReport(scanJson).plan;

describe("tauriBridge", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.channels.length = 0;
  });

  it("uses command-scoped channels and camelCase argument names", async () => {
    const options = { include_projects: true, include_global: false, roots: ["."] };
    mocks.invoke
      .mockResolvedValueOnce({ type: "completed", scan_id: "scan-1", report: scanJson })
      .mockResolvedValueOnce(dryRunJson)
      .mockResolvedValueOnce(executionJson)
      .mockResolvedValueOnce(undefined);
    const onProgress = vi.fn();

    const scanPromise = tauriBridge.scanStart("scan-1", options, onProgress, vi.fn());
    expect(mocks.channels).toHaveLength(1);
    mocks.channels[0].onmessage(progressJson);
    await scanPromise;
    await tauriBridge.planDryRun(plan, ["cargo.target:C:/work/app/target"]);
    await tauriBridge.planExecute(plan, ["cargo.target:C:/work/app/target", "npm.cache.clean:global"], executionJson.confirmation_digest);
    await tauriBridge.scanCancel("scan-1");

    expect(onProgress).toHaveBeenCalledWith(expect.objectContaining({ scan_id: "fixture-scan-1", sequence: 1 }));
    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "scan_start", { scanId: "scan-1", options, onProgress: expect.any(mocks.Channel) });
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "plan_dry_run", { plan, selectedIds: ["cargo.target:C:/work/app/target"] });
    expect(mocks.invoke).toHaveBeenNthCalledWith(3, "plan_execute", { plan, selectedIds: ["cargo.target:C:/work/app/target", "npm.cache.clean:global"], digest: executionJson.confirmation_digest });
    expect(mocks.invoke).toHaveBeenNthCalledWith(4, "scan_cancel", { scanId: "scan-1" });
  });

  it("reads the cumulative Clean total through a closed decoder", async () => {
    mocks.invoke.mockResolvedValueOnce(cleanMovedTotalsJson);
    await expect(tauriBridge.historyCleanTotals()).resolves.toEqual(cleanMovedTotalsJson);
    expect(mocks.invoke).toHaveBeenCalledWith("history_clean_totals", {});

    for (const payload of [
      { ...cleanMovedTotalsJson, path: "C:/secret" },
      { known_bytes: 1, unknown_records: 0 },
      { ...cleanMovedTotalsJson, known_bytes: -1 },
      { ...cleanMovedTotalsJson, known_bytes: 2 ** 53 },
      { ...cleanMovedTotalsJson, unknown_records: 1, lower_bound: false },
      { ...cleanMovedTotalsJson, lower_bound: "yes" },
    ]) {
      mocks.invoke.mockResolvedValueOnce(payload);
      await expect(tauriBridge.historyCleanTotals()).rejects.toThrow();
    }
  });

  it("routes malformed channel payloads to the invocation-local error callback", async () => {
    mocks.invoke.mockResolvedValue({ type: "canceled", scan_id: "scan-2" });
    const onProgress = vi.fn();
    const onError = vi.fn();
    const promise = tauriBridge.scanStart("scan-2", { include_projects: true, include_global: true, roots: ["."] }, onProgress, onError);

    mocks.channels[0].onmessage(progressJson);
    mocks.channels[0].onmessage({ ...progressJson, partial: { action: { program: "cmd.exe" } } });
    await promise;

    expect(onProgress).toHaveBeenCalledOnce();
    expect(onError).toHaveBeenCalledOnce();
  });

  it("uses a command-scoped Analyze channel and correlates terminal identity", async () => {
    mocks.invoke
      .mockResolvedValueOnce({ ...analyzeCompleteJson, operation_id: "analyze-current" })
      .mockResolvedValueOnce(undefined);
    const onProgress = vi.fn();
    const promise = tauriBridge.analyzeStart("analyze-current", "C:/fixture", onProgress, vi.fn());
    mocks.channels[0].onmessage({ ...analyzeProgressJson, operation_id: "analyze-current" });
    await promise;
    await tauriBridge.analyzeCancel("analyze-current");

    expect(onProgress).toHaveBeenCalledWith(expect.objectContaining({ operation_id: "analyze-current", sequence: 1 }));
    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "analyze_start", { operationId: "analyze-current", root: "C:/fixture", onProgress: expect.any(mocks.Channel) });
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "analyze_cancel", { operationId: "analyze-current" });
  });

  it("sends Software support commands with camelCase arguments and correlates results", async () => {
    const inventory = decodeSoftwareInventory(softwareInventoryJson);
    const planned = decodeDesktopSoftwareLeftoversPreviewResult(softwareLeftoversPlanJson);
    if (planned.type !== "planned") throw new Error("fixture must be planned");
    const selection = {
      software_id: planned.plan.software_id,
      uninstall_operation_id: planned.plan.uninstall_operation_id,
      selected_candidate_ids: planned.plan.selected_candidate_ids,
    };
    mocks.invoke
      .mockResolvedValueOnce(softwareUpdatesJson)
      .mockResolvedValueOnce(softwareStartupJson)
      .mockResolvedValueOnce(softwareStartupToggleJson)
      .mockResolvedValueOnce(softwareLeftoversPlanJson)
      .mockResolvedValueOnce(softwareLeftoversReportJson);

    await tauriBridge.softwareUpdatesCheck("software-updates-1", null);
    await tauriBridge.softwareStartupList();
    await tauriBridge.softwareStartupSet(softwareStartupToggleJson.entry.id, false, true);
    await tauriBridge.softwareLeftoversPreview("software-leftovers-2", inventory, [selection.software_id], selection);
    await tauriBridge.softwareLeftoversExecute("software-leftovers-3", planned.plan, planned.preview.digest, true);

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "software_updates_check", { operationId: "software-updates-1", inventory: null });
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "software_startup_list", {});
    expect(mocks.invoke).toHaveBeenNthCalledWith(3, "software_startup_set", { entryId: softwareStartupToggleJson.entry.id, enabled: false, confirmed: true });
    expect(mocks.invoke).toHaveBeenNthCalledWith(4, "software_leftovers_preview", { operationId: "software-leftovers-2", inventory, selectedIds: [selection.software_id], selection });
    expect(mocks.invoke).toHaveBeenNthCalledWith(5, "software_leftovers_execute", { operationId: "software-leftovers-3", plan: planned.plan, previewDigest: planned.preview.digest, confirmed: true });

    mocks.invoke.mockResolvedValueOnce(softwareUpdatesJson);
    await expect(tauriBridge.softwareUpdatesCheck("other", null)).rejects.toThrow("does not match the active operation");
    mocks.invoke.mockResolvedValueOnce(softwareStartupToggleJson);
    await expect(tauriBridge.softwareStartupSet(softwareStartupToggleJson.entry.id, true, true)).rejects.toThrow("does not match the request");
  });

  it("rejects a terminal result correlated to another scan", async () => {
    mocks.invoke.mockResolvedValue({ type: "canceled", scan_id: "stale" });
    await expect(tauriBridge.scanStart("current", { include_projects: true, include_global: true, roots: ["."] }, vi.fn(), vi.fn()))
      .rejects.toThrow("does not match the active scan");
  });
});
