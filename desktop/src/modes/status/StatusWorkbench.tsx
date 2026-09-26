import { usePreferences } from "../../preferences/context";
import { useEffect, useLayoutEffect, useReducer, useRef, useState } from "react";
import type { DesktopBridge } from "../../api/bridge";
import { decodeCommandError } from "../../api/contract";
import type {
  CommandError,
  DesktopStatusLiveResult,
  DesktopStatusSnapshotResult,
  StatusEventV1,
  StatusSnapshotV1,
} from "../../api/types.gen";
import { AccessibleUserData } from "../../app-shell/AppShell";
import { DestinationGlyph, type GlyphName } from "../../app-shell/glyphs";
import { ErrorBanner } from "../../components/ErrorBanner";
import { formatBytes } from "../../components/format";
import { message, type MessageKey, type PresentationLanguageTag } from "../../i18n";
import { DetailView, Stage, StageResult } from "../../stage";
import { OperationCoordinator } from "../../state/operation-coordinator";
import {
  INTERVAL_STEPS_MS,
  availableValue,
  formatBasisPoints,
  formatTenths,
  initialStatusState,
  memoryBasisPoints,
  peakGpuBasisPoints,
  peakTemperatureTenths,
  processRows,
  statusReducer,
  type ChartPoint,
  type ProcessSort,
  type StatusOperation,
} from "./state";
import { MAX_PINNED_PROCESSES } from "./processes";
import "./styles.css";

interface StatusWorkbenchProps {
  readonly bridge: DesktopBridge;
  readonly coordinator: OperationCoordinator;
  readonly locale: PresentationLanguageTag;
}

let operationSequence = 0;
function createOperationId(kind: StatusOperation): string {
  operationSequence += 1;
  const randomId = globalThis.crypto?.randomUUID?.();
  return randomId ? `status-${kind}-${randomId}` : `status-${kind}-${Date.now()}-${operationSequence}`;
}

function commandError(error: unknown): CommandError {
  try { return decodeCommandError(error); }
  catch { return { code: "io", message: error instanceof Error ? error.message : "Unexpected Status IPC error" }; }
}

function taggedLine(locale: PresentationLanguageTag, group: string, availability: { readonly state: string; readonly reason_code?: string; readonly reason_codes?: readonly string[] }): string {
  if (availability.state === "partial") {
    return message(locale, "status.v1.group.partial", { group, reasons: (availability.reason_codes ?? []).join(",") });
  }
  if (availability.state === "unavailable") {
    return message(locale, "status.v1.group.unavailable", { group, reason: availability.reason_code ?? "" });
  }
  if (availability.state === "permission_denied") {
    return message(locale, "status.v1.group.permission_denied", { group, reason: availability.reason_code ?? "" });
  }
  if (availability.state === "unsupported") {
    return message(locale, "status.v1.group.unsupported", { group, reason: availability.reason_code ?? "" });
  }
  return group;
}

function cpuCard(locale: PresentationLanguageTag, snapshot: StatusSnapshotV1): string {
  const cpu = availableValue(snapshot.cpu);
  if (!cpu) return taggedLine(locale, "cpu", snapshot.cpu);
  return message(locale, "status.v1.cpu", { percent: formatBasisPoints(cpu.system_utilization_basis_points) });
}

function memoryCard(locale: PresentationLanguageTag, snapshot: StatusSnapshotV1): string {
  const memory = availableValue(snapshot.memory);
  if (!memory) return taggedLine(locale, "memory", snapshot.memory);
  const text = message(locale, "status.v1.memory", {
    used: formatBytes(memory.used_bytes),
    total: formatBytes(memory.total_bytes),
    available: formatBytes(memory.available_bytes),
  });
  return snapshot.memory.state === "partial" ? `${text} ${taggedLine(locale, "memory", snapshot.memory)}` : text;
}

function volumeCards(locale: PresentationLanguageTag, snapshot: StatusSnapshotV1): string[] {
  const volumes = availableValue(snapshot.volumes);
  if (!volumes) return [taggedLine(locale, "volumes", snapshot.volumes)];
  return volumes.items.map((volume) => message(locale, "status.v1.volume.item", {
    mount: volume.mount_points[0] ?? "-",
    available: formatBytes(volume.available_bytes),
    total: formatBytes(volume.total_bytes),
  }));
}

