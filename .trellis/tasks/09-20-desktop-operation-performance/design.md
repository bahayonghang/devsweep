# Technical design

## Measurement protocol

Freeze commit, release configuration, host metadata, fixture hashes, and
window size. Use `tools/measure-resources.ps1` for process/resource samples
and the existing Analyze render benchmark for render-specific runs. Warm up
each scenario, sample every 200 ms, repeat at least five times, and retain raw
CSV/JSON plus the command line. Workloads are idle, Clean preview/execute,
Analyze, Software, Optimize, Status live, cancellation, and quiescence after
completion.

The archived five-mode design supplies the threshold reference. It is not
current proof: each run must identify whether it is local, fixture, simulated,
or native and must record skipped/unavailable criteria.

### Clean baseline and comparators (TPR-01)

The current sampler compares against a hard-coded historical median
(`tools/measure-resources.ps1:1381-1392`), passes threads and private bytes
as recorded-only `$true` gates (`:1393-1397`), and writes `pass = $true` when
the comparison is not comparable (`:1398-1404`). None of these is a real
comparison. This child replaces that block:

1. Before any performance edit, build the baseline release CLI binary from
   the parent base commit (the HEAD recorded when the first child started)
   in a separate git worktree with the same toolchain; preserve it with its
   SHA-256 and commit as `baseline`. The candidate is the current release
   CLI binary with its own SHA-256 and commit.
2. Run one discarded warm-up per binary, then five alternating pairs
   (`A/B`, `B/A`, `A/B`, `B/A`, `A/B`) of the projects-scope JSON clean scan
   (`clean scan --root <repo> --scope projects --format json`) on the same
   absolute root, with the entry count recorded immediately before the
   exclusive window; no other heavy workload runs during the window.
3. The manifest records for both binaries: commit, SHA-256, host CPU, logical
   cores, RAM, Windows build, power plan, toolchain, locale, Defender state,
   scan root, entry count, and run order.
4. Gates (nearest-rank median over five runs, maximum over five runs):
   `clean.median_ratio_le_1_20` = candidate median elapsed / baseline median
   elapsed <= 1.20; `clean.peak_private_le_baseline_plus_64mib` = candidate
   max peak private bytes <= baseline max + 67108864 B;
   `clean.max_threads_le_baseline_plus_2` = candidate max threads <=
   baseline max + 2.
5. `clean.baseline_comparable` is a real gate: it fails when the host,
   build, toolchain, power plan, scan root, entry count drift beyond the
   recorded value, or baseline binary hash is missing or differs from the
   manifest. When it fails, all three Clean gates are `fail`, and the summary
   `pass` is false. No recorded-only gate remains.

### Desktop Status stop and quiescence (TPR-02)

The current live experiment (`tools/measure-resources.ps1:1098-1144`) starts
`devsweep status live` as a CLI process, closes or kills it, and samples the
dead PID; zero samples from an absent process pass the final-five predicate.
That path stays as CLI-exit evidence under the existing `status.live.*` gate
names and is labelled `cli-exit` in the summary. Desktop quiescence is a new
experiment on the release desktop binary:

1. Launch the desktop with the CDP driver (`tools/measure-resources.ps1:552`)
   and isolated `LOCALAPPDATA`; record the app PID and process tree.
2. Route to `#/status`, start live through the UI, settle 30 s, and retain
   60 s of samples on the app PID for the live thresholds.
3. Stop request: the driver clicks the UI stop-live control, which calls
   `bridge.statusCancel` → `status_cancel`
   (`desktop/src-tauri/src/status.rs:219-225`). Acknowledgement: the Status
   surface `data-status` becomes `canceling`
   (`desktop/src/modes/status/state.ts:209-210`). Join is layered, because
   the release binary exposes no coordinator or Tauri promise to the CDP
   driver: (a) `data-status` leaves `canceling` for `ready` or `idle` when
   the terminal event is delivered; (b) a focused Rust test proves
   `status_live_start` returns only after `control.join()`
   (`status.rs:143-168`) and `finish`; (c) the CPU part of the final-five
   hold on the same PID fails if the producer or a blocking worker keeps
   running; (d) the post-stop thread floor does not grow across the five
   stops on one app PID, which fails if a thread survives each stop. All four
   are required for `status.desktop.stop_joined`.
