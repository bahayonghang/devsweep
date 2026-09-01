import { act, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import progressJson from "../api/fixtures/scan-progress.json";
import { decodeDesktopScanProgress } from "../api/contract";
import type { ActiveScan } from "../state/app-state";
import { ScanPage } from "./ScanPage";

const progress = decodeDesktopScanProgress(progressJson);
const options = { include_projects: true, include_global: true, roots: ["."] };
const noOp = () => undefined;

function activeScan(message: string, sequence: number, phase: "projects" | "global" = "projects"): ActiveScan {
  const nextProgress = { ...progress, message, sequence, phase };
  return {
    scanId: nextProgress.scan_id,
    lastSequence: sequence,
    progress: nextProgress,
    preview: nextProgress.preview,
    cancelRequested: false,
  };
}

afterEach(() => vi.useRealTimers());

describe("ScanPage announcements", () => {
  it("throttles same-phase screen-reader updates while keeping visual copy current", () => {
    vi.useFakeTimers();
    const { rerender } = render(
      <ScanPage activeScan={activeScan("First target", 1)} busy={false} options={options} onOptions={noOp} onScan={noOp} onCancel={noOp} />,
    );
    expect(screen.getByRole("status")).toHaveTextContent("First target");

    rerender(<ScanPage activeScan={activeScan("Latest target", 2)} busy={false} options={options} onOptions={noOp} onScan={noOp} onCancel={noOp} />);
    expect(screen.getByText("Latest target")).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("First target");

    act(() => vi.advanceTimersByTime(1_000));
    expect(screen.getByRole("status")).toHaveTextContent("Latest target");

    rerender(<ScanPage activeScan={activeScan("Global phase", 3, "global")} busy={false} options={options} onOptions={noOp} onScan={noOp} onCancel={noOp} />);
    expect(screen.getByRole("status")).toHaveTextContent("Global caches. Global phase");
  });

  it("uses Chinese catalogue copy instead of English chrome", () => {
    render(
      <ScanPage locale="zh-CN" activeScan={null} busy={false} options={options} onOptions={noOp} onScan={noOp} onCancel={noOp} />,
    );
    expect(screen.getByRole("button", { name: "扫描" })).toBeInTheDocument();
    expect(screen.getByText("项目")).toBeInTheDocument();
    expect(screen.getByText("全局缓存")).toBeInTheDocument();
    expect(screen.getByText("就绪")).toBeInTheDocument();
    expect(screen.queryByText("Projects")).not.toBeInTheDocument();
    expect(screen.queryByText("Scan")).not.toBeInTheDocument();
    expect(screen.queryByText("Ready")).not.toBeInTheDocument();
    expect(screen.queryByText("没有扫描结果")).not.toBeInTheDocument();
    expect(document.querySelector(".sweep-body-hero")).toBeInTheDocument();
  });

  it("idle home is a sweep hero with visible scope and Scan, not a light empty heading", () => {
    render(
      <ScanPage activeScan={null} busy={false} options={options} onOptions={noOp} onScan={noOp} onCancel={noOp} />,
    );
    expect(screen.getByRole("button", { name: "Scan" })).toBeInTheDocument();
    expect(screen.getByText("Projects")).toBeInTheDocument();
    expect(screen.getByText("Global caches")).toBeInTheDocument();
    expect(screen.getByText("Ready")).toBeInTheDocument();
    expect(screen.queryByText("No scan results")).not.toBeInTheDocument();
    expect(document.querySelector(".sweep-body-hero")).toBeInTheDocument();
  });

  it("scanning shows indeterminate progress and the backend message without a percentage", () => {
    render(
      <ScanPage activeScan={activeScan("Scanning controlled project roots", 1)} busy={false} options={options} onOptions={noOp} onScan={noOp} onCancel={noOp} />,
    );
    expect(screen.getByRole("progressbar", { name: "Scan" })).not.toHaveAttribute("aria-valuenow");
    expect(screen.getByText("Scanning controlled project roots")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancel scan" })).toBeInTheDocument();
    expect(screen.queryByText(/%|percent/i)).not.toBeInTheDocument();
  });
});
