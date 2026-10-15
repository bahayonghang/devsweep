# Clean home performance evidence

Status: **MEASURED** on 2026-10-14 at 17:34 UTC, using the frozen production build after the existing review benchmark had finished, with native measurement paused and no concurrent compilation. All three operations retained the required 30 samples after five warmups. Retention and DOM bounds passed; no new latency budget is asserted. Typecheck, scoped ESLint and its production Vite build passed. Build log: `dist/cleanup-home-final/gates/home-benchmark-build.log`.

The production `CleanOverview` and its `ProjectAdvisories` render a controlled, closed-decoded API-shaped report with 10,000 distinct project owners and 10,000 Rust build targets. Every target uses the current `rust.target` / `cargo` / `clean_manifest` combination, is unselected, and has a 6 GiB exact display estimate. The test projection starts with a 5 GiB warning threshold. Its display report identity is intentionally synthetic; this harness never imports the native bridge, claims cleanup authority, or creates files matching those paths.

Each operation uses five warmups followed by 30 samples: a fresh React mount of the completed home, a real owner pager click, and a real threshold form submission changing between 5 and 8 GiB with an incoming simulated 10,000-owner projection. Raw mutation-commit and two-RAF paint-opportunity times are retained. p95 uses nearest rank. Threshold samples include the controlled projection mapping and closed decoding; IPC, Rust aggregation, native window composition and OS input scheduling are excluded.

The harness checks that the final live state retains all 10,000 report targets and 10,000 advisory owners while only 10 owner articles are mounted. No new latency budget is introduced. The existing Clean review/preview benchmark fixture, method and frozen budgets remain unchanged and are evaluated separately by the parent.

Reproduce the measured preserved build with Node 22.23.2 and a new evidence output directory (no build or dependency installation needed):

```powershell
$node22 = 'C:/Users/lyh/AppData/Local/mise/installs/node/22.23.2/node.exe'
# From repository root:
& $node22 tools/clean-home-benchmark.mjs dist/cleanup-home-final/home-performance-new acceptance dist/cleanup-home-final/benchmark-builds/clean-home
```

For a future source rebuild, run typecheck and scoped ESLint from `desktop`, then `node node_modules/vite/bin/vite.js build --config vite.clean-home-benchmark.config.ts --mode acceptance --outDir ../dist/clean-home-reproduction`. Keep benchmark output outside `desktop/dist` before a native build: Tauri's `frontendDist` embeds that entire directory. The final run used the parent's preserved external build and did not rebuild or write `desktop/dist`.

The driver uses an isolated profile, loopback static server, owned hidden Chrome process, 1440×900 CSS viewport and emulated DPR 1.25. Dark theme is explicitly selected before sampling. After all samples finished, it also captured the oversized state at 1350×900 / DPR 1; that supplemental fixture screenshot is outside the measurement window. Browser exit and owned-process teardown are recorded.

## Results

| Metric | Current result |
| --- | --- |
| First completed home view p95 | 14.0 ms |
| Owner page change p95 | 14.1 ms |
| Threshold/projection refresh p95 | 14.2 ms |
| Raw sample count per operation | 30 after 5 warmups |
| Final retained targets / owners | 10,000 / 10,000 |
| Mounted owner articles | 10; 218 total DOM elements |
| Owned browser lifecycle | Normal exit 0; PID 119776; no forced termination |

Raw artifacts are `10000-home.json`, `record.json`, `measured-home.png`, `oversized-home.png` and `teardown.json` under `dist/cleanup-home-final/home-performance/`. This was the first and only formal home run. `home-performance-manifest.json` records artifact/build/harness hashes and independently recomputes the three nearest-rank p95 values from all samples. Every sample has nonnegative commit time no greater than its paint-opportunity time.

The embedded frontend source SHA (`36826f8d…42dcf39`) exactly matches a fresh hash of current frontend source and i18n files. Fixture SHA is `e7e291b4…d53347`. Host: Windows 10.0.26300, Intel Core Ultra 9 275HX, 24 logical processors, Node 22.23.2, Chrome 155.0.0.0. Full values are in the manifest and raw record.

`oversized-home.png` was visually inspected: the current 5 GiB threshold, distinct owner paths, 6.0 GiB build/cache subtotal, exact 0 B dependencies and above-threshold warning are readable; no target is automatically selected. This harness mounts the production home components without the shared shell. It does not prove native warning/cleanup behavior or a before/after homepage latency improvement.
