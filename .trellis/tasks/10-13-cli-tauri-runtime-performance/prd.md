# CLI and Tauri foreground and background performance

## Goal

Reduce measured redundant work and responsiveness problems in the shared Rust backend, CLI/TUI, and Tauri/React, with explicit foreground/background lifecycle and release-build evidence. Never trade away scan coverage or cleanup safety for a faster number.

## Ownership and dependencies

- Parent: `10-13-desktop-cleanup-completeness`. Own measurement harness, scheduling/materialization/backpressure, focused sizing/output improvements, TUI fairness, desktop large-result rendering, and Status/HUD lifecycle cost.
- UI child owns Planet-specific rendering/assets and graphics benchmark. Functional child owns discovery/rule/coverage semantics, new provider catalogue, roots and CLI/Tauri user-facing outcomes.
- Start baseline/harness work before implementation changes. Shared scan/event/output edits follow the functional child's DTO and rule stabilization or an explicit file handoff. Final measurements depend on integrated UI and functional children; baseline capture does not.

## Evidence and priority

Current findings are code-observed mechanisms, not newly measured regressions. Exact evidence is in `../10-13-desktop-cleanup-completeness/research/cli-tauri-performance.md`.

| Priority | Mechanism | Source anchor |
| --- | --- | --- |
| P1 | Cumulative plan clone/merge/rank per target, even with no-op CLI observer | `crates/devsweep-core/src/scan/mod.rs:338-347,394-403`; `crates/devsweep-cli/src/application/commands/clean.rs:53-64` |
| P1 | Wide directory names buffered before visit/cancel checks | `crates/devsweep-core/src/filesystem/sizing.rs:292-318`; `scan/project/mod.rs:275-303` |
| P1 | Full desktop preview projection before 100ms coalescing | `desktop/src-tauri/src/scan.rs:147-180` |
| P1 | TUI unbounded progress channel/drain-before-input and 10Hz unconditional redraw | `crates/devsweep-cli/src/tui/runtime/mod.rs:53,67-80,110-122` |
| P1 | Full cumulative decode and all-row/evidence DOM construction | `desktop/src/api/contract.ts:1293-1320`; `desktop/src/components/TargetTable.tsx:59-92` |
| P1 | Status slow-consumer queue, JSON duplicate buffers, per-event file sync | `crates/devsweep-core/src/status/sampler.rs:139,319`; CLI `application/output.rs:115-126,152-163` |
| P2 | HUD creates a fresh snapshot sampler each visible tick; combined HUD/live probes may repeat | `desktop/src-tauri/src/tray.rs:350`; core `status/mod.rs:487-491` |

Historical operation evidence recorded Software p95 5071.65ms against 5000ms and Optimize list 1009.095ms against 1000ms. A separate native record retained a 45-thread result against 40. Remeasure all of these; they are not current results or proof that Planet caused them.

## Requirements

- RT-01: Preserve three build identities: original baseline A, functionally corrected/stable-DTO reference B before runtime optimization, and optimized candidate C. Refresh B if functional semantics change. Use B-to-C parity/timing for repaired and new coverage; retain A-to-C comparisons for previously valid unchanged workloads and label behavioral differences. Record source identity, fixtures/toolchain, phase spans and process-tree resources for release CLI/native Tauri, cold versus warm work, old versus added coverage, and frontend interaction separately from backend duration.
- RT-02: Eliminate unused cumulative progress materialization for terminal-only CLI; reduce interactive snapshot/projection work before emission. Keep final target ordering, deduplication, health and selected authority identical. Update the per-target progress spec deliberately if publication changes.
- RT-03: Bound transient enumeration storage and observe cancellation during wide/file-only enumeration. Preserve no-follow, size-confidence and intentional fresh per-child sizing budgets; do not silently replace them with a smaller whole-root limit. Profile probes and reuse only safe scan-lifetime work, never live execute checks.
- RT-04: Bound replaceable progress retention and preserve lifecycle/final delivery. TUI gives input a finite service interval and redraws only for actual state/size/animation deadlines. Slow Status consumers, broken pipes and cancel/join cannot leak or deadlock; do not drop contractual NDJSON records.
- RT-05: Measure desktop decode, reducer and render separately. Reduce pre-throttle projection, needless materialization and closed-details DOM. If needed, add accessible pagination/windowing without a new production dependency. Keep full targets selectable/searchable/copyable; do not truncate data or remove boundary validation.
- RT-06: Measure idle, visible, hidden, minimized and post-operation states. Planet suspends through UI child; hidden HUD owns no sampler after join; explicitly requested main-window work continues on minimize under the existing policy. Optimize repeated HUD sampling only when profiles justify it, preserving freshness and availability.
- RT-07: Preserve operation-specific stop behavior: cooperative scan/Analyze/Status request-ack-join, wait-only Clean preview/execute completion, no forced kill to satisfy a timing target. Improve/read-only CLI scan signal handling if the benchmark reproduces deficient cancellation.
- RT-08: Re-run inherited Software/Optimize/Analyze/idle gates and fix measured causes within current features. Every result is pass, fail or unavailable with reason and owner. No unmeasured 'faster' claim, no threshold relaxation after a failing run.
- RT-09: Adapt resource tooling to isolated profiles, task-owned processes and read-only fixtures before use. Existing harness force-stops matching executables and runs maintenance (`tools/measure-resources.ps1:1192-1213`); those behaviors are outside this work's authorized benchmark scope. Keep cleanup audit durability unchanged.

