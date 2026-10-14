# Runtime performance design and measurement protocol

## Correctness boundary

Keep one core scanner/executor with distinct observer policies for terminal-only and interactive consumers. Optimize computation/publication rather than candidate eligibility. The functional child owns coverage/DTO changes; this child must prove equal target ids, rules, paths, risk/selection, size confidence, diagnostics and final ordering under equal inputs. Compare new provider coverage separately so extra legitimate I/O is not mistaken for regression or hidden by omitting roots.

Preserve original baseline A, functionally corrected reference B before runtime optimizations, and final candidate C. Functional handoff publishes B's immutable binaries, golden observations/plans and source hashes. Repaired Go/configured/nested/partial scenarios compare B-to-C; their formerly failing A output cannot be a parity requirement. Compare A-to-C for previously healthy unchanged workloads and label intentional behavior changes. Refresh B whenever the functional contract changes, before applying affected runtime edits. UI renderer comparison separately retains its own original-versus-revised asset/render baseline.

## Backend work

Instrument discovery, metadata, provider resolution/processes, sizing, dedupe/rank, snapshot construction and serialization. Avoid materializing progress snapshots when the CLI only needs the final report. For interactive clients, retain a current candidate set and perform expensive cumulative ranking/projection only at the negotiated publication points, flushing phase/terminal updates. Keep the current replacement-snapshot contract initially; a delta protocol is unnecessary unless measurement demonstrates otherwise.

Enumeration should check cancellation inside wide loops and avoid retaining unbounded child-name vectors. Retain the existing two-worker pool and fresh per-root-child size budgets; the tests intentionally specify those. Streaming/chunking must not silently reduce entries visited, follow links, or convert partial/unknown sizes to verified values. Reuse executable/metadata observations only within one scan, and keep execution's identity/config/owner checks fresh.

## Transport, TUI and CLI

Use a bounded latest-pending slot for replaceable scan observations, separate from non-droppable lifecycle and final results. Compute expensive presentation data after the emission decision where ownership permits. Preserve scan id, sequence, closed decoding and flush-before-settlement. A bounded backend slot does not establish bounded WebView lag; measure consumer latency and apply a typed flow-control mechanism only if needed.

TUI handles a finite event batch/time slice before input, coalesces obsolete progress, and redraws on dirty state or a real deadline. Keep its existing visible-row renderer. Retain job-id rejection and join-before-replacement.

CLI machine output may serialize typed data directly if the duplicate `Value`/byte buffers dominate peak memory. Status requires cancellation-aware bounded handoff to slow sinks; never block join behind an uncancelable producer send. Preserve record/sequence/terminal and broken-pipe semantics. Any normal-output flush/sync change must be explicitly documented and validated; cleanup audit fsync is unchanged.

## React result handling

Measure receive/decode, reducer, selection/filter derivation, render and commit independently. Preserve validation even if it costs time. Mount evidence details lazily when expanded if profiling supports it, use stable derived views, and use accessible pagination/windowing only if required for 10k targets. Full selection/search scope must remain the full completed report, and every selected id remains correlated with the backend plan. Interim preview remains read-only.

## Foreground/background policy matrix

| State/operation | Required behavior | Evidence |
| --- | --- | --- |
| Idle CLI one-shot after output | No owned worker/subprocess remains; process exits | Owned-process tree and completion record |
| Idle TUI | Input/resize ready; no redundant periodic full redraw | Draw counters plus CPU baseline |
| Visible Clean stage | Planet respects FPS; progress responsive | UI child's Planet trace + integrated interaction |
| Hidden/minimized main window | Decoration stops; explicitly running operations continue as existing contract requires | Native visibility and worker/sample identity |
| Hidden HUD | Cancel and join HUD sampler; no later publication | Same-process hide/show trace |
| Visible HUD + live Status | Freshness/availability preserved; measure duplicate probes | Per-producer counters and process-tree CPU/I/O |
| Status stop | Cooperative stop and join; app PID remains alive | 25 x 200ms post-stop samples; five-cycle floor check |
| Analyze cancel | Ack p95 <=500ms and all runs join | Five or more native attempts and timing |
| Clean scan cancel | Cooperative cancellation; partial observations remain unselectable | Request/ack/join and final state; no invented universal deadline |
| Clean preview/execute | Wait-only once in flight; complete before replacement/close | Disposable fixture/fake adapter, audit/outcome, join |
| Route change/window close | Cancel/join or wait per operation; reject stale completions | Worker/process count and no late events |

Do not silently disable opt-in live monitoring when minimizing. Hidden presentation coalescing is separate from stopping a backend operation. HUD sampler reuse, if warranted, is local to one visible run rather than a new global sampling service.

## Safe benchmark prerequisites

Do not invoke the historical resource script wholesale. First provide an isolated, read-only mode with task-created process identities, separate app-data/preferences and disposable fixture roots. Remove maintenance execution from these runs and never stop a process merely because its executable name matches. Only owned benchmark children can be shut down, through normal lifecycle where available. No OS-scale/security/power change and no broad user-cache scan.

Record A before product code changes, then capture B at the functional handoff before runtime optimization. Build release CLI with the lockfile; build native desktop through Tauri so frontend assets are embedded. Prove the native page loads with no dev server. Keep reference binaries outside the next build's overwritten output with hashes and immutable copies in task-owned generated evidence storage. Each paired result states A or B explicitly; do not substitute a changed-coverage build silently.

## Workloads and metrics

- Fixtures: few large targets, many tiny targets, 100/1k/10k result projections, wide files/subdirs, deep trees, overlapping roots, long Unicode paths, permission/reparse boundaries, absent/slow/noisy providers, mixed ecosystems. Record complete/partial status and fixture identity.
- Cold: fresh process/profile first-use measured separately; state that OS file-cache coldness is uncontrolled. Warm: one discarded run and at least five alternating baseline/candidate pairs. Interaction microbenchmarks use >=20 samples; Planet keeps its dedicated protocol. State percentile method; five-run nearest-rank p95 is the worst sample.
- Resources every 200ms: root PID and owned process tree user/kernel CPU, private/working bytes, threads, handles, I/O bytes/ops, subprocess count, time to first discovery, elapsed/phase time, output bytes. State CPU normalization (`100% = one logical core` for new records), and preserve historical gate normalization when comparing inherited records.
- IPC/UI: constructed/projected/sent/received snapshot counts, retained bytes, oldest pending age, sequence/drop/coalesce counters, decode duration, accepted/rejected dispatches, commit time, DOM rows, long tasks and input-to-paint p50/p95.
- Sinks: CLI human/JSON stdout/file; Status fast/slow/broken NDJSON pipe and file; TUI input during bursts. Native: scan/preview/review/rescan, cancel/route/close, Status/HUD combinations and twenty visibility cycles.
- Use matched CDP-attached baseline/candidate for instrumented tests; capture a separate uninstrumented release idle run. React profiling build timings are not uninstrumented release CPU evidence.

## Reporting and rollback

Store host/build/fixture manifests, raw JSONL/CSV, commands, phase counters, coverage comparison and concise cause/fix/effect records under child `evidence/` during implementation. Do not overwrite archived September measurements. Missing/too-short samples are unavailable, not zero. A measured miss is fail.

Make each optimization independently revertible. First land safe measurement and regression fixtures, then one measured cause at a time. A progress protocol, output durability or lifecycle policy change requires its targeted spec update before integration. New provider coverage and functional fixes remain intact when an optimization is reverted.
