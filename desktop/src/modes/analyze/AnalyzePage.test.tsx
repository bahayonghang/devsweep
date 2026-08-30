import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import snapshotFixture from "../../api/fixtures/analyze/snapshot.json";
import progressFixture from "../../api/fixtures/analyze/progress.json";
import { decodeAnalyzeSnapshot, decodeDesktopAnalyzeProgress } from "../../api/contract";
import type { DesktopBridge } from "../../api/bridge";
import type { DesktopAnalyzeResult } from "../../api/types.gen";
import { OperationCoordinator } from "../../state/operation-coordinator";
import { AnalyzePage } from "./AnalyzePage";

const analyzeStyles = readFileSync(resolve(process.cwd(), "src/modes/analyze/styles.css"), "utf8");

const snapshot = decodeAnalyzeSnapshot(snapshotFixture);
const progress = decodeDesktopAnalyzeProgress(progressFixture);

function bridge(): DesktopBridge {
  return {
    analyzeStart: vi.fn().mockImplementation(async (operationId, _root, onProgress) => {
      onProgress({ ...progress, operation_id: operationId });
      return { type: "completed", operation_id: operationId, snapshot };
    }),
    analyzeCancel: vi.fn().mockResolvedValue(undefined),
    scanStart: vi.fn().mockRejectedValue(new Error("unused")),
    scanCancel: vi.fn().mockResolvedValue(undefined),
    planDryRun: vi.fn().mockRejectedValue(new Error("unused")),
    planExecute: vi.fn().mockRejectedValue(new Error("unused")),
  };
}

