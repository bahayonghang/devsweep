import { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { DesktopPreferencesContext } from "../preferences/context";
import { DEFAULT_DESKTOP_PREFERENCES } from "../preferences/store";
import { applyAppearance } from "../preferences/appearance";
import { OperationCoordinator } from "../state/operation-coordinator";
import { ScanPage } from "../pages/ScanPage";
import { AppShell } from "../app-shell/AppShell";
import { decodeDesktopScanProgress } from "../api/contract";
import progressFixture from "../api/fixtures/scan-progress.json";
import type { ActiveScan } from "../state/app-state";
import type { DesktopPreferencesV2 } from "../api/types.gen";
import "../styles.css";

declare const __PLANET_BUILD__: { mode: string; baseline: string | null; source_sha256: string };

const query = new URLSearchParams(location.search);
const preferences: DesktopPreferencesV2 = {
  ...DEFAULT_DESKTOP_PREFERENCES,
  motion: query.get("still") === "1" ? "reduced" : "system",
  planet_fps: query.get("fps") === "15" ? 15 : 30,
  theme: query.get("theme") === "light" ? "light" : "dark",
};
applyAppearance(document.documentElement, preferences, false, false);
const locale = query.get("locale") === "zh-CN" ? "zh-CN" : "en";
const started = performance.now();
const draws: number[] = [];
const durations: number[] = [];
const longTasks: { start: number; duration: number }[] = [];
const ownedFrames = new Set<number>();
let firstFrame: number | null = null;
let sourceDiameter: number | null = null;
let gpu: { vendor: string; renderer: string } | null = null;
const originalDraw = CanvasRenderingContext2D.prototype.drawImage;
CanvasRenderingContext2D.prototype.drawImage = function (this: CanvasRenderingContext2D, ...args: unknown[]) {
  Reflect.apply(originalDraw, this, args);
  if (!this.canvas.classList.contains("planet-canvas")) return;
  const now = performance.now();
  firstFrame ??= now - started;
  draws.push(now);
  if (args[0] instanceof HTMLCanvasElement) sourceDiameter = args[0].width;
};
const originalGpuDraw = WebGLRenderingContext.prototype.drawArrays;
WebGLRenderingContext.prototype.drawArrays = function (...args: Parameters<typeof originalGpuDraw>) {
  originalGpuDraw.apply(this, args);
  if (!(this.canvas instanceof HTMLCanvasElement) || !this.canvas.classList.contains("planet-canvas")) return;
  const now = performance.now();
  firstFrame ??= now - started;
  draws.push(now);
  sourceDiameter = this.canvas.width;
  if (!gpu) {
    const debug = this.getExtension("WEBGL_debug_renderer_info");
    gpu = { vendor: String(this.getParameter(debug?.UNMASKED_VENDOR_WEBGL ?? this.VENDOR)), renderer: String(this.getParameter(debug?.UNMASKED_RENDERER_WEBGL ?? this.RENDERER)) };
  }
};
const request = window.requestAnimationFrame.bind(window);
const cancel = window.cancelAnimationFrame.bind(window);
window.requestAnimationFrame = (callback) => {
  const id = request((now) => {
    ownedFrames.delete(id);
    const before = draws.length;
    const start = performance.now();
    callback(now);
    if (draws.length > before) durations.push(performance.now() - start);
  });
  ownedFrames.add(id);
  return id;
};
window.cancelAnimationFrame = (id) => { ownedFrames.delete(id); cancel(id); };
new PerformanceObserver((entries) => {
  for (const item of entries.getEntries()) longTasks.push({ start: item.startTime, duration: item.duration });
}).observe({ type: "longtask", buffered: true });

const coordinator = new OperationCoordinator();
let settle: (() => void) | null = null;
let show: (mounted: boolean) => void = () => undefined;
let replay: (enabled: boolean) => void = () => undefined;
const interactions: number[] = [];
let interactionStarted: number | null = null;
const navigationSamples: { duration_ms: number; destination_committed: boolean }[] = [];
let navigationStarted: number | null = null;
function NavigationDestination() {
  useEffect(() => {
    if (navigationStarted === null) return;
    const start = navigationStarted;
    navigationStarted = null;
    request(() => navigationSamples.push({
      duration_ms: performance.now() - start,
      destination_committed: Boolean(document.querySelector("[data-navigation-destination]"))
        && !document.querySelector(".planet"),
    }));
  }, []);
  return <p data-navigation-destination>Navigation destination</p>;
}
function Harness() {
  const [mounted, setMounted] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [sequence, setSequence] = useState(1);
  const [canceled, setCanceled] = useState(false);
  useEffect(() => {
    show = setMounted;
    replay = (enabled) => { setCanceled(false); setScanning(enabled); };
  }, []);
  useEffect(() => {
    if (!scanning) return;
    const timer = setInterval(() => setSequence((value) => value + 1), 100);
    return () => clearInterval(timer);
  }, [scanning]);
  useEffect(() => {
    if (canceled && interactionStarted !== null) {
      const start = interactionStarted;
      request(() => interactions.push(performance.now() - start));
      interactionStarted = null;
    }
  }, [canceled]);
  const progress = decodeDesktopScanProgress({ ...progressFixture, sequence });
  const activeScan: ActiveScan | null = scanning ? { scanId: progress.scan_id, lastSequence: sequence, progress, preview: progress.preview, cancelRequested: canceled } : null;
  const scanPage = <ScanPage locale={locale} activeScan={activeScan} busy={scanning} options={{ roots: ["C:/benchmark/project"], include_projects: true, include_global: true }} onOptions={() => undefined} onScan={() => undefined} onCancel={() => {
    interactionStarted = performance.now();
    setCanceled(true);
    void coordinator.cancelAndJoin();
  }} />;
  return <DesktopPreferencesContext.Provider value={{ preferences, loading: false, unavailable: false, saving: false, saveError: false, update: async () => null, reload: () => undefined }}>
    {mounted ? query.get("navigation") === "1"
      ? <AppShell locale={locale} onLocaleChange={() => undefined} coordinator={coordinator} modes={[
        { id: "clean", render: () => scanPage },
        { id: "software", render: () => <NavigationDestination /> },
      ]} />
      : scanPage : null}
  </DesktopPreferencesContext.Provider>;
}

const benchmark = {
  snapshot: () => ({
    build: __PLANET_BUILD__, first_frame_ms: firstFrame, source_diameter: sourceDiameter,
    output_diameter: document.querySelector("canvas")?.width ?? null,
    draw_timestamps_ms: [...draws], painted_raf_durations_ms: [...durations],
    long_tasks: [...longTasks], owned_frames: ownedFrames.size,
    interaction_ms: [...interactions], navigation_samples: [...navigationSamples], dpr: devicePixelRatio,
    motion: preferences.motion, os_reduced_motion: matchMedia("(prefers-reduced-motion: reduce)").matches,
    viewport: { width: innerWidth, height: innerHeight }, browser: navigator.userAgent,
    hardware_concurrency: navigator.hardwareConcurrency, visibility: document.visibilityState,
    render_state: document.querySelector(".planet")?.getAttribute("data-render-state"),
    gpu,
    memory: (() => {
      const memory = (performance as Performance & { memory?: { usedJSHeapSize: number; totalJSHeapSize: number; jsHeapSizeLimit: number } }).memory;
      return memory ? { used: memory.usedJSHeapSize, total: memory.totalJSHeapSize, limit: memory.jsHeapSizeLimit } : null;
    })(),
    method: "Real production Stage/ScanPage/Planet and preferences. RAF callback wall time for callbacks that paint; JS submission, not GPU completion. Frame-aligned Cancel records the first post-commit RAF presentation opportunity. Read-only deterministic progress replay; no backend scan or cleanup.",
  }),
  reset: () => { draws.length = durations.length = longTasks.length = 0; },
  mount: (mounted: boolean) => show(mounted),
  async scan() {
    const lease = await coordinator.start({ kind: "clean.scan", id: "planet-replay", cancel: () => { settle?.(); }, start: () => new Promise<void>((resolve) => { settle = resolve; }) });
    void lease?.result.then(() => lease.complete());
    replay(true);
  },
  cancel() { return new Promise<void>((resolve) => request(() => {
    (document.querySelector(".stage-action") as HTMLButtonElement | null)?.click();
    resolve();
  })); },
  navigate() { return new Promise<void>((resolve, reject) => request(() => {
    const button = document.getElementById("primary-tab-software") as HTMLButtonElement | null;
    if (!button || button.disabled) { reject(new Error("Navigation control unavailable")); return; }
    navigationStarted = performance.now();
    button.click();
    resolve();
  })); },
  async home() {
    if (document.getElementById("primary-tab-clean")?.getAttribute("aria-selected") === "true"
      && document.querySelector(".planet")) return;
    const previousDraws = draws.length;
    (document.getElementById("primary-tab-clean") as HTMLButtonElement | null)?.click();
    const deadline = performance.now() + 3000;
    await new Promise<void>((resolve, reject) => {
      const check = () => {
        if (document.getElementById("primary-tab-clean")?.getAttribute("aria-selected") === "true"
          && document.querySelector(".planet") && draws.length > previousDraws) resolve();
        else if (performance.now() >= deadline) reject(new Error("Clean route did not settle"));
        else request(check);
      };
      check();
    });
  },
};
declare global { interface Window { planetBenchmark: typeof benchmark } }
window.planetBenchmark = benchmark;
ReactDOM.createRoot(document.getElementById("root")!).render(<Harness />);