function networkCards(locale: PresentationLanguageTag, snapshot: StatusSnapshotV1): string[] {
  const network = availableValue(snapshot.network);
  if (!network) return [taggedLine(locale, "network", snapshot.network)];
  return network.interfaces.map((iface) => message(locale, "status.v1.network.item", {
    name: iface.name,
    rx: formatBytes(iface.rx_bytes_per_second),
    tx: formatBytes(iface.tx_bytes_per_second),
  }));
}

function powerCard(locale: PresentationLanguageTag, snapshot: StatusSnapshotV1): string {
  const power = availableValue(snapshot.power);
  if (!power) return taggedLine(locale, "power", snapshot.power);
  const ac = message(locale, `status.v1.ac.${power.ac_state}` as MessageKey);
  const acLine = message(locale, "status.v1.power.ac", { ac });
  if (!power.battery_present) return `${acLine} ${message(locale, "status.v1.power.no_battery")}`;
  if (power.charge_basis_points === null) return acLine;
  return `${acLine} ${message(locale, "status.v1.power.battery", {
    percent: formatBasisPoints(power.charge_basis_points),
    remaining: power.remaining_seconds === null ? "-" : String(power.remaining_seconds),
  })}`;
}

function gpuCards(locale: PresentationLanguageTag, snapshot: StatusSnapshotV1): string[] {
  const gpu = availableValue(snapshot.gpu);
  if (!gpu) return [taggedLine(locale, "gpu", snapshot.gpu)];
  return gpu.adapters.map((adapter) => message(locale, "status.v1.gpu.adapter", {
    adapter: adapter.adapter_id,
    percent: formatBasisPoints(adapter.utilization_basis_points),
  }));
}

function thermalCards(locale: PresentationLanguageTag, snapshot: StatusSnapshotV1): string[] {
  const thermal = availableValue(snapshot.thermal);
  if (!thermal) return [taggedLine(locale, "thermal", snapshot.thermal)];
  return thermal.zones.map((zone) => message(locale, "status.v1.thermal.zone", {
    zone: zone.zone_id,
    celsius: formatTenths(zone.temperature_tenths_celsius),
  }));
}

/** Memory, GPU, temperature, and battery for the one-line stage facts. */
function stageFacts(locale: PresentationLanguageTag, snapshot: StatusSnapshotV1): string[] {
  const facts: string[] = [];
  const memory = memoryBasisPoints(snapshot);
  facts.push(memory === null
    ? message(locale, "status.v1.stage.memory.unavailable")
    : message(locale, "status.v1.stage.memory", { percent: formatBasisPoints(memory) }));
  const gpu = peakGpuBasisPoints(snapshot);
  facts.push(gpu === null
    ? message(locale, "status.v1.stage.gpu.unavailable")
    : message(locale, "status.v1.stage.gpu", { percent: formatBasisPoints(gpu) }));
  const temperature = peakTemperatureTenths(snapshot);
  facts.push(temperature === null
    ? message(locale, "status.v1.stage.temperature.unavailable")
    : message(locale, "status.v1.stage.temperature", { celsius: formatTenths(temperature) }));
  const power = availableValue(snapshot.power);
  if (power?.battery_present) {
    const battery = message(locale, "status.v1.stage.battery", {
      percent: power.charge_basis_points === null ? "-" : formatBasisPoints(power.charge_basis_points),
      ac: message(locale, `status.v1.ac.${power.ac_state}` as MessageKey),
    });
    facts.push(power.remaining_seconds === null
      ? battery
      : `${battery}, ${message(locale, "status.v1.stage.battery.remaining", { minutes: String(Math.floor(power.remaining_seconds / 60)) })}`);
  }
  return facts;
}

function chartValues(locale: PresentationLanguageTag, points: readonly ChartPoint[], present: (point: ChartPoint) => string | null): string {
  const gap = message(locale, "status.v1.chart.gap");
  return points.map((point) => present(point) ?? gap).join(", ");
}

