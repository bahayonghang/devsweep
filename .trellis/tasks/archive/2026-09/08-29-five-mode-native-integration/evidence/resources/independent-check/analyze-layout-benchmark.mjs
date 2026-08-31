import { createHash } from "node:crypto";
import { performance } from "node:perf_hooks";
import { pathToFileURL } from "node:url";

const mod = await import(pathToFileURL(process.argv[2]).href);
const layoutTreemap = mod.layoutTreemap;
const children = Array.from({ length: 10000 }, (_, index) => ({
  id: index + 2,
  parent_id: 1,
  name: `wide-${index.toString().padStart(5, "0")}.bin`,
  kind: "file",
  bytes: (index % 97) + 1,
  evidence: "complete",
  completeness: "complete",
  immediate_count: 0,
}));
const fixtureHash = `sha256:${createHash("sha256").update(JSON.stringify(children)).digest("hex")}`;
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
process.stdout.write(JSON.stringify({
  layout_p95_ms: layoutP95,
  nearest_rank: rank,
  layout_samples_ms: samples,
  rectangle_count: rectangleCount,
  fixture_children_sha256: fixtureHash,
  pass: rectangleCount <= 513 && layoutP95 <= 50,
}));