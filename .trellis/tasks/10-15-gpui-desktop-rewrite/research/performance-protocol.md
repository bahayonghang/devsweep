# Proposed native performance and evidence protocol

Status: budgets approved with the final planning summary on 2026-10-15; no
measurements performed during task creation. Freeze exact fixture identities before
candidate implementation. A later failed result never licenses relaxed thresholds.

## Why this gate exists

A Rust UI can reduce frontend overhead while backend inventory remains slow.
Measure the entire user-visible workflow and owned process tree, separately from
domain discovery, projection and presentation. Keep actual report equality first.

Historical C3 Software p95 5211.467 ms exceeded 5000 ms; the archived C4 record did
not repeat that formal series. Treat it as a retained historical failure, not the
current reference. Use a newly built current Tauri baseline and a feature-equivalent
GPUI release candidate on the same host and exact workload.

## Proposed release gates

| Metric | Proposed threshold | Method/endpoint |
| --- | --- | --- |
| Process-cold first usable window | p95 <= 1500 ms and <= 80% of fresh Tauri baseline | Create process -> visible interactive native window; record OS file-cache state, no claim of reboot-cold startup. |
| Warm window open/restore | p95 <= 500 ms | Input -> usable content after prior initialization; separate from cold launches. |
| Navigation, filter/select and preview UI response | p95 <= 100 ms; no UI-thread task >= 50 ms in instrumented samples | Native event -> presented result, excluding explicitly displayed backend job duration. |
| 10k-row scrolling/interaction | p95 frame interval <= 16.7 ms at 60 Hz target, explicit stall counts | Production virtualized list, fixed 60-Hz-compatible test configuration. |
| Idle private memory | <= 200 MiB and <= 70% of fresh Tauri baseline | Whole owned tree, 30-s settled idle, same route/data/settings and no debugger. |
| Idle CPU | median <= 0.5% and p95 <= 1.0% of one logical core | Whole owned tree, 200-ms samples over 30-s retained interval; normalize CPU method. |
| Hidden decoration | zero recurring animation callbacks/frames | Main hidden/minimized and tray closed; requested backend work counted separately. |
| Software inventory | p95 <= 5000 ms | Real complete same-source inventory from request to usable complete report; same installed-app manifest/build. |
| Core scan non-regression | median and p95 <= 1.20x baseline on each equal-coverage workload | Full report/exit parity mandatory; retained first-use/warm records. |
| Cooperative cancel/close | UI acknowledgement <= 100 ms; read-only fixture join <= 2000 ms | Excludes documented external operations that cannot safely be canceled; record their actual wait semantics. |
| Lifecycle retention | No monotonic worker/handle growth or orphan child process after 20 cycles | Open/close, route, minimize, tray and sampler cycles; identity-based ownership and post-exit checks. |

These are release objectives, not claims that GPUI currently meets them. If a
reference is already below an absolute limit, the relative improvement objective
still applies to startup/memory. Insufficient resolution is UNVERIFIED, not zero.

## Workloads and sample plan

- Startup: at least 20 retained launches per build, AB/BA alternation, identical
  stores/assets/window size. Distinguish first-use/process-cold/cache-warm.
- Interaction: 5 warmups then >=100 retained actions per scenario on 1k/10k rows,
  including offscreen selection and full-report search.
- Scans: versioned manifest for narrow/wide/deep/overlap/Unicode/denied-root fixtures;
  2 warmups then >=20 retained paired runs per fixture for percentile comparison.
- Software: same source scope and recorded inventory identities. Count changes
  invalidate comparison; no subset/MSIX-only run is a replacement for all sources.
- Analyze: supported large-tree/node-cap fixtures, bounded treemap/list projection;
  record partial coverage and memory cap outcomes rather than omitting them.
- Status: slow pipe, full-queue cancellation, main/tray transitions, missing
  hardware and 20 lifecycle cycles with normal owned-process settlement.
- Native destructive behavior: only owned fixtures/test apps in a disposable
  environment; no user-cache cleanup, real-app uninstall, host repair or unrelated
  process termination merely to generate acceptance evidence.

Percentiles use nearest-rank with sample counts and raw arrays. Keep slow/failed runs
unless a documented predeclared harness invalidation applies; retain the invalid
attempt and reason. Do not silently substitute medians for p95 or diagnostics for
a frozen formal measurement.

## Evidence package

Capture source commit plus dirty-diff hash, binary/assets hashes, build flags,
toolchain/dependency pin, Windows build, CPU/GPU/driver, memory, power state,
display/DPI/refresh configuration, exact args, scope, fixture identities, timestamps,
raw samples, PID+creation identity and owned process-tree exit. CLI/schema parity,
CPU/GPU timing, wall clock and render submission are distinct results.

Native screenshots/UIA/IME tests use the actual release app. Headless GPUI tests,
browser fixtures, WSL and cross-compilation do not prove native platform acceptance.
Record unsupported GPU/sensor/platform as NOT_RUN/UNVERIFIED with cause.

Before Tauri retirement, C14 must establish whole-app benefit, five-mode usability,
data/authority compatibility and an explicit rollback record. Passing an empty
GPUI-window benchmark is only C09 evidence.
