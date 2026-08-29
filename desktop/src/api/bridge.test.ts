import { beforeEach, describe, expect, it, vi } from "vitest";
import scanJson from "./fixtures/scan-report.json";
import progressJson from "./fixtures/scan-progress.json";
import dryRunJson from "./fixtures/dry-run-outcome.json";
import executionJson from "./fixtures/execution-report.json";

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
import { decodeScanReport } from "./contract";

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

  it("rejects a terminal result correlated to another scan", async () => {
    mocks.invoke.mockResolvedValue({ type: "canceled", scan_id: "stale" });
    await expect(tauriBridge.scanStart("current", { include_projects: true, include_global: true, roots: ["."] }, vi.fn(), vi.fn()))
      .rejects.toThrow("does not match the active scan");
  });
});
