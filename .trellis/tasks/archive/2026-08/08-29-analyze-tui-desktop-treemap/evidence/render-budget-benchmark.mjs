import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { performance } from "node:perf_hooks";

import { layoutTreemap } from "../../../../desktop/src/modes/analyze/treemap.ts";

const children = Array.from({ length: 10_000 }, (_, index) => ({
  id: index + 2,
  parent_id: 1,
  name: `wide-${index.toString().padStart(5, "0")}.bin`,
  kind: "file",
  bytes: (index % 97) + 1,
  evidence: "complete",
  completeness: "complete",
  immediate_count: 0,
}));
const fixtureHash = `sha256:${createHash("sha256")
  .update(JSON.stringify(children))
  .digest("hex")}`;
let rustc = null;
try {
  rustc = execFileSync("rustc", ["--version"], { encoding: "utf8" }).trim();
} catch {
  // Host metadata is evidence only; layout measurement remains executable when
  // Rust is not installed on a browser-only benchmark host.
}

const samples = [];
let rectangleCount = 0;
for (let index = 0; index < 35; index += 1) {
  const started = performance.now();
  const tiles = layoutTreemap(children, { width: 1200, height: 700 });
  const elapsed = performance.now() - started;
  rectangleCount = tiles.length;
  if (index >= 5) samples.push(elapsed);
}

const sorted = [...samples].sort((left, right) => left - right);
const rank = Math.ceil(0.95 * sorted.length);
const layoutP95 = sorted[rank - 1];
const layoutPassed = rectangleCount <= 513 && layoutP95 <= 50;
console.log(
  JSON.stringify(
    {
      status: layoutPassed ? "layout_pass_browser_pending" : "stop_budget_miss",
      command: "node --experimental-strip-types .trellis/tasks/08-29-analyze-tui-desktop-treemap/evidence/render-budget-benchmark.mjs",
      reproduction_command: "node --experimental-strip-types .trellis/tasks/08-29-analyze-tui-desktop-treemap/evidence/render-budget-benchmark.mjs",
      exit_code: 0,
      fixture: "analysis-250k-v1-equivalent-wide-directory",
      fixture_total_nodes: 250_000,
      current_directory_children: children.length,
      fixture_generation:
        "10,000 deterministic file children; id=index+2; name=wide-NNNNN.bin; bytes=(index%97)+1; complete evidence",
      fixture_children_sha256: fixtureHash,
      bounds_css_px: { width: 1200, height: 700 },
      warmups: 5,
      measured_navigations: samples.length,
      nearest_rank: rank,
      layout_samples_ms: samples,
      layout_p95_ms: layoutP95,
      limits: {
        rectangle_count_max: 513,
        dom_elements_max: 900,
        layout_p95_ms_max: 50,
        react_commit_p95_ms_max: 100,
      },
      rectangle_count: rectangleCount,
      dom_elements: null,
      react_commit_samples_ms: null,
      react_commit_p95_ms: null,
      memory_counts: null,
      host: {
        node: process.version,
        platform: `${process.platform}-${process.arch}`,
        rustc,
      },
      build: "PASS outside the implementer sandbox: just desktop-web-check and just desktop-build exit 0 in the existing main-session logs; the new benchmark harness still requires a fresh main-session run",
      webview: "UNVERIFIED: the production browser benchmark harness has not yet run on the release-build host",
      notes: [
        "The pure layout result is measured; browser/DOM/React fields remain null until the release-build harness runs.",
        "Nearest-rank p95 is the 29th sorted sample for n=30.",
        "The rectangle, DOM, layout, and React thresholds are unchanged.",
      ],
    },
    null,
    2,
  ),
);
