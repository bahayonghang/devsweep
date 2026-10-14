# Research: CLI, Tauri, and shared runtime performance

- Query: Expand the cleanup-completeness plan to optimize both CLI/TUI and Tauri, including foreground responsiveness, shared backend work, and background/hidden lifecycle.
- Scope: mixed; current repository source/specs and archived measured evidence, with primary documentation for transport/profiling semantics.
- Date: 2026-10-13
- Ownership: runtime research only. Planet appearance/rendering is owned by the planet research; cleanup root/rule/execute completeness is owned by the cleanup research. No product edits, benchmarks, machine-wide scans, cleanup, or maintenance commands were run for this report.

## Findings

### Evidence classification

The current findings below are **source-observed mechanisms and performance risks**, not newly measured CPU, latency, or memory regressions. Historical measurements are identified separately. The screenshot does not establish backend performance or cleanup coverage. New implementation should begin with a release baseline and targeted reproductions, then fix measured causes without reducing supported coverage or weakening cleanup authority.

### Files found

| File | Purpose |
| --- | --- |
| `crates/devsweep-core/src/scan/mod.rs` | Shared scan orchestration, cumulative progress, merge/deduplication/ranking, final report. |
| `crates/devsweep-core/src/scan/project/mod.rs` | Marker-driven project traversal, target sizing, cooperative cancellation. |
| `crates/devsweep-core/src/scan/project/cargo.rs` | Existing scan-lifetime Cargo metadata cache. |
| `crates/devsweep-core/src/scan/global/probe.rs` | Provider executable resolution, bounded process probes, size observation. |
| `crates/devsweep-core/src/filesystem/sizing.rs` | Shared no-follow size walker and dedicated two-worker pool. |
| `crates/devsweep-core/src/process/mod.rs` | Provider/executor deadlines, cancellation, process-tree handling. |
| `crates/devsweep-cli/src/application/commands/clean.rs` | CLI scan, plan, preview, execute; scan output consumes only the terminal report. |
| `crates/devsweep-cli/src/application/output.rs` | JSON buffers and stdout/create-new file sinks. |
| `crates/devsweep-cli/src/application/commands/status.rs` | Real Status NDJSON producer/consumer, Ctrl+C and broken-pipe lifecycle. |
| `crates/devsweep-cli/src/tui/runtime/{mod.rs,workers.rs,tests.rs}` | TUI channel, event drain, redraw, worker ownership and regression tests. |
| `crates/devsweep-cli/src/tui/render/targets.rs` | Visible target viewport rendering. |
| `desktop/src-tauri/src/{commands.rs,scan.rs}` | Blocking scan worker, coordinator, 100 ms preview delivery coalescing. |
| `desktop/src/api/{bridge.ts,contract.ts}` | Tauri Channel transport and closed-world report/preview decoding. |
| `desktop/src/state/{app-state.ts,operation-coordinator.ts}` | Stale-event rejection and cancel/join lifecycle. |
| `desktop/src/components/TargetTable.tsx` | All-row table/evidence rendering. |
| `desktop/src/modes/clean/CleanWorkbench.tsx` | Scan event dispatch, query filtering, review workflow. |
| `desktop/src/modes/status/StatusWorkbench.tsx` | Snapshot on entry, opt-in live, preference-driven restart. |
| `crates/devsweep-core/src/status/{mod.rs,sampler.rs}` | Cached live sampling, unbounded producer channel, typed cadence/availability. |
| `desktop/src-tauri/src/{hud.rs,tray.rs,status.rs}` | HUD visibility ownership, repeated snapshots, live Status join. |
| `tools/measure-resources.ps1` | Existing historical resource harness; needs an isolated read-only mode before reuse here. |
| `.trellis/tasks/archive/2026-09/09-20-desktop-operation-performance/{prd.md,design.md,evidence/record.md}` | Prior protocol, inherited bounds, measured unresolved failures. |

### 1. Shared scan work is repeated even when the CLI discards progress

