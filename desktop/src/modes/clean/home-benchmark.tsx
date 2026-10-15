import { useLayoutEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { decodeCleanAdvisories, decodeScanReport } from "../../api/contract";
import type { CleanAdvisoriesV1, ScanReport } from "../../api/types.gen";
import { CleanOverview } from "./CleanOverview";
import { initialCleanSession } from "./session-state";
import "../../styles.css";
import "./styles.css";

declare const __CLEAN_HOME_BUILD__: { label: string; source_sha256: string };
const WARMUPS = 5, SAMPLES = 30, COUNT = 10000, GIB = 1024 ** 3;
type Session = Parameters<typeof CleanOverview>[0]["session"];
let root = createRoot(document.getElementById("root")!);
let current: Session | null = null;
let currentAdvisory: CleanAdvisoriesV1 | null = null;
const frame = () => new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
const p95 = (values: number[]) => [...values].sort((a, b) => a - b)[Math.ceil(values.length * .95) - 1];
const owner = (index: number) => `C:/clean-home-fixture/project-${String(index).padStart(5, "0")}-空间`;
const targetId = (index: number) => `rust.target:${owner(index)}/target`;
const noOp = () => undefined;

/** Controlled display data shaped and closed-decoded as the Rust API contract.
 * This is not scanner/owner/digest authority, and no native bridge is imported. */
function fixtures(): { report: ScanReport; advisory: CleanAdvisoriesV1 } {
  const report = decodeScanReport({ version: 1, plan: { version: 2, targets: Array.from({ length: COUNT }, (_, index) => ({
    id: targetId(index), rule_id: "rust.target", scope: { type: "project", root: owner(index) }, ecosystem: "rust", kind: "build_artifacts",
    path: `${owner(index)}/target`, estimated_bytes: 6 * GIB, size_complete: true, sizing_warnings: [], last_modified: null,
    risk: "low", reversible: false, selected_by_default: false, intent: { type: "run_built_in_action", provider_id: "cargo", action_id: "clean_manifest" },
    evidence: [{ type: "marker_file", path: `${owner(index)}/Cargo.toml` }, { type: "rule_matched", rule_id: "rust.target" }],
  })) }, health: { completeness: "complete", totals: { verified_bytes: COUNT * 6 * GIB, partial_lower_bound_bytes: 0, unknown_target_count: 0 }, diagnostics: [], coverage: [] } });
  const advisory = decodeCleanAdvisories({ version: 1, report_id: "sha256:" + "a".repeat(64), threshold_bytes: 5 * GIB,
    included_kinds: ["build_artifacts", "test_cache", "tool_cache"], projects: Array.from({ length: COUNT }, (_, index) => ({
      owner_root: owner(index), artifacts: { known_bytes: 6 * GIB, confidence: "exact", target_ids: [targetId(index)] },
      dependencies: { known_bytes: 0, confidence: "exact", target_ids: [] }, excluded_target_ids: [], assessment: "oversized",
    })) });
  return { report, advisory };
}

function Surface({ report, advisory }: { report: ScanReport; advisory: CleanAdvisoriesV1 }) {
  const [snapshot, setSnapshot] = useState({ threshold: 5, advisory });
  const session: Session = {
    slots: { ...initialCleanSession, project: { state: { ...initialCleanSession.project.state, phase: "reviewed", scan: report }, revision: 1, completedAt: "2026-10-14T12:00:00Z", stale: false } },
    dispatch: noOp,
    scope: { options: { include_global: false, include_projects: true, roots: ["C:/clean-home-fixture"] }, suggestions: [], loading: false, saving: false, error: null, sessionOnly: false, dismissError: noOp, changeOptions: noOp, changeRoots: async () => false },
    advisory: { threshold: snapshot.threshold, sessionOnly: false, saving: false, save: async threshold => {
      // Simulated incoming authoritative projection, not frontend occupancy aggregation.
      const next = decodeCleanAdvisories({ ...advisory, threshold_bytes: threshold * GIB,
        projects: advisory.projects.map(project => ({ ...project, assessment: threshold === 5 ? "oversized" : "below_threshold" })) });
      setSnapshot({ threshold, advisory: next });
    } },
  };
  useLayoutEffect(() => { current = session; currentAdvisory = snapshot.advisory; });
  return <main className="clean-mode clean-workbench"><CleanOverview session={session} locale="en" busy={false} scan={noOp} cancel={noOp} review={noOp} preview={noOp} dismiss={noOp} assessment={{ value: snapshot.advisory, failed: false }} /></main>;
}

async function update(run: () => void, expected: () => boolean) {
  await frame();
  const began = performance.now();
  let commitMs = 0;
  await new Promise<void>((resolve, reject) => {
    const deadline = setTimeout(() => { observer.disconnect(); reject(Error("Production overview did not render in 20 seconds")); }, 20000);
    const observer = new MutationObserver(() => {
      if (expected()) { commitMs = performance.now() - began; clearTimeout(deadline); observer.disconnect(); resolve(); }
    });
    observer.observe(document.getElementById("root")!, { childList: true, subtree: true, characterData: true, attributes: true });
    run();
  });
  await frame(); await frame();
  return { commit_ms: commitMs, visible_ms: performance.now() - began };
}

async function run() {
  const { report, advisory } = fixtures();
  const fixtureWire = JSON.stringify({ report, advisory });
  const fixtureSha = [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(fixtureWire)))].map(value => value.toString(16).padStart(2, "0")).join("");
  const mounts = [], pages = [], refreshes = [];
  for (let index = 0; index < WARMUPS + SAMPLES; index++) {
    root.unmount(); root = createRoot(document.getElementById("root")!);
    const measured = await update(() => root.render(<Surface report={report} advisory={advisory} />), () => document.querySelectorAll(".clean-owner").length === 10);
    if (index >= WARMUPS) mounts.push(measured);
  }
  for (let index = 0; index < WARMUPS + SAMPLES; index++) {
    const next = index % 2 === 0;
    const measured = await update(() => {
      const buttons = document.querySelectorAll<HTMLButtonElement>(".clean-owner-list .target-pagination button");
      buttons[next ? 1 : 0].click();
    }, () => document.querySelector(".clean-owner h3")?.textContent === owner(next ? 10 : 0));
    if (index >= WARMUPS) pages.push(measured);
  }
  document.querySelector<HTMLDetailsElement>(".clean-threshold-settings")!.open = true;
  for (let index = 0; index < WARMUPS + SAMPLES; index++) {
    const threshold = index % 2 === 0 ? 8 : 5;
    const measured = await update(() => {
      document.querySelector<HTMLInputElement>("#clean-threshold")!.value = String(threshold);
      document.querySelector<HTMLFormElement>(".clean-threshold")!.requestSubmit();
    }, () => document.querySelector(".clean-threshold-settings summary")?.textContent === `Capacity warning: ${threshold} GiB`
      && document.querySelectorAll(`.clean-assessment.${threshold === 5 ? "oversized" : "below_threshold"}`).length === 10);
    if (index >= WARMUPS) refreshes.push(measured);
  }
  if (current?.slots.project.state.scan?.plan.targets.length !== COUNT || currentAdvisory?.projects.length !== COUNT || document.querySelectorAll(".clean-owner").length !== 10) throw Error("Full-report retention or owner DOM bound violated");
  return { build: __CLEAN_HOME_BUILD__, fixture_sha256: fixtureSha, fixture_version: "clean-home-v1", warmups: WARMUPS, measured_samples: SAMPLES,
    percentile: "nearest-rank p95", mount: mounts, page: pages, threshold_refresh: refreshes,
    p95_ms: { first_view: p95(mounts.map(value => value.visible_ms)), page: p95(pages.map(value => value.visible_ms)), threshold_refresh: p95(refreshes.map(value => value.visible_ms)) },
    retained_targets: current.slots.project.state.scan.plan.targets.length, retained_owners: currentAdvisory.projects.length,
    rendered_owners: document.querySelectorAll(".clean-owner").length, dom_elements: document.querySelectorAll("*").length,
    browser: navigator.userAgent, logical_processors: navigator.hardwareConcurrency, dpr: devicePixelRatio,
    budgets: null, acceptance: "Measurement only; no new latency budget is asserted. Retention and mounted-owner bounds are checked.",
    method: "Production CleanOverview/ProjectAdvisories. Five warmups plus 30 samples for each first mount, real pager click, and real threshold form submit with closed-decoded simulated incoming 10k projection. Mutation commit plus two RAF paint opportunities. No IPC/core computation or OS input scheduling is measured; no cleanup bridge.",
  };
}
declare global { interface Window { cleanHomeBenchmark: { run: typeof run } } }
window.cleanHomeBenchmark = { run };