function chartSegments(points: readonly ChartPoint[], present: (point: ChartPoint) => number | null): string[] {
  const width = Math.max(points.length - 1, 1);
  const values = points.map(present);
  const finite = values.filter((value): value is number => value !== null);
  const max = Math.max(...finite, 1);
  const segments: string[] = [];
  let current: string[] = [];
  values.forEach((value, index) => {
    if (value === null) {
      if (current.length > 1) segments.push(current.join(" "));
      current = [];
      return;
    }
    const x = (index / width) * 100;
    const y = 40 - (value / max) * 36;
    current.push(`${x.toFixed(2)},${y.toFixed(2)}`);
  });
  if (current.length > 1) segments.push(current.join(" "));
  return segments;
}

type MetricFamily = "cpu" | "memory" | "power" | "volume" | "network" | "gpu" | "thermal";

function metricGlyph(family: MetricFamily): GlyphName {
  switch (family) {
    case "cpu": return "status";
    case "memory": return "package_cache";
    case "power": return "tool_cache";
    case "volume": return "dependency_directory";
    case "network": return "node";
    case "gpu": return "generic";
    case "thermal": return "risk";
  }
}

function StatusMetricCard({
  family,
  title,
  value,
}: {
  readonly family: MetricFamily;
  readonly title?: string;
  readonly value: string;
}) {
  return <article className="card status-card">
    <span className="glyph-tile"><DestinationGlyph name={metricGlyph(family)} /></span>
    <div>
      {title ? <h3>{title}</h3> : null}
      <p><AccessibleUserData value={value} /></p>
    </div>
  </article>;
}

function StatusChart({
  locale,
  labelKey,
  points,
  present,
  format,
}: {
  readonly locale: PresentationLanguageTag;
  readonly labelKey: MessageKey;
  readonly points: readonly ChartPoint[];
  readonly present: (point: ChartPoint) => number | null;
  readonly format: (value: number) => string;
}) {
  const label = message(locale, labelKey);
  const values = chartValues(locale, points, (point) => {
    const value = present(point);
    return value === null ? null : format(value);
  });
  const segments = chartSegments(points, present);
  return <figure className="status-chart">
    <figcaption>{label}</figcaption>
    <svg viewBox="0 0 100 48" role="img" aria-label={message(locale, "status.v1.chart.alternative", { label, values })}>
      {segments.map((pointsAttr) => <polyline key={pointsAttr} className="status-chart-line" points={pointsAttr} />)}
    </svg>
    <p className="status-chart-values">{message(locale, "status.v1.chart.alternative", { label, values })}</p>
  </figure>;
}

