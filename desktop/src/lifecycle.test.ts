import { describe, expect, it, vi } from "vitest";
import {
  decodeNativeFaultMode,
  DesktopLifecycleController,
  ShellRouteCoordinatorAdapter,
  observeWindowPresentation,
  type WindowPresentationBridge,
} from "./lifecycle";
import { OperationCoordinator } from "./state/operation-coordinator";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => { resolve = resolvePromise; });
  return { promise, resolve };
}

describe("desktop lifecycle", () => {
  it("uses visible presentation in browser fixtures without native calls", () => {
    const handler = vi.fn();
    const disconnect = observeWindowPresentation(handler, null);
    expect(handler).toHaveBeenCalledExactlyOnceWith(true);
    disconnect();
  });

  it("subscribes before reading visibility and rejects a stale initial read", async () => {
    const initial = deferred<unknown>();
    const handler = vi.fn();
    let emit: (value: unknown) => void = () => undefined;
    const order: string[] = [];
    const release = vi.fn();
    const bridge: WindowPresentationBridge = {
      async listen(listener) { order.push("listen"); emit = listener; return release; },
      visible() { order.push("read"); return initial.promise; },
    };
    const disconnect = observeWindowPresentation(handler, bridge);
    expect(handler).toHaveBeenLastCalledWith(false);
    await vi.waitFor(() => expect(order).toEqual(["listen", "read"]));
    emit(false);
    initial.resolve(true);
    await Promise.resolve();
    expect(handler).not.toHaveBeenCalledWith(true);
    emit(true);
    expect(handler).toHaveBeenLastCalledWith(true);
    emit({ visible: true });
    expect(handler).toHaveBeenLastCalledWith(false);
    disconnect();
    expect(release).toHaveBeenCalledOnce();
    const calls = handler.mock.calls.length;
    emit(true);
    expect(handler).toHaveBeenCalledTimes(calls);
  });

  it("releases a late subscription and suppresses reads after disposal", async () => {
    const subscription = deferred<() => void>();
    const release = vi.fn();
    const visible = vi.fn();
    const handler = vi.fn();
    const disconnect = observeWindowPresentation(handler, { listen: () => subscription.promise, visible });
    disconnect();
    subscription.resolve(release);
    await vi.waitFor(() => expect(release).toHaveBeenCalledOnce());
    expect(visible).not.toHaveBeenCalled();
    expect(handler).toHaveBeenCalledExactlyOnceWith(false);
  });

  it("ignores a visibility read completing after disposal and suspends on bridge failures", async () => {
    const pending = deferred<unknown>();
    const handler = vi.fn();
    const visible = vi.fn(() => pending.promise);
    const disconnect = observeWindowPresentation(handler, { async listen() { return () => undefined; }, visible });
    await vi.waitFor(() => expect(visible).toHaveBeenCalledOnce());
    disconnect();
    pending.resolve(true);
    await Promise.resolve();
    expect(handler).toHaveBeenCalledExactlyOnceWith(false);
    for (const bridge of [
      { async listen() { throw new Error("listen failed"); }, visible: vi.fn() },
      { async listen() { return () => undefined; }, async visible() { throw new Error("read failed"); } },
    ]) {
      const failed = vi.fn();
      const release = observeWindowPresentation(failed, bridge);
      await vi.waitFor(() => expect(failed).toHaveBeenCalledTimes(2));
      expect(failed.mock.calls.every(([value]) => value === false)).toBe(true);
      release();
    }
  });

  it("decodes only the closed debug fault modes", () => {
    expect(decodeNativeFaultMode("disabled")).toBe("disabled");
    expect(decodeNativeFaultMode("route_cancel_once")).toBe("route_cancel_once");
    expect(() => decodeNativeFaultMode("route_cancel_always")).toThrow("unsupported native fault mode");
    expect(() => decodeNativeFaultMode({ mode: "route_cancel_once" })).toThrow("unsupported native fault mode");
  });

  it("shares one drain across duplicate close requests", async () => {
    const close = vi.fn().mockResolvedValue(undefined);
    const controller = new DesktopLifecycleController({ close });

    await Promise.all([controller.requestClose(), controller.requestClose(), controller.drain()]);

    expect(close).toHaveBeenCalledOnce();
  });

  it("awaits join and consumes cancellation failure before releasing native close", async () => {
    const coordinator = new OperationCoordinator();
    const joined = deferred<void>();
    await coordinator.start({
      kind: "status",
      id: "active-close",
      start: () => joined.promise,
      cancel: () => { throw new Error("cancel failed after request"); },
    });
    const controller = new DesktopLifecycleController(coordinator);

    const closing = controller.requestClose();
    await vi.waitFor(() => expect(coordinator.activeIdentity()).toBeNull());
    let released = false;
    void closing.then(() => { released = true; });
    expect(released).toBe(false);

    joined.resolve();
    await closing;
    expect(released).toBe(true);
    expect(coordinator.activeIdentity()).toBeNull();
  });

  it("keeps the debug route fault one-shot and otherwise delegates", async () => {
    const cancelAndJoin = vi.fn().mockResolvedValue(undefined);
    const adapter = new ShellRouteCoordinatorAdapter({ cancelAndJoin });

    adapter.setNativeFaultMode("disabled");
    await adapter.cancelAndJoin();
    adapter.setNativeFaultMode("route_cancel_once");
    await expect(adapter.cancelAndJoin()).rejects.toThrow("debug route cancellation fault");
    await adapter.cancelAndJoin();

    expect(cancelAndJoin).toHaveBeenCalledTimes(2);
  });
});