4. The 25 scheduled 200 ms samples start at the acknowledgement timestamp.
   Every sample must have `present = true` for the same app PID; a missing
   sample, an absent PID, or a different PID fails
   `status.desktop.poststop_25_samples`.
   `status.desktop.poststop_final_five_hold` keeps the archived CPU clause
   (final five consecutive samples: CPU <= baseline median + 0.5 pp) and has
   no per-rep thread clause. The baseline is the same CDP-attached app
   instance sampled on the Status route immediately before that rep starts
   live, not the separate no-remote-debug idle instance: the idle instance
   carries no remote-debugging threads and no prior Analyze work, so it is
   not a comparator for this process.

   Thread growth is checked across stops, not per rep: the post-stop thread
   floor of the last measured stop must be <= the floor of the first measured
   stop + 1. A thread that survives each stop raises the floor by one per
   stop and fails this check.

   Decision (user, 2026-09-23): drop the per-rep thread clause. Two
   comparator shapes failed on rep 2 only, with no product cause. Run
   `raw-20260922T142646Z`, maximum comparator (final five <= pre-live max +
   1): rep 2 read 42 threads for samples 0-8, then 45 for samples 9-24, with
   0.0 ms CPU in all 25 samples. Run `raw-20260922T151042Z`, floor-return
   comparator (post-stop min <= pre-live min + 1): rep 2 pre-live floor 41,
   post-stop flat at 44, 0.0 ms CPU. In both runs the live window holds 43
   threads, post-stop reaches 42 in four of five reps, and the WebView2 pool
   decays from 44 to 41 over one to two minutes, which the frozen 5 s window
   cannot observe. A per-rep thread count measures the browser pool, not the
   operation.
   `status.desktop.stop_joined` requires the join observation before the
   window ends and records request→ack and ack→join latency.

5. Repeat one warm-up and five measured stops; `CloseMainWindow`, `Kill`, or
   process exit never counts as a stop.

### Operation cancel/completion table (TPR-06)

| Operation                            | Cancel semantics                                      | Request                      | Acknowledgement                            | Join                                                    | Bound and statistic                                                         | Evidence source                                   |
| ------------------------------------ | ----------------------------------------------------- | ---------------------------- | ------------------------------------------ | ------------------------------------------------------- | --------------------------------------------------------------------------- | ------------------------------------------------- |
| Analyze                              | cooperative                                           | `bridge.analyzeCancel`       | `.analyze-mode[data-status]` = `canceling` | `data-status` in `canceled`/`partial`/`complete`/`idle` | ack p95 <= 500 ms over 5 runs, all joined (frozen)                          | `analyzeCancel` driver action                     |
| Status live                          | cooperative stop                                      | `bridge.statusCancel`        | `.status-mode[data-status]` = `canceling`  | `data-status` = `ready`/`idle`                          | joined; 25 same-PID post-stop samples; final-five hold (frozen)             | desktop stop experiment above                     |
| Clean scan                           | cooperative                                           | `bridge.scanCancel`          | reducer `scan_cancel_requested`            | `scan_canceled`/terminal report                         | joined; latency recorded, no numeric bound                                  | focused coordinator/mode tests plus release trace |
| Clean dry-run / execute              | wait-only (`cancel: () => undefined`, `cancel: None`) | none                         | none                                       | `planDryRun`/`planExecute` result                       | completes before replacement or close; never force-killed; elapsed recorded | focused tests plus release trace                  |
| Software inventory/preview/uninstall | cooperative                                           | `bridge.softwareCancel`      | reducer `cancel_requested`                 | terminal result after `finish`                          | joined; latency recorded, no numeric bound                                  | focused tests plus release trace                  |
| Optimize list/preview/run            | cooperative                                           | `bridge.optimizeCancel`      | reducer `cancel_requested`                 | terminal result after `finish`                          | joined; latency recorded, no numeric bound                                  | focused tests plus release trace                  |
| Route change / window close          | coordinator `cancelAndJoin` / `close`                 | shell route or close request | active operation's cancel                  | `operation.join` resolved                               | no owned work after join; latency recorded                                  | lifecycle tests plus release trace                |

Latency is request→acknowledgement and acknowledgement→join, each recorded
per run. Only the frozen Analyze and Status rows have numeric bounds; the
other rows pass when the join observation exists and no stale completion or
orphan is found. A timeout-based kill is a `fail`, never a pass.

### Evidence status (TPR-07)

`pass` and `fail` are measured. `fail` blocks this child's handoff until the
owning child fixes the cause and the run is repeated. `UNVERIFIED` is only
for evidence this child cannot capture (native-only rows), with reason and
owner. `skipped` needs a reason and is invalid for a required row. The
handoff never converts a measured `fail` to `UNVERIFIED`.

## Diagnosis seams

Trace one operation from React trigger through
`desktop/src/state/operation-coordinator.ts`, Tauri worker/channel delivery,
`desktop/src/api/bridge.ts` decoding, reducer/selectors, and the mode render
tree. Compare event rate, queue depth, operation IDs, request/acknowledgement
and acknowledgement/join latency from the operation table, render counts,
CPU, private memory, and thread count. A code change is
allowed only when a trace identifies a bottleneck or lifecycle defect.

## Fix boundary

Prefer one coordinator/channel/reducer correction over new buffering layers or
parallel service paths. Preserve stale-event rejection, joined cancellation,
partial/unavailable states, typed audit data, and clean plan/digest authority.
Threshold changes are never a fix.

## Rollback

Keep measurement artifacts independent of code edits. If focused tests or
release evidence regress, revert the smallest performance change and retain
the failing raw run for diagnosis; do not hide the failure by changing the
protocol.
