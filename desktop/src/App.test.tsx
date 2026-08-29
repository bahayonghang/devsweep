import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import scanJson from "./api/fixtures/scan-report.json";
import dryRunJson from "./api/fixtures/dry-run-outcome.json";
import twoTargetDryRunJson from "./api/fixtures/dry-run-outcome-two-targets.json";
import executionJson from "./api/fixtures/execution-report.json";
import progressJson from "./api/fixtures/scan-progress.json";
import { decodeDesktopScanProgress, decodeDryRunOutcome, decodeExecutionReport, decodeScanReport } from "./api/contract";
import type { DesktopBridge } from "./api/bridge";
import type { DesktopScanProgress, DesktopScanResult, ExecutionReport } from "./api/types.gen";
import { App } from "./App";

const scanReport = decodeScanReport(scanJson);
const dryRun = decodeDryRunOutcome(dryRunJson);
const twoTargetDryRun = decodeDryRunOutcome(twoTargetDryRunJson);
const execution = decodeExecutionReport(executionJson);
const progress = decodeDesktopScanProgress(progressJson);

function fakeBridge(overrides: Partial<DesktopBridge> = {}): DesktopBridge {
  return {
    scanStart: vi.fn().mockImplementation(async (scanId: string, _options, onProgress: (progress: DesktopScanProgress) => void) => {
      onProgress({ ...progress, scan_id: scanId });
      return { type: "completed", scan_id: scanId, report: scanReport };
    }),
    scanCancel: vi.fn().mockResolvedValue(undefined),
    planDryRun: vi.fn().mockResolvedValue(dryRun),
    planExecute: vi.fn().mockResolvedValue(execution),
    ...overrides,
  };
}

