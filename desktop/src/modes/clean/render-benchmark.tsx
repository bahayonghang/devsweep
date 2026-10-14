import { useLayoutEffect, useReducer } from "react";
import { createRoot } from "react-dom/client";
import { decodeDesktopScanProgress, decodeScanReport } from "../../api/contract";
import type { DesktopScanProgress, ScanReport, UntrustedTarget } from "../../api/types.gen";
import { TargetTable } from "../../components/TargetTable";
import { ReviewPage } from "../../pages/ReviewPage";
import type { PresentationLanguageTag } from "../../i18n";
import { appReducer, initialState, type AppAction, type AppState } from "../../state/app-state";
import "../../styles.css";
import "./styles.css";

declare const __CLEAN_BUILD__: { label: string; source_sha256: string };

const WARMUPS = 5;
const SAMPLES = 30;
const ROOT = "C:/clean-render-fixture";
let dispatch: ((action: AppAction) => void) | undefined;
let commit: (() => void) | undefined;
let previewMode = false;
let locale: PresentationLanguageTag = "en";
let pendingStart = 0;
let reducerMs = 0;
let current: AppState = initialState;
let root = createRoot(document.getElementById("root")!);
const p95 = (values: number[]) => [...values].sort((a, b) => a - b)[Math.ceil(values.length * .95) - 1];
const frame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

function fixture(count: number): ScanReport {
  const targets: UntrustedTarget[] = Array.from({ length: count }, (_, index) => {
    const project = `${ROOT}/project-${String(index).padStart(5, "0")}-空间`;
    const path = `${project}/node_modules`;
    const inspect = index % 17 === 16;
    return {
      id: `node.node_modules:${path}`, rule_id: "node.node_modules", scope: { type: "project", root: project },
      ecosystem: "node", kind: "dependency_directory", path,
      estimated_bytes: 4096 + index, size_complete: true, sizing_warnings: [], last_modified: null,
      risk: inspect ? "high" : "medium", reversible: true, selected_by_default: false,
      intent: { type: inspect ? "inspect_only" : "trash_project_artifact", rule_id: "node.node_modules" },
      evidence: [
        { type: "marker_file", path: `${project}/package.json` },
        { type: "rule_matched", rule_id: "node.node_modules" },
        { type: "known_cache_dir", source: "deterministic fixture", path },
        { type: "user_configured" },
      ],
    };
  });
  return { version: 1, plan: { version: 2, targets }, health: {
    completeness: "complete", diagnostics: [], coverage: [],
    totals: { verified_bytes: targets.reduce((sum, target) => sum + target.estimated_bytes, 0), partial_lower_bound_bytes: 0, unknown_target_count: 0 },
  } };
}

function progress(report: ScanReport, sequence: number): DesktopScanProgress {
  return { scan_id: "benchmark", sequence, phase: "projects", message: `Fixture ${sequence}`, preview: {
    targets: report.plan.targets.map((target) => ({
      id: target.id, scope: target.scope, ecosystem: target.ecosystem, kind: target.kind, path: target.path,
      estimated_bytes: target.estimated_bytes, size_complete: target.size_complete,
      sizing_warnings: target.sizing_warnings ?? [], last_modified: target.last_modified, risk: target.risk,
      evidence: target.evidence, disposition: target.intent.type === "inspect_only" ? "inspect_only" : "candidate",
    })),
    totals: { ...report.health.totals, target_count: report.plan.targets.length },
  } };
}

function measuredReducer(state: AppState, action: AppAction) {
  const start = performance.now();
  const next = appReducer(state, action);
  reducerMs += performance.now() - start;
  return next;
}

function Surface({ start }: { start: AppState }) {
  const [state, send] = useReducer(measuredReducer, start);
  useLayoutEffect(() => {
    dispatch = send;
    current = state;
    commit?.();
  }, [state]);
  return <main style={{ padding: 24 }}>
    <output id="selection-count">{state.selectedIds.size}</output>
    {previewMode ? <TargetTable locale={locale} mode="preview" targets={state.activeScan?.preview?.targets ?? []} /> : <ReviewPage locale={locale} state={state}
      onSelect={(targetId, selected) => send({ type: "selection_changed", targetId, selected })}
      onSelectAll={(selected) => send({ type: "select_all_changed", selected })}
      onDryRun={() => undefined}
    />}
  </main>;
}

async function show(mode: "review" | "preview", language: PresentationLanguageTag) {
  locale = language;
  const report = decodeScanReport(fixture(10_000));
  const event = decodeDesktopScanProgress(progress(report, 1));
  previewMode = mode === "preview";
  const scan = appReducer(initialState, { type: "scan_requested", scanId: "benchmark" });
  const state = appReducer(scan, previewMode ? { type: "scan_progressed", progress: event } : { type: "scan_completed", scanId: "benchmark", report });
  root.unmount(); root = createRoot(document.getElementById("root")!);
  await update(() => root.render(<Surface start={state} />));
  return { retained: current.scan?.plan.targets.length ?? current.activeScan?.preview?.targets.length, build: __CLEAN_BUILD__ };
}

