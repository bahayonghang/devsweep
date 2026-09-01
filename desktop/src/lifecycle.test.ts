import { describe, expect, it, vi } from "vitest";
import {
  decodeNativeFaultMode,
  DesktopLifecycleController,
  ShellRouteCoordinatorAdapter,
} from "./lifecycle";
import { OperationCoordinator } from "./state/operation-coordinator";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => { resolve = resolvePromise; });
  return { promise, resolve };
}

describe("desktop lifecycle", () => {
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
