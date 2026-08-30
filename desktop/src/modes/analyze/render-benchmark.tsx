import ReactDOM from "react-dom/client";
import type { DesktopBridge } from "../../api/bridge";
import type { AnalyzeNodeV1, AnalyzeSnapshotV1 } from "../../api/types.gen";
import { OperationCoordinator } from "../../state/operation-coordinator";
import "../../styles.css";
import { AnalyzePage, type AnalyzePerformanceSample } from "./AnalyzePage";

const FIXTURE_TOTAL_NODES = 250_000;
const CURRENT_DIRECTORY_CHILDREN = 10_000;
const WARMUPS = 5;
const MEASURED_NAVIGATIONS = 30;
const COMMIT_LIMIT_MS = 100;
const DOM_ELEMENT_LIMIT = 900;
const RECTANGLE_LIMIT = 513;

interface NavigationMeasurement {
  readonly durationMs: number;
  readonly rectangles: number;
  readonly domElements: number;
}

interface AnalyzeRenderBenchmarkResult {
  readonly status: "pass" | "budget_miss" | "error";
  readonly fixture: string;
  readonly fixture_total_nodes: number;
  readonly current_directory_children: number;
  readonly fixture_generation: string;
  readonly fixture_children_sha256: string;
  readonly warmups: number;
  readonly measured_navigations: number;
  readonly nearest_rank: number;
  readonly react_commit_samples_ms: readonly number[];
  readonly react_commit_p95_ms: number;
  readonly rectangle_count_max: number;
  readonly dom_elements_max: number;
  readonly limits: {
    readonly rectangle_count_max: number;
    readonly dom_elements_max: number;
    readonly react_commit_p95_ms_max: number;
  };
  readonly measurement_method: string;
  readonly build: string;
  readonly browser_webview: string;
  readonly platform: string;
  readonly hardware_concurrency: number | null;
  readonly memory_counts: null;
  readonly notes: readonly string[];
}

declare global {
  interface Window {
    __DEVSWEEP_ANALYZE_BENCHMARK_RESULT__?: AnalyzeRenderBenchmarkResult;
    __DEVSWEEP_ANALYZE_BENCHMARK_PROMISE__?: Promise<AnalyzeRenderBenchmarkResult>;
  }
}

const wideSeeds = Array.from({ length: CURRENT_DIRECTORY_CHILDREN }, (_, index) => ({
  id: index + 2,
  parent_id: 1,
  name: `wide-${index.toString().padStart(5, "0")}.bin`,
  kind: "file" as const,
  bytes: (index % 97) + 1,
  evidence: "complete" as const,
  completeness: "complete" as const,
  immediate_count: 0,
}));

function fileNode(id: number, parentId: number, name: string, bytes: number): AnalyzeNodeV1 {
  return {
    id,
    parent_id: parentId,
    name,
    kind: "file",
    bytes,
    evidence: "complete",
    immediate_count: 0,
    recursive_count: 0,
    warnings: [],
    mtime_ms: null,
  };
}

function fixture(): AnalyzeSnapshotV1 {
  const wideChildren = wideSeeds.map((seed) => fileNode(seed.id, seed.parent_id, seed.name, seed.bytes));
  const wideBytes = wideChildren.reduce((total, node) => total + node.bytes, 0);
  const overflowDirectoryId = CURRENT_DIRECTORY_CHILDREN + 2;
  const overflowChildren: AnalyzeNodeV1[] = [];
  for (let id = overflowDirectoryId + 1; id < FIXTURE_TOTAL_NODES; id += 1) {
    overflowChildren.push(fileNode(id, overflowDirectoryId, `overflow-${id}.bin`, 1));
  }
  const overflowBytes = overflowChildren.length;
  const root: AnalyzeNodeV1 = {
    id: 0,
    parent_id: null,
    name: "analysis-250k-v1",
    kind: "directory",
    bytes: wideBytes + overflowBytes,
    evidence: "complete",
    immediate_count: 2,
    recursive_count: FIXTURE_TOTAL_NODES - 1,
    warnings: [],
    mtime_ms: null,
  };
  const wideDirectory: AnalyzeNodeV1 = {
    id: 1,
    parent_id: 0,
    name: "wide-directory",
    kind: "directory",
    bytes: wideBytes,
    evidence: "complete",
    immediate_count: wideChildren.length,
    recursive_count: wideChildren.length,
    warnings: [],
    mtime_ms: null,
  };
  const overflowDirectory: AnalyzeNodeV1 = {
    id: overflowDirectoryId,
    parent_id: 0,
    name: "overflow-directory",
    kind: "directory",
    bytes: overflowBytes,
    evidence: "complete",
    immediate_count: overflowChildren.length,
    recursive_count: overflowChildren.length,
    warnings: [],
    mtime_ms: null,
  };
  return {
    version: 1,
    root: {
      input: "C:/fixture/analysis-250k-v1",
      normalized: "C:/fixture/analysis-250k-v1",
      volume: "fixture-volume",
    },
    nodes: [root, wideDirectory, ...wideChildren, overflowDirectory, ...overflowChildren],
    warnings: [],
    completeness: "complete",
    accounted_owned_bytes: 0,
  };
}

