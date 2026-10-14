// Same product layout fixture and 5 + 30 sample protocol as measure-resources.ps1.
// Node timing only; this does not claim native WebView or React commit latency.
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [sourceArg, outputArg] = process.argv.slice(2);
if (!sourceArg || !outputArg) throw new Error("usage: node --experimental-strip-types tools/analyze-layout-benchmark.mjs <treemap.ts> <new-result.json>");
const source = resolve(sourceArg), { layoutTreemap } = await import(pathToFileURL(source).href);
const children = Array.from({ length: 10000 }, (_, index) => ({
  id: index + 2, parent_id: 1, name: `wide-${index.toString().padStart(5, "0")}.bin`,
  kind: "file", bytes: (index % 97) + 1, evidence: "complete", completeness: "complete", immediate_count: 0,
}));
const samples = [], warmups = [];
let rectangleCount = 0;
for (let index = 0; index < 35; index++) {
  const started = performance.now();
  const tiles = layoutTreemap(children, { width: 1200, height: 700 });
  const elapsed = performance.now() - started;
  rectangleCount = tiles.length;
  (index >= 5 ? samples : warmups).push(elapsed);
}
const sorted = [...samples].sort((a, b) => a - b), rank = Math.ceil(0.95 * sorted.length);
const result = {
  method: "Node product layoutTreemap; 5 discarded warmups and 30 samples; nearest-rank p95; OS file cache uncontrolled",
  node: process.version, source, source_sha256: createHash("sha256").update(readFileSync(source)).digest("hex"),
  fixture_children_sha256: `sha256:${createHash("sha256").update(JSON.stringify(children)).digest("hex")}`,
  warmup_samples_ms: warmups, layout_samples_ms: samples, nearest_rank: rank,
  layout_p95_ms: sorted[rank - 1], rectangle_count: rectangleCount,
  pass: rectangleCount <= 513 && sorted[rank - 1] <= 50,
};
writeFileSync(resolve(outputArg), JSON.stringify(result, null, 2), { flag: "wx" });
console.log(JSON.stringify(result));
if (!result.pass) process.exitCode = 1;
