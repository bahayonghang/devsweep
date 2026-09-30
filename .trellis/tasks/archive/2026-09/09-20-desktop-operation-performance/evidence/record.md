# Cause, fix, and evidence record

Protocol `five-mode-v1`. Final release run `raw-20260923T021341Z`, recorded
2026-09-23T02:13:46Z. Commit `fc4269fe5c1bc65bf53df97e5c09c6b36081deda` plus
the uncommitted working tree of this task. Host: Intel Core Ultra 9 275HX, 24
logical cores, 68143853568 B RAM, Windows build 26200, Balanced power plan,
locale `zh-CN`, Defender real-time protection off, rustc 1.98.0, node v26.7.0.

Candidate CLI `target/release/devsweep.exe`
sha256 `79cd6f6b2bb43524ede37d9e5dcc39cb44906c2c41eb87215afb05b5e747e5c9`.
Candidate desktop `target/release/devsweep-desktop.exe`
sha256 `288e29ac92b1f5c94e1d05e4e8d9d3dadc9c334dc928828ba900c8acb66e4b56`.
Clean baseline `D:/dsb/target/release/devsweep.exe`
sha256 `7995850527da06c8e805bf89f219807649f8f9a385a97b6c5e782c7697f937b0`,
same commit and toolchain, preserved in a separate git worktree.

Result of the final run: 36 gates, 34 `pass`, 2 `fail`
(`software.p95_elapsed_le_5s` at 5071.65 ms, `optimize.list_preview.p95_le_1s`
at 1009.095 ms). The run before it (`raw-20260923T013011Z`, same binaries)
was also 34 `pass`, 2 `fail`, with a different pair; see open item 1.

Decision (user, 2026-09-23): performance work is paused. Functional
completeness of the five modes comes first; the two open measured `fail`
rows stay open and are revisited later. No threshold is changed.

## Causes found and fixed

### 1. The release desktop binary served no application

`cargo build --release -p devsweep-desktop` produces a binary that points at
`devUrl` (`http://127.0.0.1:4180`) and embeds no assets, because
`frontendDist` is applied by the Tauri CLI, not by cargo. Every desktop
experiment in this task's earlier runs drove a window showing
`ERR_CONNECTION_REFUSED`; `analyze-cancel.json` recorded
`filled: false, started: false, status: ""` for all rows.

Fix: build the desktop candidate with `cd desktop && npm run tauri -- build
--no-bundle`. Verified by scanning the binary for the bundled entry chunk
(`index-V1VdENQv.js`). No source change.

Owner of the durable form of this finding: `09-20-desktop-native-acceptance`
(package and executable identity).

### 2. React error #185 on entering the Status route

`PageHeaderSlot` wrote its `children` into shell state from a
`useLayoutEffect` keyed on `children`. A mode builds a new JSX element on
every render, so each mode render wrote shell state, which re-rendered the
mode, until React aborted the tree. In the release build the whole React tree
unmounted (`#root` children 0) on `#/status`. The test suite did not see it
because tests mount modes without the `PageHeaderSlotContext` provider, where
the setter is a no-op default.

Fix: `desktop/src/app-shell/AppShell.tsx` renders the chip through
`createPortal` into a slot element held in state by `ref` callback, so mode
output never becomes shell state.

Regression test: `AppShell.test.tsx` "renders a page-header chip without
re-rendering the mode that owns it". Verified non-vacuous — reverting
`AppShell.tsx` fails the test with "Maximum update depth exceeded".

### 3. The Analyze render benchmark could not find the start control

`AnalyzePage.tsx:252` renders `.analyze-toolbar .primary-button` only once a
snapshot exists; before the first run the start control lives in
`.mode-empty`. The benchmark timed out waiting for a selector that cannot
exist yet.

Fix: `desktop/src/modes/analyze/render-benchmark.tsx` also accepts
`.mode-empty .primary-button`. `analyze.react_commit_p95_le_100ms` moved from
`error` to `pass` at p95 11.5 ms.

### 4. Four Clean gates could not fail (TPR-01)

The sampler compared against a hard-coded historical median, reported threads
and peak private bytes as `*_recorded` gates that were always `$true`, and
wrote `pass = $true` when the comparison was not comparable.

Fix in `tools/measure-resources.ps1`: a preserved baseline binary, one
discarded warm-up per binary, five alternating pairs, a manifest of both
binaries and the host, a real `Test-CleanComparable`, and three real bounds.
No `*_recorded` gate remains and no gate can report `pass = $true` with
`comparable = $false`.

