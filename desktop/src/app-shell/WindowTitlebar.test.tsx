import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StrictMode } from "react";
import { describe, expect, it, vi } from "vitest";
import { createFixtureWindowBridge, type WindowControlBridge } from "../lifecycle";
import { WindowTitlebar } from "./WindowTitlebar";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => { resolve = resolvePromise; });
  return { promise, resolve };
}

function bridgeWith(overrides: Partial<WindowControlBridge> = {}) {
  return { ...createFixtureWindowBridge(), ...overrides };
}

describe("custom window titlebar", () => {
  it.each([
    ["en", "Minimize", "Maximize", "Restore", "Close"],
    ["zh-CN", "最小化", "最大化", "还原", "关闭"],
  ] as const)("labels and operates all controls in %s", async (locale, minimizeLabel, maximizeLabel, restoreLabel, closeLabel) => {
    const minimize = vi.fn().mockResolvedValue(undefined);
    const close = vi.fn().mockResolvedValue(undefined);
    const startDragging = vi.fn().mockResolvedValue(undefined);
    const bridge = bridgeWith({ minimize, close, startDragging });
    const user = userEvent.setup();
    render(<WindowTitlebar bridge={bridge} locale={locale} />);
    await waitFor(() => expect(screen.getByRole("button", { name: maximizeLabel })).toBeEnabled());
    await user.tab();
    const minimizeButton = screen.getByRole("button", { name: minimizeLabel });
    expect(minimizeButton).toHaveFocus();
    expect(minimizeButton).toHaveAttribute("title", minimizeLabel);
    await user.keyboard("{Enter}");
    expect(minimize).toHaveBeenCalledOnce();
    await user.click(screen.getByRole("button", { name: maximizeLabel }));
    expect(await screen.findByRole("button", { name: restoreLabel })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: restoreLabel }));
    expect(await screen.findByRole("button", { name: maximizeLabel })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: closeLabel }));
    expect(close).toHaveBeenCalledOnce();
    expect(startDragging).not.toHaveBeenCalled();
  });

  it("drags and double-clicks only the explicit blank title region", async () => {
    const startDragging = vi.fn().mockResolvedValue(undefined);
    const bridge = bridgeWith({ startDragging });
    const view = render(<WindowTitlebar bridge={bridge} locale="en" />);
    await waitFor(() => expect(screen.getByRole("button", { name: "Maximize" })).toBeEnabled());
    const region = view.container.querySelector(".window-drag-region")!;
    fireEvent.mouseDown(region, { button: 2, detail: 1 });
    expect(startDragging).not.toHaveBeenCalled();
    await act(async () => { fireEvent.mouseDown(region, { button: 0, detail: 1 }); });
    expect(startDragging).toHaveBeenCalledOnce();
    await act(async () => { fireEvent.mouseDown(region, { button: 0, detail: 2 }); });
    expect(screen.getByRole("button", { name: "Restore" })).toBeEnabled();
    fireEvent.mouseDown(view.container.querySelector("svg")!, { button: 0, detail: 2 });
    expect(startDragging).toHaveBeenCalledOnce();
  });

  it("holds native state until confirmation and prevents duplicate toggles", async () => {
    const action = deferred<void>();
    const toggleMaximize = vi.fn(() => action.promise);
    const isMaximized = vi.fn().mockResolvedValue(false);
    const bridge = bridgeWith({ toggleMaximize, isMaximized });
    render(<WindowTitlebar bridge={bridge} locale="en" />);
    const button = screen.getByRole("button", { name: "Maximize" });
    await waitFor(() => expect(button).toBeEnabled());
    fireEvent.click(button);
    fireEvent.click(button);
    expect(toggleMaximize).toHaveBeenCalledOnce();
    expect(button).toBeDisabled();
    expect(screen.queryByRole("button", { name: "Restore" })).not.toBeInTheDocument();
    isMaximized.mockResolvedValue(true);
    await act(async () => { action.resolve(); });
    expect(screen.getByRole("button", { name: "Restore" })).toBeEnabled();
  });

  it("shows a localized failure and retries a failed action", async () => {
    const minimize = vi.fn().mockRejectedValueOnce(new Error("native failure")).mockResolvedValue(undefined);
    render(<WindowTitlebar bridge={bridgeWith({ minimize })} locale="zh-CN" />);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "最小化" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("无法最小化窗口。");
    await user.click(screen.getByRole("button", { name: "重试窗口操作" }));
    expect(minimize).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("cleans late resize listeners during StrictMode replay and unmount", async () => {
    const first = deferred<() => void>();
    const second = deferred<() => void>();
    const firstUnlisten = vi.fn();
    const secondUnlisten = vi.fn();
    const onResized = vi.fn().mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    const view = render(<StrictMode><WindowTitlebar bridge={bridgeWith({ onResized })} locale="en" /></StrictMode>);
    expect(onResized).toHaveBeenCalledTimes(2);
    await act(async () => { second.resolve(secondUnlisten); first.resolve(firstUnlisten); });
    expect(firstUnlisten).toHaveBeenCalledOnce();
    expect(secondUnlisten).not.toHaveBeenCalled();
    view.unmount();
    expect(secondUnlisten).toHaveBeenCalledOnce();
  });

  it("keeps bilingual controls available before the language store loads", async () => {
    render(<WindowTitlebar bridge={bridgeWith()} locale={null} />);
    expect(screen.getByRole("button", { name: "Close / 关闭" })).toBeEnabled();
    await waitFor(() => expect(screen.getByRole("button", { name: "Maximize / 最大化" })).toBeEnabled());
  });
});
