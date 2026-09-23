import type { ProcessV1, StatusSnapshotV1 } from "../../api/types.gen";

export type ProcessSort = "cpu" | "memory" | "name" | "pid";
export type SortDirection = "ascending" | "descending";
export const MAX_PINNED_PROCESSES = 5;

/**
 * One pinned process. `exited` shows once and is removed on the next
 * snapshot. `unsampled` means the PID is absent from a truncated row set, so
 * DevSweep cannot tell whether it exited.
 */
export interface PinnedProcess {
  readonly pid: number;
  readonly name: string;
  readonly status: "live" | "exited" | "unsampled";
}

export type ProcessTableRow =
  | {
      readonly kind: "process";
      readonly process: ProcessV1;
      readonly pinned: boolean;
    }
  | {
      readonly kind: "absent";
      readonly pid: number;
      readonly name: string;
      readonly status: "exited" | "unsampled";
    };

export function defaultDirection(sort: ProcessSort): SortDirection {
  return sort === "cpu" || sort === "memory" ? "descending" : "ascending";
}

/** Clicking the active column reverses it; another column starts at its default. */
export function nextSort(
  current: ProcessSort,
  direction: SortDirection,
  requested: ProcessSort,
): { readonly sort: ProcessSort; readonly direction: SortDirection } {
  if (current === requested) {
    return {
      sort: requested,
      direction: direction === "ascending" ? "descending" : "ascending",
    };
  }
  return { sort: requested, direction: defaultDirection(requested) };
}

export function compareProcesses(sort: ProcessSort, direction: SortDirection) {
  return (left: ProcessV1, right: ProcessV1): number => {
    const base =
      sort === "cpu"
        ? left.cpu_basis_points_of_one_logical_core -
          right.cpu_basis_points_of_one_logical_core
        : sort === "memory"
          ? left.private_bytes - right.private_bytes
          : sort === "name"
            ? left.name.localeCompare(right.name)
            : left.pid - right.pid;
    const ordered = direction === "ascending" ? base : -base;
    return ordered || left.pid - right.pid;
  };
}

export function sortProcesses(
  items: readonly ProcessV1[],
  sort: ProcessSort,
  direction: SortDirection,
): ProcessV1[] {
  return [...items].sort(compareProcesses(sort, direction));
}

/** Pin a process, or unpin it when it is already pinned. The limit is 5. */
export function togglePin(
  pins: readonly PinnedProcess[],
  pid: number,
  name: string,
): readonly PinnedProcess[] {
  if (pins.some((pin) => pin.pid === pid))
    return pins.filter((pin) => pin.pid !== pid);
  if (pins.length >= MAX_PINNED_PROCESSES) return pins;
  return [...pins, { pid, name, status: "live" }];
}

/**
 * Reconcile pins with a new snapshot. A PID that is absent from a complete
 * row set, or reused by another process name, is `exited`. An `exited` pin is
 * removed on the next snapshot.
 */
export function reconcilePins(
  pins: readonly PinnedProcess[],
  snapshot: StatusSnapshotV1,
): readonly PinnedProcess[] {
  const processes = snapshot.processes;
  const group =
    (processes.state === "available" || processes.state === "partial") &&
    processes.value
      ? processes.value
      : null;
  if (!group) return pins.filter((pin) => pin.status !== "exited");
  const complete =
    processes.state === "available" &&
    !group.truncated_by_limit &&
    !group.budget_exhausted;
  return pins.flatMap((pin): PinnedProcess[] => {
    if (pin.status === "exited") return [];
    if (
      group.items.some((item) => item.pid === pin.pid && item.name === pin.name)
    ) {
      return [{ ...pin, status: "live" }];
    }
    const reused = group.items.some((item) => item.pid === pin.pid);
    return [{ ...pin, status: complete || reused ? "exited" : "unsampled" }];
  });
}

/** Pinned rows first (sorted, then absent pins), then the other rows sorted. */
export function processTableRows(
  items: readonly ProcessV1[],
  pins: readonly PinnedProcess[],
  sort: ProcessSort,
  direction: SortDirection,
): readonly ProcessTableRow[] {
  const isPinned = (process: ProcessV1) =>
    pins.some(
      (pin) =>
        pin.pid === process.pid &&
        pin.name === process.name &&
        pin.status === "live",
    );
  const sorted = sortProcesses(items, sort, direction);
  const pinned = sorted
    .filter(isPinned)
    .map((process): ProcessTableRow => ({
      kind: "process",
      process,
      pinned: true,
    }));
  const absent = pins.flatMap((pin): ProcessTableRow[] =>
    pin.status === "live"
      ? []
      : [{ kind: "absent", pid: pin.pid, name: pin.name, status: pin.status }],
  );
  const rest = sorted
    .filter((process) => !isPinned(process))
    .map((process): ProcessTableRow => ({
      kind: "process",
      process,
      pinned: false,
    }));
  return [...pinned, ...absent, ...rest];
}
