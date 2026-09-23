import type {
  CommandError,
  ProcessV1,
  StatusEventV1,
  StatusSnapshotV1,
} from "../../api/types.gen";
import {
  defaultDirection,
  nextSort,
  processTableRows,
  reconcilePins,
  sortProcesses,
  togglePin,
  type PinnedProcess,
  type ProcessSort,
  type ProcessTableRow,
  type SortDirection,
} from "./processes";

export type { PinnedProcess, ProcessSort, ProcessTableRow, SortDirection } from "./processes";

export const MAX_CHART_POINTS = 60;
export const INTERVAL_STEPS_MS = [1_000, 2_000, 5_000, 10_000, 30_000, 60_000] as const;
export const DEFAULT_INTERVAL_MS = 2_000;
export const DEFAULT_PROCESS_LIMIT = 15;

export type StatusPhase = "idle" | "snapshot" | "ready" | "live" | "canceling" | "failed";
export type StatusOperation = "snapshot" | "live";

export interface ChartPoint {
  readonly sequence: number;
  readonly sampledAtUnixMs: number | null;
  readonly cpuBp: number | null;
  readonly memoryUsedBytes: number | null;
  readonly rxBytesPerSecond: number | null;
  readonly txBytesPerSecond: number | null;
}

export interface StatusState {
  readonly status: StatusPhase;
  readonly operationId: string | null;
  readonly operation: StatusOperation | null;
  readonly streamId: string | null;
  readonly snapshot: StatusSnapshotV1 | null;
  readonly chart: readonly ChartPoint[];
  readonly processSort: ProcessSort;
  readonly processSortDirection: SortDirection;
  readonly pins: readonly PinnedProcess[];
  readonly intervalMs: number;
  readonly processLimit: number;
  readonly lastSequence: number | null;
  readonly skippedTotal: number;
  readonly error: CommandError | string | null;
}

export const initialStatusState: StatusState = {
  status: "idle",
  operationId: null,
  operation: null,
  streamId: null,
  snapshot: null,
  chart: [],
  processSort: "cpu",
  processSortDirection: defaultDirection("cpu"),
  pins: [],
  intervalMs: DEFAULT_INTERVAL_MS,
  processLimit: DEFAULT_PROCESS_LIMIT,
  lastSequence: null,
  skippedTotal: 0,
  error: null,
};

export type StatusAction =
  | { readonly type: "operation_requested"; readonly operationId: string; readonly operation: StatusOperation }
  | { readonly type: "cancel_requested"; readonly operationId: string }
  | { readonly type: "snapshot_completed"; readonly operationId: string; readonly snapshot: StatusSnapshotV1 }
  | { readonly type: "live_event"; readonly operationId: string; readonly event: StatusEventV1 }
  | { readonly type: "operation_canceled"; readonly operationId: string }
  | { readonly type: "operation_failed"; readonly operationId: string; readonly error: CommandError | string }
  | { readonly type: "interval_changed"; readonly intervalMs: number }
  | { readonly type: "sort_changed"; readonly sort: ProcessSort }
  | { readonly type: "pin_toggled"; readonly pid: number; readonly name: string }
  | { readonly type: "error_dismissed" }
  | { readonly type: "released" };

export function availableValue<T>(availability: { readonly state: string; readonly value?: T }): T | null {
  return (availability.state === "available" || availability.state === "partial") && availability.value !== undefined
    ? availability.value
    : null;
}

export function formatBasisPoints(points: number): string {
  return `${Math.floor(points / 100)}.${String(points % 100).padStart(2, "0")}`;
}

export function formatTenths(tenths: number): string {
  const sign = tenths < 0 ? "-" : "";
  const magnitude = Math.abs(tenths);
  return `${sign}${Math.floor(magnitude / 10)}.${magnitude % 10}`;
}

/** Used memory in basis points of total memory, or null without a value. */
export function memoryBasisPoints(snapshot: StatusSnapshotV1): number | null {
  const memory = availableValue(snapshot.memory);
  if (!memory || memory.total_bytes <= 0) return null;
  return Math.min(10_000, Math.floor((memory.used_bytes * 10_000) / memory.total_bytes));
}

/** The busiest GPU adapter, or null when the GPU probe has no value. */
export function peakGpuBasisPoints(snapshot: StatusSnapshotV1): number | null {
  const gpu = availableValue(snapshot.gpu);
  if (!gpu || gpu.adapters.length === 0) return null;
  return Math.max(...gpu.adapters.map((adapter) => adapter.utilization_basis_points));
}

/** The warmest thermal zone, or null when the thermal probe has no value. */
export function peakTemperatureTenths(snapshot: StatusSnapshotV1): number | null {
  const thermal = availableValue(snapshot.thermal);
  if (!thermal || thermal.zones.length === 0) return null;
  return Math.max(...thermal.zones.map((zone) => zone.temperature_tenths_celsius));
}

function gapPoint(sequence: number): ChartPoint {
  return {
    sequence,
    sampledAtUnixMs: null,
    cpuBp: null,
    memoryUsedBytes: null,
    rxBytesPerSecond: null,
    txBytesPerSecond: null,
  };
}