**Source evidence:** `scan/mod.rs:338-347` and `:394-403` merge, deduplicate, rank, and clone the cumulative plan on every discovered target. `merge_and_rank_targets` at `:448-452` rebuilds the merged target set and calls ranking each time. End-of-phase merging repeats work at `:352` and `:407`. The CLI passes a no-op progress closure at `application/commands/clean.rs:53-64`, but the core still builds those snapshots. A rescan currently first runs the normal full scan (`scan/mod.rs:313-314`) and then resizes the exact target; it is not an incremental cached scan.

**Risk:** with N small targets, cumulative clone volume grows as the sum of 1..N, before IPC and rendering costs. The sorting/deduplication cost is additional. This can dominate fast fixture scans and is likely to become more visible when functional coverage adds many targets. A slow disk scan may instead be I/O-bound; measure phase spans before predicting a speedup.

**Recommended narrow work:** add timing/counters for discovery, probes, sizing, merge/rank, snapshot construction and output; provide a way for terminal-only CLI consumers to avoid unused snapshot construction without duplicating the scanner or changing final ordering. For interactive consumers, investigate deferring expensive cumulative projection until an emission is needed. Keep cumulative, deduplicated, core-ranked snapshots and phase/terminal flush semantics. Changing the per-target progress API or removing per-target publication requires a deliberate update to the associated spec and tests, not a hidden optimization.

### 2. Sizing is worker-bounded, but enumeration and aggregate work deserve separate tests

**Already present:** `filesystem/sizing.rs:62-86` defines a default local entry budget of 50,000 and a reusable two-worker Rayon pool (hard ceiling four). Multi-root sizing reuses that pool (`:125-153`). Root-child fan-out is parallel, deeper recursion is sequential (`:320-351`). No-follow inspection and partial/unknown size semantics must stay intact. The historical one-handle Windows reparse/metadata optimization has already landed; do not propose it again as new work.

**Source risk A:** `sizing.rs:292-318` first collects `children: Vec<PathBuf>`. That enumeration loop checks `remaining == 0`, but does not decrement it and does not check cancellation inside the loop. The decrement happens when visiting an entry at `:211-224`. A wide directory can therefore buffer many names and delay cancellation before recursive visit budgets take effect. Project discovery has a similar latency seam: its entry loop at `scan/project/mod.rs:275-303` does metadata work for each file, with recursive cancellation checks reached for directories, not each file-only entry.

**Source risk B, intentional existing behavior:** `sizing.rs:377-385` gives each root child a fresh local budget. The explicit regression test `dedicated_and_serial_modes_preserve_fresh_root_child_budgets` at `:632-661` requires this behavior. Consequently, 50,000 is not a whole-tree or whole-scan work ceiling. Treat total work under high fan-out as a policy question, not an accidental bug to fix by cutting coverage.

**Recommended narrow work:** create deterministic wide-file, wide-subdirectory, deep-tree, unreadable, reparse/cloud-placeholder and cancellation-during-enumeration fixtures. Measure enumeration count, retained path count, metadata calls, I/O, warnings, and time to observe cancellation. Bound transient enumeration storage and check cancellation in the enumeration hot path if reproduced. Preserve fresh child budgets unless the functional design explicitly introduces separately named global budgets with truthful partial diagnostics and review/rescan behavior. Do not use broader parallelism as the default answer.

### 3. Provider phases already have deadlines; added coverage can consume them

`process/mod.rs:60-63` sets 5 s per provider probe and 30 s for the provider phase. `scan/global/probe.rs:35-40` starts the phase clock; `:49-99` enforces the remaining deadline and cancellation when executing probes. `estimate_path_size` uses the size walk, whose elapsed time also consumes the clock before later commands. Global providers are visited sequentially (`scan/global/mod.rs:62-101`). Cargo resolution already caches successful workspace members and failures for a scan (`scan/project/cargo.rs:17`, `:83-103`, `:140`).

**Risk:** additional rules/providers can increase discovery/probe/size latency or leave later probes without deadline, even if no individual provider becomes slower. PATH/executable probes can also repeat. Profile command count, actual resolved program, elapsed time, timeout/cancellation outcome and size time separately. Compare successful, absent, slow, inaccessible and malformed provider fixtures. Preserve neutral working directory, program/argv separation, process-tree containment and bounded output. Any memoization must be scan-scoped and cannot replace live authorization during execute. Increasing deadlines or skipping slow providers is not a performance fix or evidence of complete coverage.