describe("AnalyzePage", () => {
  it("renders bilingual empty/complete states with canonical list and supplemental labelled rectangles", async () => {
    const user = userEvent.setup();
    render(<AnalyzePage bridge={bridge()} coordinator={new OperationCoordinator()} locale="en" />);
    expect(screen.getByRole("heading", { name: "No analysis snapshot" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Analyze path" }));
    await waitFor(() => expect(screen.getByText(/Analysis complete/)).toBeInTheDocument());

    expect(screen.getByRole("listbox", { name: "Accessible directory list" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /a\.bin/ })).toBeInTheDocument();
    expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
    expect(screen.queryByText(/delete|cleanup/i)).not.toBeInTheDocument();
  });

  it("links a treemap focus to the paged list and supports keyboard up", async () => {
    const user = userEvent.setup();
    const nested = {
      ...snapshot,
      nodes: [
        { ...snapshot.nodes[0], immediate_count: 1 },
        { ...snapshot.nodes[1], kind: "directory" as const, name: "folder", immediate_count: 1, recursive_count: 1 },
        { ...snapshot.nodes[1], id: 2, parent_id: 1, name: "nested.bin" },
      ],
    };
    const custom = bridge();
    custom.analyzeStart = vi.fn().mockImplementation(async (operationId) => ({ type: "completed", operation_id: operationId, snapshot: nested }));
    render(<AnalyzePage bridge={custom} coordinator={new OperationCoordinator()} locale="zh-CN" />);
    await user.click(screen.getByRole("button", { name: "分析路径" }));
    const tile = await screen.findByRole("button", { name: /folder/ });
    tile.focus();
    fireEvent.keyDown(tile, { key: "Enter" });
    expect(await screen.findByRole("option", { name: /nested\.bin/ })).toBeInTheDocument();
    expect(screen.getByRole("listbox")).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("listbox"), { key: "Backspace" });
    expect(await screen.findByRole("option", { name: /folder/ })).toBeInTheDocument();
    expect(screen.getByRole("listbox")).toHaveFocus();
  });

  it("restores the focused directory on its canonical 200-row parent page", async () => {
    const user = userEvent.setup();
    const children = Array.from({ length: 450 }, (_, index) => ({
      ...snapshot.nodes[1],
      id: index + 1,
      parent_id: 0,
      kind: "directory" as const,
      name: `dir-${String(index + 1).padStart(3, "0")}`,
      bytes: 450 - index,
      immediate_count: 0,
    }));
    const wide = {
      ...snapshot,
      nodes: [{ ...snapshot.nodes[0], immediate_count: children.length, recursive_count: children.length }, ...children],
    };
    const custom = bridge();
    custom.analyzeStart = vi.fn().mockImplementation(async (operationId) => ({ type: "completed", operation_id: operationId, snapshot: wide }));
    render(<AnalyzePage bridge={custom} coordinator={new OperationCoordinator()} locale="en" />);
    await user.click(screen.getByRole("button", { name: "Analyze path" }));
    const next = await screen.findByRole("button", { name: "Next page" });
    await user.click(next);
    await user.click(next);
    const list = screen.getByRole("listbox");
    await user.selectOptions(list, "401");
    fireEvent.keyDown(list, { key: "Enter" });
    fireEvent.keyDown(screen.getByRole("listbox"), { key: "Backspace" });

    expect(await screen.findByRole("option", { name: /dir-401/ })).toBeInTheDocument();
    expect(screen.getByText(/Page 3 of 3/)).toBeInTheDocument();
    expect(screen.getByRole("listbox")).toHaveFocus();
  });

  it.each([
    {
      locale: "en" as const,
      empty: "No analysis snapshot",
      start: "Analyze path",
      cancel: "Cancel analysis",
      loading: /Analyzing/,
      canceling: /Cancel requested/,
      canceled: /Analysis canceled/,
      partial: /budget bound/,
      error: /Analysis failed/,
      complete: /Analysis complete/,
    },
    {
      locale: "zh-CN" as const,
      empty: "尚无分析快照",
      start: "分析路径",
      cancel: "取消分析",
      loading: /正在分析/,
      canceling: /已请求取消/,
      canceled: /分析已取消/,
      partial: /预算上限/,
      error: /分析失败/,
      complete: /分析完成/,
    },
  ])("covers empty/loading/canceling/canceled/partial/error/complete copy in $locale", async (copy) => {
    const user = userEvent.setup();
    let call = 0;
    let firstOperationId = "";
    let finishCanceled: ((value: DesktopAnalyzeResult) => void) | undefined;
    let releaseCancel: (() => void) | undefined;
    const custom = bridge();
    custom.analyzeStart = vi.fn().mockImplementation((operationId: string) => {
      call += 1;
      if (call === 1) {
        firstOperationId = operationId;
        return new Promise((resolve) => {
          finishCanceled = resolve;
        });
      }
      if (call === 2) {
        return Promise.resolve({
          type: "completed",
          operation_id: operationId,
          snapshot: { ...snapshot, completeness: "partial_budget" },
        });
      }
      if (call === 3) return Promise.reject(new Error("fixture failure"));
      return Promise.resolve({ type: "completed", operation_id: operationId, snapshot });
    });
    custom.analyzeCancel = vi.fn().mockImplementation(() => new Promise<void>((resolve) => {
      releaseCancel = resolve;
    }));

    render(<AnalyzePage bridge={custom} coordinator={new OperationCoordinator()} locale={copy.locale} />);
    expect(screen.getByRole("heading", { name: copy.empty })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: copy.start }));
    expect(await screen.findByRole("status")).toHaveTextContent(copy.loading);
    await user.click(screen.getByRole("button", { name: copy.cancel }));
    expect(screen.getByRole("status")).toHaveTextContent(copy.canceling);
    finishCanceled?.({
      type: "canceled",
      operation_id: firstOperationId,
      snapshot: { ...snapshot, completeness: "canceled" },
    });
    releaseCancel?.();
    await waitFor(() => expect(screen.getByText(copy.canceled)).toBeInTheDocument());

    await user.click(screen.getByRole("button", { name: copy.start }));
    await waitFor(() => expect(screen.getByText(copy.partial)).toBeInTheDocument());
    await user.click(screen.getByRole("button", { name: copy.start }));
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent(copy.error));
    await user.click(screen.getByRole("button", { name: copy.start }));
    await waitFor(() => expect(screen.getByText(copy.complete)).toBeInTheDocument());
  });

  it("keeps text contrast, keyboard focus, reduced motion, and forced-colors contracts explicit", () => {
    expect(analyzeStyles).toContain(".analyze-tile.evidence-incomplete { fill: #8f671f; }");
    expect(contrastRatio("#8f671f", "#ffffff")).toBeGreaterThanOrEqual(4.5);
    expect(analyzeStyles).toContain(".analyze-tile:focus-visible");
    expect(analyzeStyles).toContain("@media (prefers-reduced-motion: reduce)");
    expect(analyzeStyles).toContain("@media (forced-colors: active)");
  });
});

function contrastRatio(foreground: string, background: string): number {
  const luminance = (color: string) => {
    const channels = color.slice(1).match(/.{2}/g)!.map((channel) => Number.parseInt(channel, 16) / 255);
    const [red, green, blue] = channels.map((channel) => channel <= 0.03928
      ? channel / 12.92
      : ((channel + 0.055) / 1.055) ** 2.4);
    return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
  };
  const foregroundLuminance = luminance(foreground);
  const backgroundLuminance = luminance(background);
  return (Math.max(foregroundLuminance, backgroundLuminance) + 0.05)
    / (Math.min(foregroundLuminance, backgroundLuminance) + 0.05);
}
