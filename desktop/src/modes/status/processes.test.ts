import { describe, expect, it } from "vitest";
import snapshotJson from "../../api/fixtures/status/snapshot.json";
import { decodeStatusSnapshot } from "../../api/contract";
import type { ProcessV1, StatusSnapshotV1 } from "../../api/types.gen";
import {
  MAX_PINNED_PROCESSES,
  nextSort,
  processTableRows,
  reconcilePins,
  sortProcesses,
  togglePin,
  type PinnedProcess,
} from "./processes";
import { initialStatusState, processRows, statusReducer } from "./state";

const base = decodeStatusSnapshot(snapshotJson);

function row(
  pid: number,
  name: string,
  cpu: number,
  memory: number,
): ProcessV1 {
  return {
    pid,
    name,
    cpu_basis_points_of_one_logical_core: cpu,
    private_bytes: memory,
    read_bytes_per_second: 0,
    write_bytes_per_second: 0,
  };
}

const rows = [
  row(30, "beta.exe", 500, 4096),
  row(10, "alpha.exe", 2000, 1024),
  row(20, "gamma.exe", 1000, 8192),
];

function snapshotWith(
  items: readonly ProcessV1[],
  complete: boolean,
): StatusSnapshotV1 {
  return {
    ...base,
    processes: complete
      ? {
          state: "available",
          sampled_at_unix_ms: 1,
          age_ms: 0,
          value: {
            items: [...items],
            enumerated_count: items.length,
            returned_count: items.length,
            requested_limit: 15,
            enumeration_ceiling: 4096,
            detail_budget_ms: 150,
            truncated_by_limit: false,
            budget_exhausted: false,
          },
        }
      : {
          state: "partial",
          sampled_at_unix_ms: 1,
          age_ms: 0,
          value: {
            items: [...items],
            enumerated_count: 400,
            returned_count: items.length,
            requested_limit: 15,
            enumeration_ceiling: 4096,
            detail_budget_ms: 150,
            truncated_by_limit: true,
            budget_exhausted: false,
          },
          reason_codes: ["process_limit"],
        },
  } as StatusSnapshotV1;
}

describe("process sort", () => {
  it("sorts by CPU, private bytes, and name in both directions", () => {
    const pids = (items: readonly ProcessV1[]) => items.map((item) => item.pid);
    expect(pids(sortProcesses(rows, "cpu", "descending"))).toEqual([
      10, 20, 30,
    ]);
    expect(pids(sortProcesses(rows, "cpu", "ascending"))).toEqual([30, 20, 10]);
    expect(pids(sortProcesses(rows, "memory", "descending"))).toEqual([
      20, 30, 10,
    ]);
    expect(pids(sortProcesses(rows, "memory", "ascending"))).toEqual([
      10, 30, 20,
    ]);
    expect(pids(sortProcesses(rows, "name", "ascending"))).toEqual([
      10, 30, 20,
    ]);
    expect(pids(sortProcesses(rows, "name", "descending"))).toEqual([
      20, 30, 10,
    ]);
  });

  it("reverses the active column and starts other columns at their default", () => {
    expect(nextSort("cpu", "descending", "cpu")).toEqual({
      sort: "cpu",
      direction: "ascending",
    });
    expect(nextSort("cpu", "ascending", "name")).toEqual({
      sort: "name",
      direction: "ascending",
    });
    expect(nextSort("name", "ascending", "memory")).toEqual({
      sort: "memory",
      direction: "descending",
    });
  });
});

describe("process pins", () => {
  it("pins at most five processes and unpins on a second toggle", () => {
    let pins: readonly PinnedProcess[] = [];
    for (let pid = 1; pid <= MAX_PINNED_PROCESSES + 1; pid += 1) {
      pins = togglePin(pins, pid, `p${pid}.exe`);
    }
    expect(pins).toHaveLength(MAX_PINNED_PROCESSES);
    expect(pins.some((pin) => pin.pid === MAX_PINNED_PROCESSES + 1)).toBe(
      false,
    );
    expect(togglePin(pins, 1, "p1.exe")).toHaveLength(MAX_PINNED_PROCESSES - 1);
  });

  it("keeps pinned rows at the top in the current sort order", () => {
    const pins = togglePin(togglePin([], 30, "beta.exe"), 20, "gamma.exe");
    const table = processTableRows(rows, pins, "cpu", "descending");
    expect(
      table.map((entry) =>
        entry.kind === "process"
          ? [entry.process.pid, entry.pinned]
          : [entry.pid, entry.status],
      ),
    ).toEqual([
      [20, true],
      [30, true],
      [10, false],
    ]);
  });

  it("shows an exited pin once and then removes it", () => {
    const pins = togglePin([], 30, "beta.exe");
    const exited = reconcilePins(pins, snapshotWith([rows[1]], true));
    expect(exited).toEqual([{ pid: 30, name: "beta.exe", status: "exited" }]);
    expect(processTableRows([rows[1]], exited, "cpu", "descending")[0]).toEqual(
      { kind: "absent", pid: 30, name: "beta.exe", status: "exited" },
    );
    expect(reconcilePins(exited, snapshotWith([rows[1]], true))).toEqual([]);
  });

  it("marks a pin exited when another process reuses the PID", () => {
    const pins = togglePin([], 30, "beta.exe");
    expect(
      reconcilePins(pins, snapshotWith([row(30, "other.exe", 0, 0)], false)),
    ).toEqual([{ pid: 30, name: "beta.exe", status: "exited" }]);
  });

  it("does not claim an exit when the row set is truncated", () => {
    const pins = togglePin([], 30, "beta.exe");
    const unsampled = reconcilePins(pins, snapshotWith([rows[1]], false));
    expect(unsampled).toEqual([
      { pid: 30, name: "beta.exe", status: "unsampled" },
    ]);
    expect(reconcilePins(unsampled, snapshotWith(rows, false))).toEqual([
      { pid: 30, name: "beta.exe", status: "live" },
    ]);
  });

  it("routes pin toggles and sort changes through the reducer", () => {
    let state = statusReducer(initialStatusState, {
      type: "operation_requested",
      operationId: "snap",
      operation: "snapshot",
    });
    state = statusReducer(state, {
      type: "snapshot_completed",
      operationId: "snap",
      snapshot: snapshotWith(rows, true),
    });
    state = statusReducer(state, {
      type: "pin_toggled",
      pid: 30,
      name: "beta.exe",
    });
    state = statusReducer(state, { type: "sort_changed", sort: "cpu" });
    expect(state.processSortDirection).toBe("ascending");
    expect(
      processRows(state).map((entry) =>
        entry.kind === "process" ? entry.process.pid : entry.pid,
      ),
    ).toEqual([30, 20, 10]);
  });
});
