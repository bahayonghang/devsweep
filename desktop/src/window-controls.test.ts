import { describe, expect, it, vi } from "vitest";
import { createFixtureWindowBridge, tauriWindowControlBridge, WindowControlController } from "./lifecycle";

const nativeWindow = vi.hoisted(() => ({
  minimize: vi.fn().mockResolvedValue(undefined),
  toggleMaximize: vi.fn().mockResolvedValue(undefined),
  isMaximized: vi.fn().mockResolvedValue(true),
  startDragging: vi.fn().mockResolvedValue(undefined),
  close: vi.fn().mockResolvedValue(undefined),
  onResized: vi.fn().mockResolvedValue(() => undefined),
}));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => nativeWindow }));

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => { resolve = resolvePromise; });
  return { promise, resolve };
}

describe("window control adapter", () => {
  it("delegates the exact methods to the current native window", async () => {
    const listener = vi.fn();
    await tauriWindowControlBridge.minimize();
    await tauriWindowControlBridge.toggleMaximize();
    expect(await tauriWindowControlBridge.isMaximized()).toBe(true);
    await tauriWindowControlBridge.startDragging();
    await tauriWindowControlBridge.close();
    const unlisten = await tauriWindowControlBridge.onResized(listener);
    expect(unlisten).toBeTypeOf("function");
    for (const method of Object.values(nativeWindow)) expect(method).toHaveBeenCalledOnce();
    expect(nativeWindow.onResized).toHaveBeenCalledWith(listener);
  });

  it("rejects stale state reads and reads that finish after disposal", async () => {
    let resize: () => void = () => undefined;
    const isMaximized = vi.fn().mockResolvedValue(false);
    const unlisten = vi.fn();
    const controller = new WindowControlController({
      ...createFixtureWindowBridge(),
      isMaximized,
      async onResized(handler) { resize = handler; return unlisten; },
    });
    const disconnect = controller.connect();
    await vi.waitFor(() => expect(controller.getSnapshot().maximized).toBe(false));
    const older = deferred<boolean>();
    const newer = deferred<boolean>();
    isMaximized.mockReturnValueOnce(older.promise).mockReturnValueOnce(newer.promise);
    resize();
    resize();
    newer.resolve(true);
    await vi.waitFor(() => expect(controller.getSnapshot().maximized).toBe(true));
    older.resolve(false);
    await Promise.resolve();
    expect(controller.getSnapshot().maximized).toBe(true);
    const finalRead = deferred<boolean>();
    isMaximized.mockReturnValueOnce(finalRead.promise);
    resize();
    disconnect();
    const state = controller.getSnapshot();
    finalRead.resolve(false);
    await Promise.resolve();
    expect(controller.getSnapshot()).toBe(state);
    expect(unlisten).toHaveBeenCalledOnce();
    const reads = isMaximized.mock.calls.length;
    resize();
    expect(isMaximized).toHaveBeenCalledTimes(reads);
  });

  it("retains confirmed state on read failure and refreshes on retry", async () => {
    let resize: () => void = () => undefined;
    const isMaximized = vi.fn().mockResolvedValue(true);
    const controller = new WindowControlController({
      ...createFixtureWindowBridge(),
      isMaximized,
      async onResized(handler) { resize = handler; return () => undefined; },
    });
    const disconnect = controller.connect();
    await vi.waitFor(() => expect(controller.getSnapshot().maximized).toBe(true));
    isMaximized.mockRejectedValueOnce(new Error("read failure"));
    resize();
    await vi.waitFor(() => expect(controller.getSnapshot().error).toBe("state"));
    expect(controller.getSnapshot().maximized).toBe(true);
    isMaximized.mockResolvedValue(false);
    await controller.retry();
    expect(controller.getSnapshot()).toEqual({ maximized: false, pending: null, error: null });
    disconnect();
  });

  it("retries failed subscription without adding duplicate resize listeners", async () => {
    const unlisten = vi.fn();
    const onResized = vi.fn().mockRejectedValueOnce(new Error("listen failure")).mockResolvedValue(unlisten);
    const controller = new WindowControlController({ ...createFixtureWindowBridge(), onResized });
    const disconnect = controller.connect();
    await vi.waitFor(() => expect(controller.getSnapshot().error).toBe("subscription"));
    await Promise.all([controller.retry(), controller.retry()]);
    expect(onResized).toHaveBeenCalledTimes(2);
    expect(controller.getSnapshot().error).toBeNull();
    disconnect();
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it("repairs the resize subscription when a later action error is retried", async () => {
    const unlisten = vi.fn();
    const onResized = vi.fn().mockRejectedValueOnce(new Error("listen failure")).mockResolvedValue(unlisten);
    const minimize = vi.fn().mockRejectedValueOnce(new Error("action failure")).mockResolvedValue(undefined);
    const controller = new WindowControlController({ ...createFixtureWindowBridge(), onResized, minimize });
    const disconnect = controller.connect();
    await vi.waitFor(() => expect(controller.getSnapshot().error).toBe("subscription"));
    await controller.perform("minimize");
    expect(controller.getSnapshot().error).toBe("minimize");
    await controller.retry();
    expect(minimize).toHaveBeenCalledTimes(2);
    expect(onResized).toHaveBeenCalledTimes(2);
    expect(controller.getSnapshot().error).toBeNull();
    disconnect();
    expect(unlisten).toHaveBeenCalledOnce();
  });
});
