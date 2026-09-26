import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StrictMode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { fixtureBridge } from "./api/fixture-bridge";
import { createFixtureWindowBridge, type DesktopLifecycleBridge } from "./lifecycle";
import { OperationCoordinator } from "./state/operation-coordinator";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => { resolve = resolvePromise; });
  return { promise, resolve };
}

function settings() {
  return {
    async load() { return { language: "en" as const }; },
    async save() { return { language: "en" as const }; },
  };
}

describe("custom controls and App lifecycle", () => {
  beforeEach(() => window.history.replaceState(null, "", "#/clean"));

  it.each(["cooperative", "wait-only"] as const)("joins %s work before native destruction after a custom close", async (kind) => {
    const coordinator = new OperationCoordinator();
    const work = deferred<void>();
    const cancel = vi.fn();
    await coordinator.start({ kind: kind === "wait-only" ? "clean.execute" : "clean.scan", id: kind, start: () => work.promise, cancel });
    const closeCoordinator = vi.spyOn(coordinator, "close");
    const native = createFixtureWindowBridge();
    const destroy = vi.fn();
    const lifecycle: DesktopLifecycleBridge = {
      ...native,
      onCloseRequested: (handler) => native.onCloseRequested(async () => { await handler(); destroy(); }),
    };
    const view = render(<StrictMode><App bridge={fixtureBridge} presentationSettings={settings()} lifecycle={lifecycle} windowControls={native} coordinator={coordinator} /></StrictMode>);
    const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: "Minimize" }));
    expect(cancel).not.toHaveBeenCalled();
    expect(coordinator.activeIdentity()?.id).toBe(kind);
    await user.click(screen.getByRole("button", { name: "Close" }));
    await waitFor(() => expect(cancel).toHaveBeenCalledOnce());
    expect(destroy).not.toHaveBeenCalled();
    const repeatedNativeClose = native.close();
    await act(async () => { work.resolve(); await repeatedNativeClose; });
    expect(destroy).toHaveBeenCalled();
    expect(closeCoordinator).toHaveBeenCalledOnce();
    expect(coordinator.activeIdentity()).toBeNull();
    view.unmount();
    await act(async () => { await Promise.resolve(); });
    expect(closeCoordinator).toHaveBeenCalledOnce();
  });

  it("retains window controls when the presentation store is unavailable", async () => {
    const native = createFixtureWindowBridge();
    const minimize = vi.spyOn(native, "minimize");
    render(<App bridge={fixtureBridge} presentationSettings={{ ...settings(), load: vi.fn().mockRejectedValue(new Error("unavailable")) }} lifecycle={native} windowControls={native} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("presentation");
    await userEvent.setup().click(screen.getByRole("button", { name: "Minimize / 最小化" }));
    expect(minimize).toHaveBeenCalledOnce();
    expect(screen.getByRole("button", { name: "Close / 关闭" })).toBeEnabled();
  });
});
