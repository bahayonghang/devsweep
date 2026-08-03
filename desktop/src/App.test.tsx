import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import scanJson from "./api/fixtures/scan-report.json";
import dryRunJson from "./api/fixtures/dry-run-outcome.json";
import twoTargetDryRunJson from "./api/fixtures/dry-run-outcome-two-targets.json";
import executionJson from "./api/fixtures/execution-report.json";
import { decodeDryRunOutcome, decodeExecutionReport, decodeScanReport } from "./api/contract";
import type { DesktopBridge } from "./api/bridge";
import type { ExecutionReport, ScanProgress } from "./api/types.gen";
import { App } from "./App";

const scanReport = decodeScanReport(scanJson);
const dryRun = decodeDryRunOutcome(dryRunJson);
const twoTargetDryRun = decodeDryRunOutcome(twoTargetDryRunJson);
const execution = decodeExecutionReport(executionJson);

function fakeBridge(overrides: Partial<DesktopBridge> = {}): DesktopBridge {
  return {
    scanStart: vi.fn().mockResolvedValue(scanReport),
    scanCancel: vi.fn().mockResolvedValue(undefined),
    planDryRun: vi.fn().mockResolvedValue(dryRun),
    planExecute: vi.fn().mockResolvedValue(execution),
    onScanProgress: vi.fn().mockImplementation(async (handler: (progress: ScanProgress) => void) => {
      handler({ phase: "projects", message: "Scanning project roots", partial: null });
      return () => undefined;
    }),
    ...overrides,
  };
}

describe("desktop workflow", () => {
  it("cancels a scan, reports the error, and rescans", async () => {
    let rejectFirst: (reason: unknown) => void = () => undefined;
    const first = new Promise<never>((_, reject) => { rejectFirst = reject; });
    const scanStart = vi.fn().mockReturnValueOnce(first).mockResolvedValueOnce(scanReport);
    const scanCancel = vi.fn().mockResolvedValue(undefined);
    let progressHandler: ((progress: ScanProgress) => void) | undefined;
    const onScanProgress = vi.fn().mockImplementation(async (handler: (progress: ScanProgress) => void) => {
      progressHandler = handler;
      return () => undefined;
    });
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ scanStart, scanCancel, onScanProgress })} />);

    await user.click(screen.getByRole("button", { name: "Scan" }));
    act(() => progressHandler?.({ phase: "projects", message: "Scanning project roots", partial: null }));
    expect(screen.getByText(/Scanning project roots/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cancel scan" }));
    expect(scanCancel).toHaveBeenCalledOnce();
    rejectFirst({ code: "scan_failed", message: "Scan canceled" });
    expect(await screen.findByRole("alert")).toHaveTextContent("Scan canceled");
    await user.click(screen.getByRole("button", { name: "Scan" }));
    expect(await screen.findByText("Cleanup targets")).toBeInTheDocument();
    expect(scanStart).toHaveBeenCalledTimes(2);
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