### 4. Desktop throttles sends after doing expensive projection

`desktop/src-tauri/src/scan.rs:147-180` constructs `scan_preview_from_plan` before deciding whether to emit or replace `pending_preview`. This means the 100 ms coalescer reduces channel sends, but not upstream plan cloning, full projection, ID checks and total construction. `:192-209` flushes and sequences sends; `:225-232` flushes before terminal settlement. `commands.rs:101-110` correctly runs scanning off the UI executor with `spawn_blocking`, and releases backend ownership after the worker result.

On the frontend, `api/bridge.ts:205-211` decodes every message before dispatch. `api/contract.ts:1293-1320` builds a new target array, validates unique IDs, and recomputes totals across the full snapshot. `state/app-state.ts:154-163` rejects stale IDs/sequences but only after decode. This is necessary trust-boundary work, not validation that should simply be removed. A cumulative preview can be large even at ten sends per second. No frontend acknowledgment-based backlog limit was observed in this path; the existing backend pending slot alone is not proof that the WebView consumer cannot lag.

**Recommended narrow work:** separately instrument raw discoveries, constructed snapshots, projected snapshots, sends, bytes, JS receive/decode, reducer acceptance, render commits and terminal timing. Move work behind a negotiated publication policy or reduce duplicate materialization while preserving closed decoding. Test slow consumers and receiver disposal. Keep the current complete-replacement IPC contract initially; a delta protocol would require new sequence/recovery semantics and is not justified before measurement. Phase/final flushes must remain lossless even when interim previews are coalesced.

### 5. Desktop target rendering scales with every target and expanded evidence structure

`TargetTable.tsx:59-61` filters review targets for counts; `:74-92` maps all targets and mounts evidence lists inside `<details>` for every row. Closed `<details>` still creates React children/DOM nodes. `CleanWorkbench.tsx:221-233` creates a filtered report projection on query changes. Native input delay, decode time and React commit time should be measured independently so a slow decode is not misdiagnosed as a table-only problem.

**Recommended narrow work:** benchmark the real preview/review components with 100, 1,000, and 10,000 deterministic targets, long Unicode paths, mixed unknown/partial sizes and multiple evidence entries. Apply lazy evidence mounting and stable derivations first if they fix the measured bottleneck; choose accessible pagination or virtualization only if still necessary. Preserve target identity, keyboard focus, select-all meaning, full copyable paths/evidence, protected/inspect-only behavior and immediate digest invalidation. New production dependencies require explicit approval under the desktop spec. Do not truncate the report to make rendering cheap.

### 6. TUI event backlog and redraw differ from desktop behavior

`tui/runtime/mod.rs:53` uses `std::sync::mpsc::channel`; `:67-80` drains, draws and polls every 100 ms even without a state change; `:110-122` drains until the channel is empty before returning to keyboard input. `runtime/workers.rs:368-375` forwards every cumulative scan plan. A producer burst can accumulate increasingly large snapshots and extend the drain loop, delaying keyboard/cancel handling. This is a source risk, not a measured starvation claim.

The target renderer already renders a viewport, not every target row: `render/targets.rs:91-109` selects a visible slice after `visible_target_rows()`. Avoid reintroducing a second viewport abstraction. Measure all-row derivation separately from actual drawn rows.

**Recommended narrow work:** bounded/coalesced replaceable progress storage, a finite event-processing time slice and redraw-on-dirty with explicit animation/live deadlines. Preserve staged progress visibility, real terminal events, job IDs and join-before-replacement. Lifecycle events and final reports cannot be dropped with interim previews. Add a burst/slow-consumer runtime test proving input remains serviced and the latest final report arrives; add a quiet-idle draw-count test. Existing tests at `runtime/tests.rs:240`, `:304`, `:543`, `:629` cover progress, canceled-result handling, join-before-new-scan and cancellation-request semantics and should remain.

### 7. CLI output has serialization and durability costs; Status has an unbounded slow-consumer seam