describe("desktop workflow", () => {
  it("keeps a canceled preview read-only and replaces it on rescan", async () => {
    let finishFirst: (result: DesktopScanResult) => void = () => undefined;
    let invocation = 0;
    const scanStart = vi.fn().mockImplementation((scanId: string, _options, onProgress: (progress: DesktopScanProgress) => void) => {
      invocation += 1;
      onProgress({ ...progress, scan_id: scanId });
      if (invocation === 1) return new Promise<DesktopScanResult>((resolve) => { finishFirst = resolve; });
      return Promise.resolve<DesktopScanResult>({ type: "completed", scan_id: scanId, report: scanReport });
    });
    const scanCancel = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ scanStart, scanCancel })} />);

    await user.click(screen.getByRole("button", { name: "Scan" }));
    expect(screen.getByText("Discovered so far")).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "Scan progress" })).not.toHaveAttribute("aria-valuenow");
    expect(screen.queryByRole("checkbox", { name: /Select node.node_modules/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Review dry run" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cancel scan" }));
    expect(scanCancel).toHaveBeenCalledWith(expect.any(String));
    expect(screen.getByText("Cancel requested; finishing the current safe boundary")).toBeInTheDocument();
    const firstScanId = scanStart.mock.calls[0][0] as string;
    act(() => finishFirst({ type: "canceled", scan_id: firstScanId }));
    expect(await screen.findByText("Scan canceled. These partial observations remain read-only.")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Projects 1" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Scan" }));
    expect(await screen.findByText("Cleanup targets")).toBeInTheDocument();
    expect(scanStart).toHaveBeenCalledTimes(2);
  });

  it("distinguishes an active empty scan from a completed empty report", async () => {
    let finish: (result: DesktopScanResult) => void = () => undefined;
    const scanStart = vi.fn().mockImplementation(() => new Promise<DesktopScanResult>((resolve) => { finish = resolve; }));
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ scanStart })} />);

    await user.click(screen.getByRole("button", { name: "Scan" }));
    expect(screen.getByText("Discovering cleanup targets…")).toBeInTheDocument();
    expect(screen.queryByText("No scan results")).not.toBeInTheDocument();
    const scanId = scanStart.mock.calls[0][0] as string;
    act(() => finish({ type: "completed", scan_id: scanId, report: { ...scanReport, plan: { ...scanReport.plan, targets: [] } } }));
    expect(await screen.findByText("Scan complete; no cleanup targets found")).toBeInTheDocument();
  });

  it("offers an explicit return to the previous completed report after a stopped rescan", async () => {
    let finishRescan: (result: DesktopScanResult) => void = () => undefined;
    let invocation = 0;
    const scanStart = vi.fn().mockImplementation((scanId: string, _options, onProgress: (value: DesktopScanProgress) => void) => {
      invocation += 1;
      if (invocation === 1) return Promise.resolve<DesktopScanResult>({ type: "completed", scan_id: scanId, report: scanReport });
      onProgress({ ...progress, scan_id: scanId });
      return new Promise<DesktopScanResult>((resolve) => { finishRescan = resolve; });
    });
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ scanStart })} />);

    await user.click(screen.getByRole("button", { name: "Scan" }));
    await screen.findByText("Cleanup targets");
    await user.click(screen.getByRole("button", { name: "Scan" }));
    const rescanId = scanStart.mock.calls[1][0] as string;
    act(() => finishRescan({ type: "canceled", scan_id: rescanId }));

    const returnButton = await screen.findByRole("button", { name: "Return to previous report" });
    expect(screen.queryByRole("button", { name: "Review dry run" })).not.toBeInTheDocument();
    await user.click(returnButton);
    expect(await screen.findByText("Cleanup targets")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Review dry run" })).toBeEnabled();
  });

  it("requests cancellation on a malformed progress payload and waits for the terminal result", async () => {
    let finish: (result: DesktopScanResult) => void = () => undefined;
    const scanCancel = vi.fn().mockResolvedValue(undefined);
    const scanStart = vi.fn().mockImplementation((scanId: string, _options, onProgress: (value: DesktopScanProgress) => void, onProgressError: (error: unknown) => void) => {
      onProgress({ ...progress, scan_id: scanId });
      onProgressError(new Error("Malformed scan progress"));
      return new Promise<DesktopScanResult>((resolve) => { finish = resolve; });
    });
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ scanStart, scanCancel })} />);

    await user.click(screen.getByRole("button", { name: "Scan" }));
    await waitFor(() => expect(scanCancel).toHaveBeenCalledOnce());
    expect(screen.getByRole("button", { name: "Canceling…" })).toBeDisabled();
    expect(screen.getByRole("alert")).toHaveTextContent("Malformed scan progress");
    const scanId = scanStart.mock.calls[0][0] as string;
    act(() => finish({ type: "canceled", scan_id: scanId }));
    expect(await screen.findByText("Scan stopped after an error. These partial observations remain read-only.")).toBeInTheDocument();
  });

  it("groups cumulative preview rows by scope without announcing the table", async () => {
    let finish: (result: DesktopScanResult) => void = () => undefined;
    const scanStart = vi.fn().mockImplementation((scanId: string, _options, onProgress: (value: DesktopScanProgress) => void) => {
      const projectTarget = progress.preview!.targets[0];
      const globalTarget = {
        ...projectTarget,
        id: "fixture-global",
        scope: { type: "global" as const },
        kind: "package_cache" as const,
        path: "C:/fixture/global-cache",
        estimated_bytes: 8192,
      };
      onProgress({
        scan_id: scanId,
        sequence: 2,
        phase: "global",
        message: "Scanning controlled global providers",
        preview: {
          targets: [globalTarget, projectTarget],
          totals: { target_count: 2, verified_bytes: 12288, partial_lower_bound_bytes: 0, unknown_target_count: 0 },
        },
      });
      onProgress({ ...progress, scan_id: scanId, sequence: 1, message: "Stale project message" });
      return new Promise<DesktopScanResult>((resolve) => { finish = resolve; });
    });
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ scanStart })} />);

    await user.click(screen.getByRole("button", { name: "Scan" }));
    expect(screen.getByRole("heading", { name: "Projects 1" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Global caches 1" })).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Scanning controlled global providers");
    expect(screen.queryByText("Stale project message")).not.toBeInTheDocument();
    expect(screen.getAllByRole("table")).toHaveLength(2);
    expect(screen.getAllByRole("table").every((table) => !table.hasAttribute("aria-live"))).toBe(true);
    const scanId = scanStart.mock.calls[0][0] as string;
    act(() => finish({ type: "canceled", scan_id: scanId }));
  });

  it("invalidates a dry run after selection changes, then confirms and reports execution", async () => {
    const planDryRun = vi.fn().mockResolvedValueOnce(dryRun).mockResolvedValueOnce(twoTargetDryRun);
    const planExecute = vi.fn().mockResolvedValue(execution);
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ planDryRun, planExecute })} />);

    await user.click(screen.getByRole("button", { name: "Scan" }));
    expect(await screen.findByText("Cleanup targets")).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: /C:\/Users\/dev\/\.cargo/ })).toBeDisabled();
    expect(screen.getByRole<HTMLInputElement>("checkbox", { name: "Select all executable targets" }).indeterminate).toBe(true);
    await user.click(screen.getByRole("button", { name: "Review dry run" }));
    expect(await screen.findByText("Dry-run preview")).toBeInTheDocument();
    expect(screen.getByText("1 selected · 1 succeeded · 0 failed · 0 skipped")).toBeInTheDocument();
    expect(screen.getAllByText("500 MB")).toHaveLength(2);

    await user.click(screen.getByRole("button", { name: "Change selection" }));
    await user.click(screen.getByRole("checkbox", { name: /npm.cache.clean/ }));
    expect(screen.queryByText("Dry-run preview")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Review dry run" }));
    expect(await screen.findByText("Dry-run preview")).toBeInTheDocument();
    expect(planDryRun).toHaveBeenCalledTimes(2);
    expect(screen.getByText("2 selected · 2 succeeded · 0 failed · 0 skipped")).toBeInTheDocument();
    expect(screen.getByText("500 MB + at least 128 MB")).toBeInTheDocument();
    expect(screen.getAllByText(twoTargetDryRun.digest)).toHaveLength(2);
    expect(twoTargetDryRun.digest).not.toBe(dryRun.digest);

    await user.click(screen.getByRole("button", { name: "Continue to confirmation" }));
    expect(screen.getByRole("dialog")).toHaveTextContent(twoTargetDryRun.digest);
    expect(screen.getByText("Irreversible command selected.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Execute cleanup" }));
    expect(await screen.findByText("Cleanup report")).toBeInTheDocument();
    expect(screen.getByText(/capacity becomes available after trash is emptied/)).toBeInTheDocument();
    expect(planExecute).toHaveBeenCalledWith(scanReport.plan, expect.arrayContaining(["cargo.target:C:/work/app/target", "npm.cache.clean:global"]), twoTargetDryRun.digest);
  });

  it("disables conflicting scan controls during dry run and execution", async () => {
    const oneTargetExecution: ExecutionReport = { ...dryRun.report, dry_run: false, audit_log: "C:/fixture-audit.jsonl" };
    let resolveDryRun: (value: typeof dryRun) => void = () => undefined;
    let resolveExecution: (value: ExecutionReport) => void = () => undefined;
    const planDryRun = vi.fn().mockReturnValue(new Promise<typeof dryRun>((resolve) => { resolveDryRun = resolve; }));
    const planExecute = vi.fn().mockReturnValue(new Promise<ExecutionReport>((resolve) => { resolveExecution = resolve; }));
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ planDryRun, planExecute })} />);

    await user.click(screen.getByRole("button", { name: "Scan" }));
    await screen.findByText("Cleanup targets");
    await user.click(screen.getByRole("button", { name: "Review dry run" }));
    expect(screen.getByRole("button", { name: "Scan" })).toBeDisabled();
    await act(async () => resolveDryRun(dryRun));

    await user.click(screen.getByRole("button", { name: "Continue to confirmation" }));
    await user.click(screen.getByRole("button", { name: "Execute cleanup" }));
    expect(screen.getByRole("button", { name: "Scan" })).toBeDisabled();
    await act(async () => resolveExecution(oneTargetExecution));
    expect(await screen.findByText("Cleanup report")).toBeInTheDocument();
  });

  it.each([
    [{ code: "scan_already_running" } as const, "already running"],
    [{ code: "stale_confirmation", expected_digest: "old", actual_digest: "new" } as const, "run the dry run again"],
    [{ code: "unknown_target", target_id: "gone" } as const, "no longer available"],
  ])("renders recovery copy for $0.code", async (error, message) => {
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ scanStart: vi.fn().mockRejectedValue(error) })} />);
    await user.click(screen.getByRole("button", { name: "Scan" }));
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent(message));
  });
});
