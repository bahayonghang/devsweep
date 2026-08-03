import { beforeEach, describe, expect, it, vi } from "vitest";
import scanJson from "./fixtures/scan-report.json";
import dryRunJson from "./fixtures/dry-run-outcome.json";
import executionJson from "./fixtures/execution-report.json";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));

import { tauriBridge } from "./bridge";
import { decodeScanReport } from "./contract";

const plan = decodeScanReport(scanJson).plan;

describe("tauriBridge", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.listen.mockReset();
  });

  it("uses Tauri command names and camelCase argument names", async () => {
    const options = { include_projects: true, include_global: false, roots: ["."] };
    mocks.invoke.mockResolvedValueOnce(scanJson).mockResolvedValueOnce(dryRunJson).mockResolvedValueOnce(executionJson).mockResolvedValueOnce(undefined);

    await tauriBridge.scanStart(options);
    await tauriBridge.planDryRun(plan, ["cargo.target:C:/work/app/target"]);
    await tauriBridge.planExecute(plan, ["cargo.target:C:/work/app/target", "npm.cache.clean:global"], executionJson.confirmation_digest);
    await tauriBridge.scanCancel();

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "scan_start", { options });
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "plan_dry_run", { plan, selectedIds: ["cargo.target:C:/work/app/target"] });
    expect(mocks.invoke).toHaveBeenNthCalledWith(3, "plan_execute", { plan, selectedIds: ["cargo.target:C:/work/app/target", "npm.cache.clean:global"], digest: executionJson.confirmation_digest });
    expect(mocks.invoke).toHaveBeenNthCalledWith(4, "scan_cancel");
  });

  it("subscribes to the exact event, returns unlisten, and rejects authority-bearing progress", async () => {
    const unlisten = vi.fn();
    let callback: ((event: { payload: unknown }) => void) | undefined;
    mocks.listen.mockImplementation(async (_event, handler) => { callback = handler; return unlisten; });
    const onProgress = vi.fn();
    const onError = vi.fn();

    const dispose = await tauriBridge.onScanProgress(onProgress, onError);
    callback?.({ payload: { phase: "projects", message: "Scanning", partial: null } });
    callback?.({ payload: { phase: "projects", message: "Unsafe", partial: { action: { program: "cmd.exe" } } } });
    dispose();

    expect(mocks.listen).toHaveBeenCalledWith("scan://progress", expect.any(Function));
    expect(onProgress).toHaveBeenCalledOnce();
    expect(onError).toHaveBeenCalledOnce();
    expect(unlisten).toHaveBeenCalledOnce();
  });
});
