import { describe, expect, it, vi } from "vitest";
import { OperationCoordinator, type CoordinatedOperation } from "../state/operation-coordinator";

function deferred<T = void>() {
  let resolve: (value: T) => void = () => undefined;
  let reject: (reason?: unknown) => void = () => undefined;
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}

function operation<T>(
  id: string,
  start: CoordinatedOperation<T>["start"],
  cancel: CoordinatedOperation<T>["cancel"] = vi.fn(),
): CoordinatedOperation<T> {
  return { kind: "clean.scan", id, start, cancel };
}

describe("OperationCoordinator", () => {
  it("cancels and joins the previous operation before invoking the next service", async () => {
    const first = deferred();
    const second = deferred();
    const events: string[] = [];
    const coordinator = new OperationCoordinator();
    const firstLease = await coordinator.start(operation(
      "first",
      () => { events.push("start-first"); return first.promise; },
      () => { events.push("cancel-first"); },
    ));
    expect(firstLease).not.toBeNull();

    const replacement = coordinator.start(operation(
      "second",
      () => { events.push("start-second"); return second.promise; },
      () => { events.push("cancel-second"); },
    ));
    await vi.waitFor(() => expect(events).toEqual(["start-first", "cancel-first"]));
    first.resolve();
    const secondLease = await replacement;
    expect(events).toEqual(["start-first", "cancel-first", "start-second"]);
    expect(coordinator.activeIdentity()).toEqual({ kind: "clean.scan", id: "second" });

    second.resolve();
    expect(await secondLease?.complete()).toBe(true);
    expect(await secondLease?.complete()).toBe(false);
  });

  it("rejects stale completion without disturbing the current operation", async () => {
    const current = deferred();
    const coordinator = new OperationCoordinator();
    const lease = await coordinator.start(operation("current", () => current.promise, () => current.resolve()));
    expect(await coordinator.complete("stale")).toBe(false);
    expect(coordinator.activeIdentity()).toEqual({ kind: "clean.scan", id: "current" });
    current.resolve();
    expect(await lease?.complete()).toBe(true);
  });

  it("close cancels, joins, and refuses late work without invoking or canceling it", async () => {
    const current = deferred();
    const cancelCurrent = vi.fn(() => current.resolve());
    const coordinator = new OperationCoordinator();
    await coordinator.start(operation("current", () => current.promise, cancelCurrent));
    await coordinator.close();
    expect(cancelCurrent).toHaveBeenCalledOnce();
    expect(coordinator.activeIdentity()).toBeNull();

    const startLate = vi.fn(() => Promise.resolve());
    const cancelLate = vi.fn();
    expect(await coordinator.start(operation("late", startLate, cancelLate))).toBeNull();
    expect(startLate).not.toHaveBeenCalled();
    expect(cancelLate).not.toHaveBeenCalled();
  });

  it("joins after cancellation failure and never invokes the refused replacement", async () => {
    const current = deferred();
    const joined = vi.fn();
    const startReplacement = vi.fn(() => Promise.resolve());
    const coordinator = new OperationCoordinator();
    await coordinator.start(operation(
      "current",
      () => current.promise.then(joined),
      () => { current.resolve(); throw new Error("cancel failed"); },
    ));
    await expect(coordinator.start(operation("replacement", startReplacement))).rejects.toThrow("cancel failed");
    expect(joined).toHaveBeenCalledOnce();
    expect(startReplacement).not.toHaveBeenCalled();
    expect(coordinator.activeIdentity()).toBeNull();
  });

  it("contains synchronous and asynchronous start failures without an active leak", async () => {
    const coordinator = new OperationCoordinator();
    await expect(coordinator.start(operation("sync", () => { throw new Error("sync start"); })))
      .rejects.toThrow("sync start");
    expect(coordinator.activeIdentity()).toBeNull();

    const rejected = await coordinator.start(operation("async", () => Promise.reject(new Error("async start"))));
    await expect(rejected?.result).rejects.toThrow("async start");
    expect(await rejected?.complete()).toBe(true);
    expect(coordinator.activeIdentity()).toBeNull();
  });

  it("serializes rapid replacement races in arrival order", async () => {
    const coordinator = new OperationCoordinator();
    const first = deferred();
    const second = deferred();
    const third = deferred();
    const events: string[] = [];
    await coordinator.start(operation(
      "first",
      () => { events.push("start-first"); return first.promise; },
      () => { events.push("cancel-first"); first.resolve(); },
    ));
    const replaceSecond = coordinator.start(operation(
      "second",
      () => { events.push("start-second"); return second.promise; },
      () => { events.push("cancel-second"); second.resolve(); },
    ));
    const replaceThird = coordinator.start(operation(
      "third",
      () => { events.push("start-third"); return third.promise; },
      () => { events.push("cancel-third"); third.resolve(); },
    ));
    await Promise.all([replaceSecond, replaceThird]);
    expect(events).toEqual([
      "start-first", "cancel-first", "start-second", "cancel-second", "start-third",
    ]);
    expect(coordinator.activeIdentity()?.id).toBe("third");
    third.resolve();
    await coordinator.complete("third");
  });
});