async function sha256(value: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return `sha256:${Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("")}`;
}

function benchmarkBridge(snapshot: AnalyzeSnapshotV1): DesktopBridge {
  return {
    analyzeStart: async (operationId) => ({ type: "completed", operation_id: operationId, snapshot }),
    analyzeCancel: async () => undefined,
    scanStart: async () => { throw new Error("unused benchmark bridge method"); },
    scanCancel: async () => undefined,
    planDryRun: async () => { throw new Error("unused benchmark bridge method"); },
    planExecute: async () => { throw new Error("unused benchmark bridge method"); },
  };
}

function animationFrame(): Promise<void> {
  return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

async function waitFor<T>(read: () => T | null, label: string, timeoutMs = 30_000): Promise<T> {
  const deadline = performance.now() + timeoutMs;
  for (;;) {
    const value = read();
    if (value !== null) return value;
    if (performance.now() >= deadline) throw new Error(`Timed out waiting for ${label}`);
    await animationFrame();
  }
}

let commitWaiter: ((sample: AnalyzePerformanceSample, endedAt: number) => void) | null = null;

function nextCommit(): Promise<{ readonly sample: AnalyzePerformanceSample; readonly endedAt: number }> {
  if (commitWaiter) throw new Error("Concurrent benchmark navigation is not supported");
  return new Promise((resolve, reject) => {
    const timeout = window.setTimeout(() => {
      commitWaiter = null;
      reject(new Error("Timed out waiting for the AnalyzePage post-commit layout effect"));
    }, 30_000);
    commitWaiter = (sample, endedAt) => {
      window.clearTimeout(timeout);
      commitWaiter = null;
      resolve({ sample, endedAt });
    };
  });
}

function onPerformanceSample(sample: AnalyzePerformanceSample): void {
  if (sample.kind === "commit") commitWaiter?.(sample, performance.now());
}

function findWideTile(): SVGRectElement | null {
  return Array.from(document.querySelectorAll<SVGRectElement>(".analyze-tile"))
    .find((tile) => tile.getAttribute("aria-label")?.startsWith("wide-directory —")) ?? null;
}

async function drillIntoWideDirectory(rootElement: HTMLElement): Promise<NavigationMeasurement> {
  const tile = await waitFor(findWideTile, "the wide-directory treemap tile");
  const committed = nextCommit();
  const startedAt = performance.now();
  tile.dispatchEvent(new MouseEvent("dblclick", { bubbles: true, cancelable: true, view: window }));
  const { sample, endedAt } = await committed;
  return {
    durationMs: endedAt - startedAt,
    rectangles: sample.rectangles,
    domElements: rootElement.querySelectorAll("*").length,
  };
}

async function returnToRoot(): Promise<void> {
  const rootBreadcrumb = await waitFor(
    () => document.querySelector<HTMLButtonElement>(".analyze-breadcrumbs button"),
    "the root breadcrumb",
  );
  const committed = nextCommit();
  rootBreadcrumb.click();
  await committed;
  await waitFor(findWideTile, "the root-directory treemap");
}

function nearestRank95(samples: readonly number[]): { readonly rank: number; readonly value: number } {
  const sorted = [...samples].sort((left, right) => left - right);
  const rank = Math.ceil(0.95 * sorted.length);
  return { rank, value: sorted[rank - 1] };
}

async function runBenchmark(rootElement: HTMLElement): Promise<AnalyzeRenderBenchmarkResult> {
  const startButton = await waitFor(
    () => document.querySelector<HTMLButtonElement>(".analyze-toolbar .primary-button"),
    "the Analyze start button",
  );
  startButton.click();
  await waitFor(() => document.querySelector<HTMLElement>(".analyze-workspace"), "the completed Analyze snapshot");
  await animationFrame();

  const measured: NavigationMeasurement[] = [];
  for (let index = 0; index < WARMUPS + MEASURED_NAVIGATIONS; index += 1) {
    const sample = await drillIntoWideDirectory(rootElement);
    if (index >= WARMUPS) measured.push(sample);
    await returnToRoot();
  }

  const durations = measured.map((sample) => sample.durationMs);
  const p95 = nearestRank95(durations);
  const rectangleCount = Math.max(...measured.map((sample) => sample.rectangles));
  const domElements = Math.max(...measured.map((sample) => sample.domElements));
  const passed = p95.value <= COMMIT_LIMIT_MS
    && rectangleCount <= RECTANGLE_LIMIT
    && domElements <= DOM_ELEMENT_LIMIT;
  return {
    status: passed ? "pass" : "budget_miss",
    fixture: "analysis-250k-v1-equivalent-wide-directory",
    fixture_total_nodes: FIXTURE_TOTAL_NODES,
    current_directory_children: CURRENT_DIRECTORY_CHILDREN,
    fixture_generation: "250,000 represented nodes with one deterministic 10,000-file wide directory; wide child id/name/bytes match the pure-layout fixture",
    fixture_children_sha256: await sha256(JSON.stringify(wideSeeds)),
    warmups: WARMUPS,
    measured_navigations: measured.length,
    nearest_rank: p95.rank,
    react_commit_samples_ms: durations,
    react_commit_p95_ms: p95.value,
    rectangle_count_max: rectangleCount,
    dom_elements_max: domElements,
    limits: {
      rectangle_count_max: RECTANGLE_LIMIT,
      dom_elements_max: DOM_ELEMENT_LIMIT,
      react_commit_p95_ms_max: COMMIT_LIMIT_MS,
    },
    measurement_method: "For each real SVG drill-in event, performance.now() starts immediately before dispatch. AnalyzePage's post-commit useLayoutEffect calls onPerformanceSample; the harness records performance.now() at that callback. The interval includes React event handling, selector/layout render work, and DOM commit, but excludes the following paint.",
    build: import.meta.env.PROD ? "Vite production benchmark build" : "Vite development server",
    browser_webview: navigator.userAgent,
    platform: navigator.platform,
    hardware_concurrency: Number.isFinite(navigator.hardwareConcurrency) ? navigator.hardwareConcurrency : null,
    memory_counts: null,
    notes: [
      "DOM elements are counted under #root immediately after each measured commit.",
      "Only the 30 drill-in commits after five warm-ups are included; root-return commits are synchronization steps and are not sampled.",
      "The harness mounts the production AnalyzePage component without React StrictMode and uses the product OperationCoordinator.",
    ],
  };
}

const rootElement = document.getElementById("root");
const statusElement = document.getElementById("benchmark-status");
const outputElement = document.getElementById("benchmark-output");
if (!rootElement || !statusElement || !outputElement) throw new Error("Benchmark host elements are missing");

const snapshot = fixture();
ReactDOM.createRoot(rootElement).render(
  <AnalyzePage
    bridge={benchmarkBridge(snapshot)}
    coordinator={new OperationCoordinator()}
    locale="en"
    initialRoot={snapshot.root.normalized}
    onPerformanceSample={onPerformanceSample}
  />,
);

statusElement.textContent = "Running 5 warm-ups and 30 measured Analyze drill-ins…";
window.__DEVSWEEP_ANALYZE_BENCHMARK_PROMISE__ = runBenchmark(rootElement)
  .then((result) => {
    window.__DEVSWEEP_ANALYZE_BENCHMARK_RESULT__ = result;
    const json = JSON.stringify(result, null, 2);
    statusElement.textContent = result.status === "pass" ? "Analyze render benchmark passed." : "Analyze render benchmark missed a budget.";
    outputElement.textContent = json;
    console.info(`DEVSWEEP_ANALYZE_BENCHMARK ${JSON.stringify(result)}`);
    return result;
  })
  .catch((error: unknown) => {
    const message = error instanceof Error ? error.message : String(error);
    const failed: AnalyzeRenderBenchmarkResult = {
      status: "error",
      fixture: "analysis-250k-v1-equivalent-wide-directory",
      fixture_total_nodes: FIXTURE_TOTAL_NODES,
      current_directory_children: CURRENT_DIRECTORY_CHILDREN,
      fixture_generation: "250,000 represented nodes with one deterministic 10,000-file wide directory",
      fixture_children_sha256: "UNAVAILABLE: benchmark failed before hashing completed",
      warmups: WARMUPS,
      measured_navigations: 0,
      nearest_rank: 29,
      react_commit_samples_ms: [],
      react_commit_p95_ms: Number.NaN,
      rectangle_count_max: 0,
      dom_elements_max: 0,
      limits: {
        rectangle_count_max: RECTANGLE_LIMIT,
        dom_elements_max: DOM_ELEMENT_LIMIT,
        react_commit_p95_ms_max: COMMIT_LIMIT_MS,
      },
      measurement_method: "Benchmark did not complete.",
      build: import.meta.env.PROD ? "Vite production benchmark build" : "Vite development server",
      browser_webview: navigator.userAgent,
      platform: navigator.platform,
      hardware_concurrency: Number.isFinite(navigator.hardwareConcurrency) ? navigator.hardwareConcurrency : null,
      memory_counts: null,
      notes: [message],
    };
    window.__DEVSWEEP_ANALYZE_BENCHMARK_RESULT__ = failed;
    statusElement.textContent = `Analyze render benchmark failed: ${message}`;
    outputElement.textContent = JSON.stringify(failed, null, 2);
    console.error(`DEVSWEEP_ANALYZE_BENCHMARK_ERROR ${JSON.stringify(failed)}`);
    return failed;
  });
