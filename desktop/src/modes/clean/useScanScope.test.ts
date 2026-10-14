import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { fixtureBridge } from "../../api/fixture-bridge";
import type { CleanScanPreferencesSnapshot } from "../../api/types.gen";
import { useScanScope } from "./useScanScope";

const snapshot = (roots: string[]): CleanScanPreferencesSnapshot => ({ version: 1, roots, suggestions: [] });
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

describe("explicit scan scope", () => {
  it("ignores delayed startup roots after a user edit and invalidates before validation settles", async () => {
    const startup = deferred<CleanScanPreferencesSnapshot>();
    const validation = deferred<CleanScanPreferencesSnapshot>();
    const invalidate = vi.fn();
    const bridge = { ...fixtureBridge, cleanScanPreferencesGet: () => startup.promise, cleanScanRootsValidate: () => validation.promise };
    const { result } = renderHook(() => useScanScope(bridge, invalidate));
    let editing!: Promise<boolean>;
    act(() => { editing = result.current.changeRoots(["D:/new"]); });
    expect(invalidate).toHaveBeenCalledOnce();
    await act(async () => { startup.resolve(snapshot(["C:/stale"])); });
    expect(result.current.options.roots).toEqual([]);
    await act(async () => { validation.resolve(snapshot(["D:/new"])); await editing; });
    expect(result.current.options.roots).toEqual(["D:/new"]);
  });

  it("serializes writes, fences overlapping calls and leaves the final saved roots visible", async () => {
    const saving = deferred<CleanScanPreferencesSnapshot>();
    const writes = vi.fn().mockReturnValueOnce(saving.promise).mockImplementation(async (roots: string[]) => snapshot(roots));
    const bridge = { ...fixtureBridge, cleanScanPreferencesSet: writes };
    const { result } = renderHook(() => useScanScope(bridge, vi.fn()));
    await waitFor(() => expect(result.current.loading).toBe(false));
    let first!: Promise<boolean>;
    act(() => { first = result.current.changeRoots(["C:/one"]); });
    await waitFor(() => expect(writes).toHaveBeenCalledOnce());
    await act(async () => { expect(await result.current.changeRoots(["C:/overlap"])).toBe(false); });
    expect(writes).toHaveBeenCalledOnce();
    await act(async () => { saving.resolve(snapshot(["C:/one"])); await first; });
    await act(async () => { await result.current.changeRoots(["D:/final"]); });
    expect(writes).toHaveBeenLastCalledWith(["D:/final"]);
    expect(result.current.options.roots).toEqual(["D:/final"]);
  });

  it("keeps validated session roots editable when saved bytes are corrupt", async () => {
    const bridge = { ...fixtureBridge, cleanScanPreferencesGet: vi.fn().mockRejectedValue(new Error("future version")), cleanScanPreferencesSet: vi.fn().mockRejectedValue(new Error("future version")) };
    const { result } = renderHook(() => useScanScope(bridge, vi.fn()));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.sessionOnly).toBe(true);
    await act(async () => { expect(await result.current.changeRoots(["D:/valid"])).toBe(true); });
    expect(result.current.options.roots).toEqual(["D:/valid"]);
    expect(result.current.error).toBeInstanceOf(Error);
    await act(async () => { await result.current.changeRoots([]); });
    expect(result.current.options.roots).toEqual([]);
  });

  it("rejects invalid additions without changing the prior roots", async () => {
    const bridge = { ...fixtureBridge, cleanScanRootsValidate: vi.fn().mockRejectedValue(new Error("relative root")), cleanScanPreferencesSet: vi.fn() };
    const { result } = renderHook(() => useScanScope(bridge, vi.fn()));
    await waitFor(() => expect(result.current.loading).toBe(false));
    await act(async () => { expect(await result.current.changeRoots(["."])).toBe(false); });
    expect(result.current.options.roots).toEqual(["C:/work/app"]);
    expect(bridge.cleanScanPreferencesSet).not.toHaveBeenCalled();
  });
});