Self-test (`-SelfTest`, 23 assertions, all pass) feeds an absent PID, a
different PID, an empty sample set, a missing manifest, a missing baseline
hash, a differing baseline hash, toolchain drift, host drift, root drift, and
entry-count drift, and requires `fail` for each.

### 5. Status stop was proved only by a dead CLI process (TPR-02)

The previous live experiment killed `devsweep status live` and sampled the
dead PID; zero samples from an absent process satisfied the final-five
predicate. That path is retained under its existing `status.live.*` gate
names and is labelled `cli-exit evidence:` in the gate detail.

Added: a desktop stop experiment that starts live through the UI, samples the
live app PID, clicks the UI stop control, and takes 25 scheduled 200 ms
post-stop samples on the same PID. `CloseMainWindow`, `Kill`, and process
exit never count as a stop. `Test-PostStopSamples` requires exactly 25
samples, all `present`, all carrying the same `pid`.

Added a focused Rust test,
`desktop/src-tauri/src/status.rs::run_live_returns_only_after_the_producer_thread_is_joined`.
The live producer holds the single process `LIVE_PERMIT` for its whole life
and releases it only on thread exit, so a second `spawn_live` that succeeds on
the first try, with no sleep and no retry, proves `run_live` returned only
after the thread was joined.

### 6. Software inventory spent 5 s sizing MSIX install paths

Trace of `software inventory --source msix` on the pre-change binary:
enumeration 0.7 s, install-path sizing 4.8-5.4 s over 184 paths and 50184
files. ARP took 43 ms and MSI 84 ms. The sizing walk in
`crates/devsweep-core/src/filesystem/sizing.rs` opened every entry twice:
`fs::symlink_metadata`, then the `CreateFileW` reparse probe.

Fix (user decision, 2026-09-23): one handle per entry.
`PathReparseProbe::inspect_entry` in
`crates/devsweep-core/src/filesystem/reparse.rs` returns metadata and the
reparse tag from one no-follow `CreateFileW` open (`GetFileInformationByHandle`
plus `FileAttributeTagInfo`). Any open or query failure returns `None`, and
the walk falls back to the previous two-step path with its error
classification unchanged. Test probes keep the default `None`, so every
injected-probe test still exercises the two-step path. The same walk sizes
Clean scan results.

Test: `one_handle_entry_inspection_matches_metadata_plus_probe` asserts that
the one-handle result equals the two-step result for a file, a directory, and
a directory symlink (symlink creation works on this host), and that a missing
path takes the fallback and returns an error.

Effect, interleaved A/B on MSIX only, six pairs: median wall 8275 ms to
5743 ms, median CPU 14906 ms to 9781 ms. Release protocol: Software p95
3596.429 ms (`raw-20260923T013011Z`) and 5071.65 ms
(`raw-20260923T021341Z`); Clean candidate/baseline median ratio 0.683 and
0.674, against 0.941 before the fix.

## Operation table

Measured values from `evidence/resources/operation-table.json`.

| Operation                            | Bound                                                     | request→ack p95                        | ack→join p95 | Status       | Gate or source                                                                               |
| ------------------------------------ | --------------------------------------------------------- | -------------------------------------- | ------------ | ------------ | -------------------------------------------------------------------------------------------- |
| Analyze                              | ack p95 <= 500 ms over 5 runs, all joined                 | 3 ms                                   | 25 ms        | `pass`       | `analyze.cancel_ack_p95_le_500ms`                                                            |
| Status live                          | joined; 25 same-PID post-stop samples; CPU final-five hold; floor stable across stops | 4 ms | 266 ms | `pass` | `status.desktop.stop_joined` |
| Clean scan                           | joined; latency recorded, no numeric bound                | not measured                           | not measured | `UNVERIFIED` | release sampler has no cancel driver for Clean scan; owner `09-20-desktop-native-acceptance` |
| Clean dry-run / execute              | completes before replacement or close; never force-killed | not applicable (wait-only, no request) | not measured | `UNVERIFIED` | wait-only path; owner `09-20-desktop-native-acceptance`                                      |
| Software inventory/preview/uninstall | joined; latency recorded, no numeric bound                | not measured                           | not measured | `UNVERIFIED` | release sampler has no cancel driver for Software; owner `09-20-desktop-native-acceptance`   |
| Optimize list/preview/run            | joined; latency recorded, no numeric bound                | not measured                           | not measured | `UNVERIFIED` | release sampler has no cancel driver for Optimize; owner `09-20-desktop-native-acceptance`   |
| Route change / window close          | no owned work after join; latency recorded                | not measured                           | not measured | `UNVERIFIED` | lifecycle covered by focused tests only; owner `09-20-desktop-native-acceptance`             |