For Clean JSON, `commands/clean.rs:232-251` materializes a `serde_json::Value`; `output.rs:152-163` then serializes a complete byte vector. Peak memory can therefore include the typed report, value tree and encoded bytes. `OutputSink::write_all` at `output.rs:115-126` locks/flushes stdout and flushes plus `sync_all`s each create-new-file write.

The real Status live path (`commands/status.rs:149-160`) serializes each event and calls that sink. Thus NDJSON `--output` syncs each event to disk. `status/sampler.rs:139` uses an unbounded channel and `:319-320` sends without consumer backpressure. A slow or stalled output pipe/file can accumulate snapshots while sampling continues. The current loop already cancels and joins on broken pipe and output errors (`commands/status.rs:121-143`); preserve it. A naive blocking bounded channel can deadlock cancellation if the consumer calls join while the producer is blocked on send, so test this explicitly.

**Recommended narrow work:** compare human/JSON/file and NDJSON fast/slow/broken consumers, track bytes and peak buffered events, and only then choose direct typed serialization or a bounded stream mechanism. Retain V1 envelopes, NDJSON sequence/terminal contract, one valid JSON document, stderr-only diagnostics, create-new behavior and exit classes. Changing file sync cadence is a durability-contract change to document and review; cleanup audit fsync behavior is out of this optimization's scope. Clean CLI currently uses the noncanceling scan API (`commands/clean.rs:64`); evaluate signal parity as a separate read-only scan requirement, not a change to wait-only execute semantics.

### 8. Status and HUD lifecycle is partly optimized already; hidden work must be measured by intent

Live Status owns one permit and joins its producer (`status/sampler.rs:124-172`); it reuses a cache, skips overlapping ticks (`:249-314`), refreshes process groups at 4 s and volume/power at 30 s (`status/mod.rs:31-42`). `StatusWorkbench.tsx:345-351` captures one snapshot on entry and keeps live opt-in. No visibility-based pause is present in that workbench; this is not automatically a bug because the component spec says minimizing must not cancel an operation (`desktop-frontend/component-guidelines.md:43`).

HUD already samples only while shown, cancels/joins on hide/drop, and refuses duplicate starts (`hud.rs:46-113`). `tray.rs:282-294` moves WebView creation/preference I/O off the event loop. Hide and destroy call shutdown (`:232-245`, `:332-338`). Tooltip posting deliberately avoids join/event-loop deadlock (`:354-362`). Do not describe hidden HUD polling as an established defect.

**Source risks:** every visible HUD tick invokes `capture_snapshot` (`tray.rs:350`), which constructs `StatusSampler::default()` (`status/mod.rs:487-491`); snapshot clones its state and performs a baseline/wait/second-sample cycle (`sampler.rs:109-120`). This does not reuse the live sampling cache between HUD ticks. HUD and live Status can run independent probes because HUD intentionally does not use the app operation coordinator. HUD's wait checks cancellation every 10 ms (`hud.rs:117-126`), which is a wakeup cost to measure. Hide joins synchronously (`hud.rs:91-98`); a slow underlying native probe could affect UI responsiveness despite cooperative waits.

**Recommended narrow work:** benchmark Status alone, HUD alone, both visible, main minimized with live enabled, hidden HUD, repeated show/hide, route change and app close. Consider a persistent sampler instance/state scoped to one visible HUD run if that removes measured repeated collection costs; preserve age/availability semantics, cancel/join, no post-hide publication and generation guards. Do not introduce a global shared sampling service or silently stop explicit background live monitoring before measurement and policy review. Hidden presentation updates may be coalesced independently from whether a requested backend operation continues.

### 9. Preserve operation-specific lifecycle; there is no universal cancel deadline

`state/operation-coordinator.ts:31-36`, `:81-89`, `:102-111` serializes transitions, calls cancellation and awaits join before replacement/close. This is an existing capability, not missing infrastructure. Desktop spec `state-management.md:169-185` additionally requires decode failure to request cancellation and wait for command settlement rather than releasing the active slot early.

Inherited operation table from archived performance `design.md`:

