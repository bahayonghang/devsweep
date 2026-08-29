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
});