The five `UNVERIFIED` rows carry no invented numbers. Their focused-test
coverage exists (`just ci`: 51 desktop Rust tests pass, 54 focused desktop
UI tests pass), but this child did not capture release-run latency for them.

## Gate results

Final run `raw-20260923T021341Z`: 34 `pass`, 2 `fail`.

Clean, with the real preserved baseline:

- `clean.baseline_comparable` `pass`
- `clean.median_ratio_le_1_20` `pass` — ratio 0.674, candidate median
  10371.786 ms, baseline median 15398.923 ms
- `clean.peak_private_le_baseline_plus_64mib` `pass` — 3719168 B vs 3895296 B
- `clean.max_threads_le_baseline_plus_2` `pass` — 8 vs 8

Analyze: `accounted_le_256mib` 38906299 B, `peak_private_le_512mib`
449327104 B, `p95_cpu_le_200` 188.2, `layout_p95_le_50ms` 1.750 ms,
`react_commit_p95_le_100ms` 6.5 ms, `cancel_ack_p95_le_500ms` 28 ms — all
`pass`.

Status desktop: all three `status.desktop.*` gates `pass`. Every stop holds
25 present samples on the same live app PID and executable path, reaches
terminal `data-status = ready` (join 254, 263, 259, 266, 266 ms), and meets
the CPU final-five hold. Post-stop thread floors 46, 42, 42, 42, 42: no
growth across the stops.

Idle: `idle.max_threads_le_40` `pass` at 35 in both post-fix runs.

## Open items

### Open item 1 — two marginal gates, measured `fail`

| Gate | Bound | `raw-20260923T013011Z` | `raw-20260923T021341Z` |
| --- | --- | --- | --- |
| `software.p95_elapsed_le_5s` | 5000 ms | `pass` 3596.429 | `fail` 5071.65 (runs 5072, 4957, 4938, 4734, 4950) |
| `optimize.list_preview.p95_le_1s` | 1000 ms | `fail` 1063.993 (runs 1064, 1054, 1009, 890, 802) | `fail` 1009.095 (runs 1009, 969, 921, 833, 897) |
| `optimize.settings.search.launch_p95_le_2s` | 2000 ms | `fail` 2549.286 (runs 1138, 2549, 2087, 1168, 1115) | `pass` 1113.325 |

Host speed differed between the two runs. The unchanged Clean baseline
binary measured a median of 9048.118 ms in the first run and 15398.923 ms in
the second, 70% slower. Before the final runs the host averaged 45% CPU load
from interactive applications (editor, terminals, chat, WebView2 hosts).

Optimize code does not call the changed sizing code. The settings gate times
a Windows Settings page launch.

Status: measured `fail` for the gates that failed in the final run. Not
converted and thresholds not changed. A run on a host without other
interactive load is needed to separate host load from a regression. Owners:
Software mode for `software.p95_elapsed_le_5s`, Optimize mode for
`optimize.list_preview.p95_le_1s`.

### Resolved in this task

- `idle.max_threads_le_40`: measured 45 and 46 in two earlier runs, 35 in both
  post-fix runs. Handed to `09-20-desktop-native-acceptance` for native
  re-measurement (user decision, 2026-09-23).
- Desktop Status thread clause: two per-rep comparator shapes failed on rep 2
  with 0.0 ms CPU in every post-stop sample, because WebView2 pool threads
  decay over one to two minutes. Replaced by a floor that must not grow across
  the stops (user decision, 2026-09-23; design.md TPR-02 step 4 records the
  measurements).

## Working-tree notes

- `.trellis/tasks/08-29-five-mode-native-integration/` holds empty
  directories and one `raw-latest.txt` created by early `-SelfTest` runs
  before the `-SelfTest` output guard was added. Removal was denied by the
  permission classifier and is left for the user.
- `D:/dsb` is a git worktree holding the preserved Clean baseline binary. It
  must stay until the acceptance child finishes or the baseline is
  re-recorded.
- `evidence/resources/repro-launch.json` is a smoke-test leftover.
