# Final C2 production component measurements

Measured 2026-10-13 against the final C2 Planet, including the native presentation
visibility bridge. This is a production-browser component comparison. Draw
callbacks include synchronous CPU work and GPU submission, not GPU completion
or whole-application CPU. Native process-tree comparisons are recorded by the
runtime child separately.

## Results

| Measure | Original Planet | C2 geographic Planet |
| --- | ---: | ---: |
| Cold first draw submission p95, 20 fresh documents | 69.3 ms | 24.9 ms |
| Cold observed tasks at least 50 ms | 20 / 20 | 0 / 20 |
| Warm painted callback p95, five windows | 1 / 1 / 1 / 1 / 0.9 ms | 0.2 / 0.1 / 0.1 / 0.1 / 0.1 ms |
| Median synchronous painted work per 30 seconds | 732.8 ms | 46.8 ms |
| Warm frame gaps exceeding twice the 30 FPS interval | 0 / 4315 | 0 / 4316 |
| Cancel visible response p95, 100 aligned events | 7.7 ms | 7.0 ms |
| Navigation visible response p95, 100 aligned events | 7.0 ms | 7.0 ms |

The median synchronous painted-work reduction is **93.6%**. Every warm window
contains at least 500 painted callbacks. Both builds' twenty unmount/remount
cycles recorded zero owned RAF handles and zero later draws while unmounted.
The cold, draw, cadence and both interaction budgets pass on this host. This
does not claim a 93.6% whole-app CPU, GPU time or energy improvement.

## Fixed methods and identities

The full comparison uses 20 fresh document/module/React contexts, a 3-second
warm-up and five 30-second animated windows per build. Viewport is 1440x900 CSS
pixels at DPR 2. Both builds run in one isolated Chrome instance with identical
media settings; original precedes candidate. The separate 100-event Cancel
check reverses order. Percentiles use nearest rank. These are document-cold
samples, not twenty fresh OS processes or an OS-cache-cleared benchmark.
The cold timer starts in the loaded harness before React mounts and ends at
the first instrumented Earth draw call; it excludes module fetching and does
not establish the GPU presentation timestamp.

The final navigation extension uses the real production AppShell, ScanPage and
OperationCoordinator with the same deterministic progress replay. It clicks
the registered Software tab while Clean is active and records the first RAF
after a static destination sentinel commits and Planet has unmounted. Reset
and first Earth drawing occur outside the timed interval. Five warm-up events
precede 100 measured events per build, candidate then original. All 200 measured
destination assertions passed. The destination does not run Software inventory;
native backend cancellation/join and actual operation latency remain separate.

The full and Cancel runs use the frozen assets in
`dist/performance/cleanup-earth/planet-C2-build/`; their HTML hashes were checked
against the recorded run. Navigation-only harness changes followed those runs;
its two builds are preserved in `planet-C2-navigation-build/` (144 input hashes,
14 assets, manifest SHA-256
`d9cb835f6bfd0f87228a88bba7df0f454858ffd0333f688e42fdc63f58751ce7`).
The extension changes no packaged application module or native binary. Original
Planet sources come from `e75a29d`; shared production controls are identical in
both comparison builds. Renderer source hashes, browser/GPU/host details,
individual cold and interaction samples, warm-window summaries and all unmount
samples are in [renderer-c2-summary.json](renderer-c2-summary.json).

Raw records and their hashes are retained at:

- `dist/performance/cleanup-earth/planet-C2-full-02/`
- `dist/performance/cleanup-earth/planet-C2-interaction-01/`
- `dist/performance/cleanup-earth/planet-C2-navigation-01/`

All three owned browsers closed normally with exit 0 and no forced termination.
There was no concurrent build/test/native benchmark during their timed windows.
Node 22.23.2, Vite 7.3.6 and the installed Chrome were used. Navigation harness
TypeScript, both production builds and final scoped ESLint passed. Independent
review recomputed both interaction gates and verified raw/build hashes and
normal teardown. Repository whitespace check passed.

## Retained failures and limits

`planet-C2-full-01/` failed before collecting samples: sandboxed Chrome exited
2147483651 and the CDP connection closed. Its failed teardown and explanation
remain. The fresh-directory retry used the approved normal execution context,
with no product or benchmark source change.

Earlier renderer and interaction records remain in
[renderer-measurements.md](renderer-measurements.md), including the initial
20-event unaligned relative interaction failure. They are historical evidence,
not silently replaced or counted as final C2 measurements. Native minimize,
Windows scale, DPR emulation limitations and memory/handle observations are
reported in the separate native acceptance record.