async function update(run: () => void) {
  await frame();
  reducerMs = 0;
  pendingStart = performance.now();
  let commitMs = 0;
  const done = new Promise<void>((resolve, reject) => {
    const deadline = setTimeout(() => { commit = undefined; reject(new Error("Production component did not commit within 20 seconds")); }, 20_000);
    commit = () => {
      commitMs = performance.now() - pendingStart;
      clearTimeout(deadline); commit = undefined; resolve();
    };
  });
  run();
  await done;
  // Two frame boundaries include a painting opportunity after the real commit.
  await frame(); await frame();
  return { commit_ms: commitMs, visible_ms: performance.now() - pendingStart, reducer_ms: reducerMs };
}

async function run(count: number, mode: "review" | "preview") {
  if (![100, 1000, 10000].includes(count)) throw new Error("Only frozen fixture sizes are supported");
  const report = fixture(count);
  const wire = JSON.stringify(report);
  const fixtureSha = [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(wire)))].map((value) => value.toString(16).padStart(2, "0")).join("");
  const parse: number[] = [], decode: number[] = [], reduction: number[] = [];
  const scan = appReducer(initialState, { type: "scan_requested", scanId: "benchmark" });
  const eventWire = JSON.stringify(progress(report, 1));
  let decodedReport = report;
  let decodedProgress = progress(report, 1);
  for (let index = 0; index < WARMUPS + SAMPLES; index += 1) {
    let began = performance.now();
    const value: unknown = JSON.parse(mode === "review" ? wire : eventWire);
    const parseMs = performance.now() - began;
    began = performance.now();
    if (mode === "review") decodedReport = decodeScanReport(value);
    else decodedProgress = decodeDesktopScanProgress(value);
    const decodeMs = performance.now() - began;
    began = performance.now();
    appReducer(scan, mode === "review" ? { type: "scan_completed", scanId: "benchmark", report: decodedReport } : { type: "scan_progressed", progress: decodedProgress });
    if (index >= WARMUPS) { parse.push(parseMs); decode.push(decodeMs); reduction.push(performance.now() - began); }
  }
  previewMode = mode === "preview";
  root.unmount(); root = createRoot(document.getElementById("root")!);
  const state = appReducer(scan, previewMode ? { type: "scan_progressed", progress: decodedProgress } : { type: "scan_completed", scanId: "benchmark", report: decodedReport });
  const mount = await update(() => root.render(<Surface start={state} />));
  const row = [], all = [], replay = [];
  const replayDecode: number[] = [];
  const selectionCounts: number[] = [];
  const executableCount = report.plan.targets.filter((target) => target.intent.type !== "inspect_only").length;
  for (let index = 0; index < WARMUPS + SAMPLES; index += 1) {
    if (previewMode) {
      const began = performance.now();
      const fresh = decodeDesktopScanProgress(JSON.parse(eventWire));
      const decodeDuration = performance.now() - began;
      const measurement = await update(() => dispatch?.({ type: "scan_progressed", progress: { ...fresh, sequence: index + 2 } }));
      if (index >= WARMUPS) { replay.push(measurement); replayDecode.push(decodeDuration); }
    } else {
      const rowMeasurement = await update(() => {
        const input = document.querySelector<HTMLInputElement>("tbody input[type=checkbox]:not(:disabled)");
        if (!input) throw new Error("No selectable production row");
        input.click();
      });
      const allMeasurement = await update(() => {
        const input = document.querySelector<HTMLInputElement>(".select-all-control input");
        if (!input) throw new Error("Production select-all control missing");
        input.click();
      });
      if (current.selectedIds.size !== executableCount) throw new Error("Select-all lost a target or selected inspect-only data");
      const selectedCount = current.selectedIds.size;
      const clearMeasurement = await update(() => document.querySelector<HTMLInputElement>(".select-all-control input")!.click());
      if (Number(current.selectedIds.size) !== 0) throw new Error("Deselect-all retained selection");
      if (index >= WARMUPS) { row.push(rowMeasurement); all.push(allMeasurement, clearMeasurement); selectionCounts.push(selectedCount, current.selectedIds.size); }
    }
  }
  return {
    count, mode, build: __CLEAN_BUILD__, fixture_sha256: fixtureSha, fixture_version: "clean-render-v1",
    warmups: WARMUPS, measured_samples: SAMPLES, percentile: "nearest-rank p95",
    parse_ms: parse, decode_ms: decode, report_reducer_ms: reduction, mount, row, select_all: all, preview_replay: replay,
    preview_replay_parse_decode_ms: replayDecode, selection_counts: selectionCounts, executable_count: executableCount,
    p95_ms: { parse: p95(parse), decode: p95(decode), report_reducer: p95(reduction),
      row_visible: row.length ? p95(row.map((sample) => sample.visible_ms)) : null,
      select_all_visible: all.length ? p95(all.map((sample) => sample.visible_ms)) : null,
      preview_visible: replay.length ? p95(replay.map((sample) => sample.visible_ms)) : null },
    dom_elements: document.querySelectorAll("*").length, rendered_rows: document.querySelectorAll("tbody tr").length,
    retained_targets: current.scan?.plan.targets.length ?? current.activeScan?.preview?.targets.length ?? 0,
    browser: navigator.userAgent, logical_processors: navigator.hardwareConcurrency, dpr: devicePixelRatio,
    method: "Production ReviewPage/TargetTable, closed decoders and reducer; real checkbox click to layout commit plus two RAF paint boundaries. Synthetic clicks measure handler-to-visible, not OS input scheduling. No native cleanup bridge.",
  };
}

declare global { interface Window { cleanBenchmark: { run: typeof run; show: typeof show } } }
window.cleanBenchmark = { run, show };
