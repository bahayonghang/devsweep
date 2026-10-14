# Scanner, TUI and Status implementation checkpoint

Implementation began only after the coordinator released the immutable B3 native
Clean gate. This checkpoint reports focused implementation evidence; paired native
B3-to-C resource/coverage acceptance remains owned by the coordinator/runtime work.

## Work eliminated and retained contracts

- `scan::Sweeper::with_progress_policy` selects default `Interactive` or
  `TerminalOnly`. Interactive discovery constructs/ranks/clones its cumulative
  snapshot on the first target per phase, then at 100 ms, with phase/cancel/final
  boundary publication. Terminal-only CLI Clean/rescan publishes zero snapshots;
  scanner target discovery, final deduplication/ranking and health remain intact.
- `validate_scan_preview_plan` borrows IDs while checking empty/duplicate identity.
  DTO projection calls it; Tauri can validate raw observations before deciding
  whether projection is needed. No wire fields or authority are added.
- Project traversal already streamed names. Its loop now checks cancellation
  before inspecting each entry, including file-only directories, and records
  canceled root coverage. Root-failure comment was corrected to the existing
  continue-healthy-roots behavior.
- TUI delivery uses a dedicated mailbox with a 32-record lossless FIFO and one
  replaceable pending scan observation. A single lock orders queue/slot access;
  full-FIFO waits release that lock. Phase/lifecycle/final events flush the latest
  observation first. Non-scan events are never coalesced. Sender/receiver close
  wakes waiters and full-queue receiver drop releases blocked senders.
- TUI applies synchronous dispatch refusal/preferences outcomes directly to its
  reducer, so the UI never blocks sending to its own full FIFO. It drains at most
  32 worker events before polling input. Idle polls do not redraw; worker changes,
  input and resize mark the screen dirty. Existing job-id and join-before-replace
  rules remain in place.
- Canceled Inventory workers now emit exactly one terminal (`JobCanceled`),
  removing the formerly ignored subsequent `InventoryFinished`. On event-loop
  teardown, receiver delivery closes before joining. Read-only work is canceled;
  already-dispatched Clean/Software uninstall/Optimize run waits for completion.
  Existing explicit user cancel effects are unchanged. No teardown mutation is
  implicitly canceled by a draw/input error.
- `status::spawn_live` returns exported `StatusEventReceiver`, preserving the
  `recv_timeout`/standard timeout-error API used by CLI/TUI/Tauri. Its dedicated
  lossless channel holds 16 normal records plus at most one canceled in-flight
  record and one terminal. Backpressure waits check cancellation every 25 ms;
  catch-up ticks stop after cancellation. This lets `LiveControl::join` finish
  while the receiver remains alive and undrained, preserving every accepted
  sequence and the terminal for later reads. Producer drop wakes disconnection;
  receiver drop wakes a blocked send. No output/audit flush or fsync change.

## Deterministic regressions

- 100 discoveries at one logical instant and polls at 99/100 ms produce exactly
  two materialized snapshots (1 then 100 targets); final ordering retained.
- Terminal versus interactive scan of 151 targets: zero terminal progress events,
  identical complete internal plan and health.
- File-only traversal canceled by a probe stops before entry inspection and emits
  a typed cancellation diagnostic and coverage status.
- 10,000 TUI preview replacements retain one latest value; phase/latest/terminal
  order preserved. Full-FIFO lifecycle records remain ordered and lossless.
- A burst yields after 32 events with a pending preview still available. 100 idle
  render checks perform one actual TestBackend draw; marking dirty redraws once.
- Full-mailbox synchronous dispatch rejection returns without blocking. Fake
  cleanup teardown waits for release with its cancel flag unset; read-only worker
  cancellation still occurs. No cleanup command is run by these tests.
- Status full-queue cancellation retains exactly 18 ordered records including
  terminal; receiver close unblocks pending send. Actual `LiveControl::join`
  runs against a full queue with the receiver alive and no prior reads, then all
  records drain in sequence. The synthetic producer uses the production channel
  and control, avoiding OS probe/timing dependence for the queue proof.

## Checks run

All Cargo commands used `--locked --target-dir dist` and were serialized with
the runtime owner. Last observed focused results:

| Check | Result |
| --- | --- |
| Core `--lib scan::` | 77 passed, 8.07 s |
| Core `--lib scan::tests --quiet` after staged-test hardening | 19 passed, 8.64 s |
| Core `--lib filesystem::sizing::tests` (runtime-owned changes) | 21 passed, 0.15 s |
| Core `--lib status::` | 37 passed, 0.52 s |
| CLI `--lib` | 191 passed, 1.69 s |
| CLI `--lib tui::runtime --quiet` final | 18 passed, 0.05 s |
| CLI `--test five_mode_contract cancellation_joins_status_live_and_leaves_no_producer_after_broken_pipe -- --exact` | 1 passed, 1.92 s |
| `cargo clippy -p devsweep-core -p devsweep-cli --all-targets -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| Scoped `git diff --check` | Passed; Git only reported line-ending normalization notices |

## Limits and remaining acceptance

No runtime speedup or native idle/latency pass is inferred from these unit tests.
The synthetic queue proofs establish bounded retention and cancel/join behavior;
the real CLI subprocess test establishes broken-pipe success and process exit.
Native slow sinks, TUI input/resize timing, large-result CPU/memory and equal-root
B3-to-C report comparisons still require the coordinated measurement runs.
A synchronous OS pipe write can remain blocked until its reader progresses or
closes; this patch bounds the producer handoff and removes producer-send join
deadlock, without claiming to interrupt an already blocked OS write.