export function StatusWorkbench({ bridge, coordinator, locale }: StatusWorkbenchProps) {
  const preferences = usePreferences();
  const [state, dispatch] = useReducer(statusReducer, initialStatusState);
  const intervalMs = preferences.preferences.status_interval_seconds * 1000;
  const processLimit = preferences.preferences.status_process_limit;
  const intervalRequest = useRef(0);
  const activeOperation = useRef(state.operationId);
  useLayoutEffect(() => { activeOperation.current = state.operationId; }, [state.operationId]);
  const [details, setDetails] = useState(false);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      intervalRequest.current += 1;
      dispatch({ type: "released" });
    };
  }, []);

  const runOperation = async <T,>(
    operation: StatusOperation,
    start: (operationId: string) => Promise<T>,
    completed: (operationId: string, result: T) => void,
  ) => {
    const operationId = createOperationId(operation);
    let started = false;
    try {
      const lease = await coordinator.start<T>({
        kind: "status",
        id: operationId,
        cancel: async () => {
          if (mounted.current) dispatch({ type: "cancel_requested", operationId });
          await bridge.statusCancel(operationId);
        },
        start: () => {
          started = true;
          if (mounted.current) dispatch({ type: "operation_requested", operationId, operation });
          return start(operationId);
        },
      });
      if (!lease) return;
      try {
        const result = await lease.result;
        if (mounted.current) completed(operationId, result);
      } finally {
        await lease.complete().catch(() => false);
      }
    } catch (error) {
      if (started && mounted.current) dispatch({ type: "operation_failed", operationId, error: commandError(error) });
    }
  };

  const captureSnapshot = () => void runOperation<DesktopStatusSnapshotResult>(
    "snapshot",
    (operationId) => bridge.statusSnapshot(operationId, processLimit),
    (operationId, result) => {
      if (result.type === "completed") dispatch({ type: "snapshot_completed", operationId, snapshot: result.snapshot });
      else dispatch({ type: "operation_canceled", operationId });
    },
  );

  const startLive = (nextIntervalMs = intervalMs, nextProcessLimit = processLimit) => {
    void runOperation<DesktopStatusLiveResult>(
      "live",
      (operationId) => bridge.statusLiveStart(
        operationId,
        nextIntervalMs,
        nextProcessLimit,
        (event: StatusEventV1) => {
          if (mounted.current) dispatch({ type: "live_event", operationId, event });
        },
        (error) => {
          if (mounted.current) dispatch({ type: "operation_failed", operationId, error: error instanceof Error ? error.message : String(error) });
        },
      ),
      (operationId, result) => {
        if (result.type === "canceled") dispatch({ type: "operation_canceled", operationId });
      },
    );
  };

  const cancelActive = async () => {
    intervalRequest.current += 1;
    const operationId = state.operationId;
    if (!operationId) return;
    dispatch({ type: "cancel_requested", operationId });
    try { await bridge.statusCancel(operationId); }
    catch (error) {
      dispatch({ type: "operation_failed", operationId, error: commandError(error) });
    }
  };

  useEffect(() => {
    let disposed = false;
    queueMicrotask(() => { if (!disposed) captureSnapshot(); });
    // Auto-load one snapshot on enter; live remains opt-in.
    return () => { disposed = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const active = state.operationId !== null;
  const live = state.status === "live" || state.status === "canceling";
  const statusText = state.status === "snapshot" ? message(locale, "status.v1.state.loading")
    : state.status === "live" ? message(locale, "status.v1.state.live")
      : state.status === "canceling" ? message(locale, "status.v1.state.canceling")
        : state.status === "failed" ? message(locale, "status.v1.state.failed")
          : state.status === "ready" ? message(locale, "status.v1.state.ready")
            : null;
  const chipTone = state.status === "failed" ? "status-chip-danger"
    : state.status === "live" || state.status === "ready" ? "status-chip-ok"
      : state.status === "canceling" ? "status-chip-warning"
        : "";
  const processes = processRows(state);
  const pinLimitReached = state.pins.length >= MAX_PINNED_PROCESSES;
  const processGroup = state.snapshot ? availableValue(state.snapshot.processes) : null;
  const decodeError = typeof state.error === "string" ? state.error : null;
  const commandErr = state.error && typeof state.error !== "string" ? state.error : null;

  const statusChip = statusText ? <span className={`status-chip ${chipTone}`.trim()}><span className="status-chip-dot" aria-hidden="true" />{statusText}</span> : null;
  const liveStatus = statusText ? <p className="status-live-status" role="status" aria-live="polite" aria-label={statusText}>{statusText}</p> : null;
  const liveToggle = (className: string, stopClassName: string) => live
    ? <button type="button" className={stopClassName} disabled={state.status === "canceling"} onClick={() => void cancelActive()}>
        {message(locale, "status.v1.action.live.stop")}
      </button>
    : <button type="button" className={className} disabled={active} onClick={() => startLive()}>
        {message(locale, "status.v1.action.live.start")}
      </button>;
  const errors = <>
    {preferences.saveError && <p className="preferences-warning" role="alert">{message(locale, "preferences.v1.save.error")}</p>}
    {commandErr ? <ErrorBanner error={commandErr} onDismiss={() => dispatch({ type: "error_dismissed" })} /> : null}
    {decodeError ? <div className="error-banner" role="alert">
      <span>{message(locale, "status.v1.decode.error", { reason: decodeError })}</span>
      <button type="button" className="icon-button" onClick={() => dispatch({ type: "error_dismissed" })} aria-label="Dismiss error">×</button>
    </div> : null}
  </>;
  const cpu = state.snapshot ? availableValue(state.snapshot.cpu) : null;

  return <div className="status-mode" data-status={state.status}>
    {!details || !state.snapshot ? <>
      {!state.snapshot ? <Stage
        mode="status"
        label={message(locale, "command.status")}
        busy={active}
        title={message(locale, "status.v1.chart.empty")}
        meta={<><p>{message(locale, "status.v1.capability.note")}</p>{liveStatus}</>}
        primary={<button type="button" className="stage-action" disabled={active} onClick={captureSnapshot}>{message(locale, "status.v1.action.snapshot")}</button>}
        secondary={liveToggle("stage-link", "stage-link")}
      /> : <StageResult
        mode="status"
        label={message(locale, "command.status")}
        caption={message(locale, "status.v1.chart.cpu")}
        value={cpu ? formatBasisPoints(cpu.system_utilization_basis_points) : cpuCard(locale, state.snapshot)}
        unit={cpu ? "%" : undefined}
        meta={<><p className="status-stage-facts">{stageFacts(locale, state.snapshot).join(" · ")}</p>{liveStatus}</>}
        action={liveToggle("stage-action", "stage-action")}
        secondary={<button type="button" className="stage-link" onClick={() => setDetails(true)}>{message(locale, "stage.v1.action.details")}</button>}
      />}
      {errors}
    </> : <DetailView locale={locale} onBack={() => setDetails(false)} status={statusChip}>
    <section className="status-toolbar" aria-label={message(locale, "command.status")}>
      {state.snapshot ? <button type="button" className="primary-button" disabled={active} onClick={captureSnapshot}>
        {message(locale, "status.v1.action.snapshot")}
      </button> : null}
      {liveToggle("secondary-button", "danger-button")}
      <label>
        <span>{message(locale, "status.v1.interval.label")}</span>
        <select
          aria-label={message(locale, "status.v1.interval.label")}
          value={String(intervalMs)}
          disabled={preferences.saving || preferences.unavailable || preferences.loading || state.status === "canceling"}
          onChange={(event) => {
            const request = ++intervalRequest.current;
            const wasLive = state.status === "live";
            const previousOperation = state.operationId;
            void preferences.update({ field: "status_interval_seconds", value: Number(event.target.value) / 1000 }).then((snapshot) => {
              if (!snapshot || !mounted.current || request !== intervalRequest.current) return;
              if (wasLive && previousOperation === activeOperation.current) startLive(snapshot.preferences.status_interval_seconds * 1000, snapshot.preferences.status_process_limit);
            });
          }}
        >
          {INTERVAL_STEPS_MS.map((step) => (
            <option key={step} value={step}>{message(locale, "status.v1.interval.value", { seconds: String(step / 1000) })}</option>
          ))}
        </select>
      </label>
      {liveStatus}
    </section>

    {errors}

    <>
      <section className="card status-capability">
        <p>{message(locale, "status.v1.snapshot.title", { id: state.snapshot.snapshot_id })}</p>
        <p>{message(locale, "status.v1.sampled_at", { timestamp: String(state.snapshot.sampled_at_unix_ms) })}</p>
        <p>{message(locale, "status.v1.logical.processors", { count: String(state.snapshot.logical_processor_count) })}</p>
        <p>{message(locale, "status.v1.capability.note")}</p>
      </section>
      <section className="status-cards" aria-label={message(locale, "command.status")}>
        <StatusMetricCard family="cpu" title={message(locale, "status.v1.chart.cpu")} value={cpuCard(locale, state.snapshot)} />
        <StatusMetricCard family="memory" title={message(locale, "status.v1.chart.memory")} value={memoryCard(locale, state.snapshot)} />
        <StatusMetricCard family="power" value={powerCard(locale, state.snapshot)} />
        {gpuCards(locale, state.snapshot).map((line) => <StatusMetricCard family="gpu" key={line} title={message(locale, "status.v1.chart.gpu")} value={line} />)}
        {thermalCards(locale, state.snapshot).map((line) => <StatusMetricCard family="thermal" key={line} title={message(locale, "status.v1.chart.temperature")} value={line} />)}
        {volumeCards(locale, state.snapshot).map((line) => <StatusMetricCard family="volume" key={line} value={line} />)}
        {networkCards(locale, state.snapshot).map((line) => <StatusMetricCard family="network" key={line} value={line} />)}
      </section>
    </>

    <section className="card status-charts">
      <h2>{message(locale, "status.v1.state.live")}</h2>
      {state.chart.length === 0
        ? <p>{message(locale, "status.v1.chart.empty")}</p>
        : <>
          <StatusChart locale={locale} labelKey="status.v1.chart.cpu" points={state.chart} present={(point) => point.cpuBp} format={formatBasisPoints} />
          <StatusChart locale={locale} labelKey="status.v1.chart.memory" points={state.chart} present={(point) => point.memoryUsedBytes} format={formatBytes} />
          <StatusChart locale={locale} labelKey="status.v1.chart.network.rx" points={state.chart} present={(point) => point.rxBytesPerSecond} format={formatBytes} />
          <StatusChart locale={locale} labelKey="status.v1.chart.network.tx" points={state.chart} present={(point) => point.txBytesPerSecond} format={formatBytes} />
        </>}
    </section>

    <section className="card status-processes">
      <h2>{message(locale, "status.v1.process.summary", {
        returned: String(processGroup?.returned_count ?? 0),
        enumerated: String(processGroup?.enumerated_count ?? 0),
        limit: String(processGroup?.requested_limit ?? state.processLimit),
      })}</h2>
      {processGroup?.truncated_by_limit
        ? <p className="status-warning">{message(locale, "status.v1.process.truncated", {
            returned: String(processGroup.returned_count),
            enumerated: String(processGroup.enumerated_count),
          })}</p>
        : null}
      {processGroup?.budget_exhausted
        ? <p className="status-warning">{message(locale, "status.v1.process.budget", {
            budget: String(processGroup.detail_budget_ms),
          })}</p>
        : null}
      {pinLimitReached
        ? <p className="status-muted">{message(locale, "status.v1.process.pin.limit", { limit: String(MAX_PINNED_PROCESSES) })}</p>
        : null}
      <div className="status-table-frame">
        <table className="status-table">
          <thead>
            <tr>
              <th scope="col">{message(locale, "status.v1.table.pin")}</th>
              {(["name", "pid", "cpu", "memory"] as const satisfies readonly ProcessSort[]).map((sort) => {
                const active = state.processSort === sort;
                return <th key={sort} scope="col" aria-sort={active ? state.processSortDirection : undefined}>
                  <button type="button" onClick={() => dispatch({ type: "sort_changed", sort })}>
                    {message(locale, `status.v1.table.${sort}` as MessageKey)}
                    {active ? <span aria-hidden="true">{state.processSortDirection === "ascending" ? " \u25B2" : " \u25BC"}</span> : null}
                  </button>
                </th>;
              })}
              <th scope="col">{message(locale, "status.v1.table.read")}</th>
              <th scope="col">{message(locale, "status.v1.table.write")}</th>
            </tr>
          </thead>
          <tbody>
            {processes.map((row) => {
              if (row.kind === "absent") {
                return <tr key={`absent-${row.pid}`} className="status-row-pinned status-row-absent">
                  <td>
                    <button type="button" className="status-pin" aria-pressed="true" aria-label={message(locale, "status.v1.process.unpin", { name: row.name, pid: String(row.pid) })} onClick={() => dispatch({ type: "pin_toggled", pid: row.pid, name: row.name })}>
                      {message(locale, "status.v1.action.unpin")}
                    </button>
                  </td>
                  <td><AccessibleUserData value={row.name} /></td>
                  <td>{row.pid}</td>
                  <td colSpan={4}>{message(locale, row.status === "exited" ? "status.v1.process.exited" : "status.v1.process.unsampled")}</td>
                </tr>;
              }
              const { process, pinned } = row;
              const label = { name: process.name, pid: String(process.pid) };
              return <tr key={process.pid} className={pinned ? "status-row-pinned" : undefined}>
                <td>
                  <button
                    type="button"
                    className="status-pin"
                    aria-pressed={pinned}
                    aria-label={message(locale, pinned ? "status.v1.process.unpin" : "status.v1.process.pin", label)}
                    disabled={!pinned && pinLimitReached}
                    onClick={() => dispatch({ type: "pin_toggled", pid: process.pid, name: process.name })}
                  >
                    {message(locale, pinned ? "status.v1.action.unpin" : "status.v1.action.pin")}
                  </button>
                </td>
                <td><AccessibleUserData value={process.name} /></td>
                <td>{process.pid}</td>
                <td>{formatBasisPoints(process.cpu_basis_points_of_one_logical_core)}</td>
                <td>{formatBytes(process.private_bytes)}</td>
                <td>{formatBytes(process.read_bytes_per_second)}</td>
                <td>{formatBytes(process.write_bytes_per_second)}</td>
              </tr>;
            })}
          </tbody>
        </table>
      </div>
    </section>
    </DetailView>}
  </div>;
}