function pointFromSnapshot(sequence: number, snapshot: StatusSnapshotV1): ChartPoint {
  const network = availableValue(snapshot.network);
  return {
    sequence,
    sampledAtUnixMs: snapshot.sampled_at_unix_ms,
    cpuBp: availableValue(snapshot.cpu)?.system_utilization_basis_points ?? null,
    memoryUsedBytes: availableValue(snapshot.memory)?.used_bytes ?? null,
    rxBytesPerSecond: network
      ? network.interfaces.reduce((sum, item) => sum + item.rx_bytes_per_second, 0)
      : null,
    txBytesPerSecond: network
      ? network.interfaces.reduce((sum, item) => sum + item.tx_bytes_per_second, 0)
      : null,
  };
}

function pushChart(points: readonly ChartPoint[], point: ChartPoint): ChartPoint[] {
  const next = [...points, point];
  return next.length > MAX_CHART_POINTS ? next.slice(next.length - MAX_CHART_POINTS) : next;
}

function clampInterval(intervalMs: number): number {
  const rounded = Math.floor(intervalMs / 1_000) * 1_000;
  return Math.min(60_000, Math.max(1_000, rounded));
}

function finish(state: StatusState, status: StatusPhase): StatusState {
  return {
    ...state,
    status,
    operationId: null,
    operation: null,
    streamId: null,
    lastSequence: null,
  };
}

function failDecode(state: StatusState, reason: string): StatusState {
  return { ...state, status: "failed", error: reason };
}

function acceptSequence(state: StatusState, operationId: string, sequence: number): StatusState | null {
  if (state.streamId !== operationId) return failDecode(state, "live event is missing status_started");
  const expected = state.lastSequence === null ? null : state.lastSequence + 1;
  if (expected !== sequence) return failDecode(state, "skipped, duplicate, or nonmonotonic sequence");
  return { ...state, lastSequence: sequence };
}

function ingestLive(state: StatusState, event: StatusEventV1): StatusState {
  if (state.streamId !== null && state.streamId !== event.operation_id) return state;
  if (event.event === "status_started") {
    if (state.streamId !== null || state.lastSequence !== null || event.sequence !== 0) {
      return failDecode(state, "status_started sequence is not exact");
    }
    return {
      ...state,
      status: "live",
      streamId: event.operation_id,
      lastSequence: 0,
      intervalMs: event.data.interval_ms,
      processLimit: event.data.process_limit,
    };
  }
  const sequenced = acceptSequence(state, event.operation_id, event.sequence);
  if (sequenced === null || sequenced.status === "failed") return sequenced ?? state;
  if (event.event === "status_snapshot") {
    return {
      ...sequenced,
      snapshot: event.data,
      pins: reconcilePins(sequenced.pins, event.data),
      chart: pushChart(sequenced.chart, pointFromSnapshot(event.sequence, event.data)),
    };
  }
  if (event.event === "tick_skipped") {
    return {
      ...sequenced,
      skippedTotal: event.data.skipped_total,
      chart: pushChart(sequenced.chart, gapPoint(event.sequence)),
    };
  }
  const phase = event.data.reason === "producer_error" ? "failed" : sequenced.snapshot ? "ready" : "idle";
  return {
    ...finish(sequenced, phase),
    error: event.data.reason === "producer_error" ? event.data.error_code : sequenced.error,
  };
}

export function sortedProcesses(state: StatusState): readonly ProcessV1[] {
  const group = state.snapshot ? availableValue(state.snapshot.processes) : null;
  if (!group) return [];
  return sortProcesses(group.items, state.processSort, state.processSortDirection);
}

export function processRows(state: StatusState): readonly ProcessTableRow[] {
  const group = state.snapshot ? availableValue(state.snapshot.processes) : null;
  return processTableRows(group?.items ?? [], state.pins, state.processSort, state.processSortDirection);
}

export function statusReducer(state: StatusState, action: StatusAction): StatusState {
  switch (action.type) {
    case "operation_requested":
      if (state.operationId !== null && !(action.operation === "live" && state.operation === "live")) return state;
      return {
        ...state,
        status: action.operation === "live" ? "live" : "snapshot",
        operationId: action.operationId,
        operation: action.operation,
        streamId: action.operation === "live" ? null : state.streamId,
        lastSequence: action.operation === "live" ? null : state.lastSequence,
        chart: action.operation === "live" ? [] : state.chart,
        skippedTotal: action.operation === "live" ? 0 : state.skippedTotal,
        error: null,
      };
    case "cancel_requested":
      return state.operationId === action.operationId ? { ...state, status: "canceling" } : state;
    case "snapshot_completed":
      if (state.operationId !== action.operationId || state.operation !== "snapshot") return state;
      return {
        ...finish(state, "ready"),
        snapshot: action.snapshot,
        pins: reconcilePins(state.pins, action.snapshot),
        error: null,
      };
    case "live_event":
      if (state.operationId !== action.operationId || state.operation !== "live") return state;
      return ingestLive(state, action.event);
    case "operation_canceled":
      if (state.operationId !== action.operationId) return state;
      return finish(state, state.snapshot ? "ready" : "idle");
    case "operation_failed":
      if (state.operationId !== action.operationId) return state;
      return { ...finish(state, "failed"), error: action.error };
    case "interval_changed":
      return { ...state, intervalMs: clampInterval(action.intervalMs) };
    case "sort_changed": {
      const next = nextSort(state.processSort, state.processSortDirection, action.sort);
      return { ...state, processSort: next.sort, processSortDirection: next.direction };
    }
    case "pin_toggled":
      return { ...state, pins: togglePin(state.pins, action.pid, action.name) };
    case "error_dismissed":
      return { ...state, error: null };
    case "released":
      return initialStatusState;
  }
}