| Operation | Contract to retain |
| --- | --- |
| Analyze | Cooperative; request-to-ack p95 <= 500 ms over five runs; every run joins. |
| Status live | Cooperative; stop while the same app PID remains alive; join observed and 25 scheduled 200 ms post-stop samples. |
| Clean scan | Cooperative; record request/ack/join; no inherited numeric deadline. |
| Clean dry-run / execute | Wait-only; complete before replacement/close. Never force-kill to satisfy a performance gate. |
| Software/Optimize operations | Preserve each adapter's cooperative/after-dispatch outcome semantics; record request/ack/join, no universal bound. |
| Route change / window close | Cancel-and-join or wait for completion; no owned worker, subprocess or stale result after settlement. |

For Status post-stop, keep final-five CPU <= the same-instance pre-live baseline median + 0.5 percentage points. Check thread floors across five stops (last <= first + 1), not a per-stop WebView thread return assumption. Historical tests proved browser pool threads can decay over one to two minutes, beyond the 5 s window. Existing `desktop/src-tauri/src/status.rs:259` proves live returns only after joining the producer; retain that test.

### Historical measured evidence is unresolved, not current proof

The archived `evidence/record.md` reports final run `raw-20260923T021341Z`: 36 gates, 34 pass and two measured failures: Software p95 5071.65 ms against 5000 ms; Optimize list p95 1009.095 ms against 1000 ms. The historical decision paused performance work for functional completeness; the present request brings it back into scope, but does not turn old failures into passes.

The report also records previously fixed causes: missing bundled frontend assets in a cargo-only desktop build; shell header React update loop; broken Analyze benchmark selector; vacuous baseline/quiescence gates; and repeated Windows entry inspection. Software MSIX enumeration was approximately 0.7 s, size inspection 4.8-5.4 s over 184 paths/50,184 files before the latter fix. These are historical measurements, not measurements of the current tree.

The same unchanged Clean baseline took 9048.118 ms and 15398.923 ms median in two historical runs, showing substantial host-load variance. Preserve alternating pairs, fresh hashes, identical coverage and recorded ambient load. The record also left release cancellation/close evidence unverified for Clean scan, Clean dry-run/execute, Software, Optimize and route/close despite focused tests; include those missing scenarios in the new evidence plan.

### Prioritized implementation tasks and dependencies

| Priority | Deliverable | Verification / dependency |
| --- | --- | --- |
| P0 | Reproducible release CLI + native Tauri baseline, phase metrics, owned-process read-only harness mode | Before runtime edits; shared fixture/coverage manifest from cleanup child; no current performance claim without raw evidence. |
| P1 | Bound enumeration storage and verify prompt cancellation; avoid unused CLI cumulative progress work | Instrument first; compare identical target IDs, rules, size confidence, exclusions and warnings; preserve per-child budget contract unless explicitly revised. |
| P1 | TUI progress/event-drain fairness and slow Status consumer behavior | Burst, stalled-consumer, disconnect, Ctrl+C/cancel and join tests; bounded retained work with no terminal loss. |
| P1 | Desktop projection/IPC/decode/table hot path | 100/1k/10k fixtures; identify dominant stage; preserve authority-free previews and final report promotion; coordinate with cleanup UI child. |
| P2 | HUD visible sampling cost, combined Status/HUD load and hidden quiescence | Native same-PID visibility matrix and repeated show/hide; preserve minimize semantics; coordinate with planet child for graphics-only idle cost. |
| P2 | Revisit inherited Software/Optimize failures | Fresh stable-host runs; separate startup/sizing/output/OS-launch time; small verified fixes only. |
| P2 | CLI machine-output memory/durability optimization | Only if large-report/slow-file measurements justify it; no schema, stdout/stderr, audit-durability or exit-code regression. |

Runtime baseline/harness work can proceed independently of the planet assets. Final all-scope performance acceptance depends on finalized shared coverage and frontend integration. Record this dependency in the runtime child artifacts; tree position alone is insufficient.

## Continued evidence

See [Runtime measurement protocol, budgets, and source references](runtime-measurement-protocol.md) for the remainder of this investigation, including boundaries, acceptance proposals and references. Load both files for implementation/check context.