## Acceptance criteria and budgets

- [ ] RT-AC1 (RT-01,09): Reproducible evidence contains source/build/asset hashes, exact argv, fixture seed/hash/counts, effective roots/rules, host/toolchain/WebView2/power/DPR metadata, all raw samples and percentile method. No user process is terminated and no live user cleanup/maintenance is used as a benchmark.
- [ ] RT-AC2 (RT-02,03): B-to-C golden results are semantically identical at equal coverage; A-to-C parity is required only for unchanged healthy workloads. Repaired Go/configured/nested/partial cases use B, never A's failed or incomplete output as the correctness baseline. Terminal-only CLI constructs zero unused progress snapshots; interactive expensive projections scale with publications rather than every discovered target. Wide enumeration has bounded temporary storage and cancellation checkpoints without lowering inherited per-child coverage.
- [ ] RT-AC3 (RT-01,02,03): Comparable Clean candidate median <=1.20x reference, peak private bytes <=reference +64MiB, peak threads <=reference +2 (inherited regression guards). Reference is B for repaired/new semantics, with additional A comparison for unchanged healthy workloads; each record identifies it. Show a measurable improvement in the profiled dominant phase or a deterministic eliminated-work count when timing is below noise; a regression guard alone is not an optimization result.
- [ ] RT-AC4 (RT-04,05): On frozen 1k/10k result fixtures, key/click-to-visible-update p95 <=100ms (proposed). Interim replaceable progress retains O(1) snapshots per producer; lifecycle delivery is bounded and lossless. A paused/slow consumer, canceled producer and closed window settle without deadlock or stale final promotion.
- [ ] RT-AC5 (RT-04): Idle TUI has no steady 10Hz redraw absent changes/deadlines. Status NDJSON maintains valid monotonic sequence/terminal semantics with slow pipe/file and broken pipe; producer is canceled/joined and no unbounded queue remains. JSON stays one valid V1 envelope and named output stays create-new.
- [ ] RT-AC6 (RT-06,07): The design's lifecycle matrix passes with request/ack/join records. Hidden HUD and settled operations emit zero later samples; twenty visibility/navigation cycles show no owned-resource growth. Minimize preserves explicitly running work while decorative/presentation work suspends or coalesces appropriately.
- [ ] RT-AC7 (RT-06,08): Remeasure inherited native host-PID idle CPU <=1%, private <=192MiB, threads <=40; Software p95 <=5s; Optimize list p95 <=1s; Analyze layout p95 <=50ms, commit p95 <=100ms, cancellation ack p95 <=500ms. Keep full-process-tree resources separately reported. Misses remain failed criteria, not excused by historical task archival.
- [ ] RT-AC8 (RT-07): Same-PID Status stop has join plus 25 scheduled 200ms post-stop samples; final-five CPU <=same-instance pre-live median +0.5 percentage points. Across five stop cycles thread floors do not grow by more than one from first to last. Clean scan records latency without inventing a universal join deadline; Clean preview/execute complete without forced interruption.
- [ ] RT-AC9: Root `just ci`, relevant desktop tests/build/wire parity, production component benchmarks and native lifecycle checks pass. Parent receives integrated baseline/candidate comparisons with coverage equality and all unresolved failures explicitly recorded.

## Out of scope

Blanket async/parallel rewrites; changing audited cleanup durability; substituting a subprocess CLI for core; unbounded whole-machine scans; new profiling/virtualization production dependencies; silently pausing user-requested background work; new maintenance actions; user process termination; changing OS scaling or power/security settings.

## Artifact status

The user approved the parent and this child's interaction/retention targets on 2026-10-13; inherited thresholds remain separately labeled. Fresh measurements and narrow fixes are implementation work, not completed results. No whole-app portable GPU/process-tree absolute limit is invented; the protocol records a comparable baseline before choosing a targeted optimization.
