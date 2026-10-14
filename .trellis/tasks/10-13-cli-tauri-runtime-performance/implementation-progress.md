# Runtime implementation progress

## Frontend checkpoint — 2026-10-13

Frontend ownership handoff is ready. See
[rendering evidence](evidence/frontend-rendering.md) and
[corrected-reference search fix](evidence/frontend-reference-correction.md).

- Corrected B preserved after display-only search/full-report authority repair.
- Measured all-row C1 still failed the 10k interaction budget; C2 adds 100-row
  display pages, lazy disclosures and stable group derivation without dropping
  targets or changing decoder/reducer authority.
- C2 1k/10k interaction microbenchmarks meet 100 ms; 10k review row/select-all
  p95 14 ms, fresh preview p95 13.5 ms. Full plan selection and all observations
  remain retained. Source/fixture/build hashes and raw failures are preserved.
- Full frontend lint/typecheck/generation, 429 Vitest tests, five Node tests and
  production build pass. Sixteen bilingual/width component captures have no
  document overflow. Existing large-chunk warning remains.

This checkpoint does **not** mark an entire RT acceptance criterion complete.
Native Rust/wire parity, transport retention/backpressure, extended alternating
pairs, host/tree resources and foreground/background lifecycle work remain
owned by the runtime/integration agents. No actual user cleanup was performed.

## Backend continuation checkpoint — 2026-10-13

Corrected B3 source/CLI/native identities and packaged Clean parity are recorded
in `evidence/runtime-resumption.md` and the functional child's native acceptance.
This gate passed before the following production performance edits.

- `ScanProgressPolicy::{TerminalOnly, Interactive}` now controls intermediate
  work. CLI Clean selects terminal-only; interactive clients keep an immediate
  first discovery, 100 ms cadence and phase/cancellation boundary flushes.
- All 77 scanner tests pass, including equal final plans/health between policies
  and a controlled 100-discovery workload with two published snapshots.
- Project traversal already streamed names. Its change is the missing per-entry
  cancellation checkpoint, with a typed canceled outcome for file-only walks.
- Shared sizing streams nested names and retains at most 64 root names per
  batch through the existing two-worker pool. All 21 sizing tests pass, including
  fresh child budgets, singleton/chunk boundaries and cancellation.
- The core raw-preview identity helper is available to Tauri. Desktop retains
  one raw pending preview and projects only at emission, while preserving every
  input's validation. The stale pending-after-new-send regression is covered by
  new tests; its owner still needs to report the focused desktop gate.

TUI/Status backpressure, final native lifecycle and repeated release comparisons
are still in progress. No timing benefit or complete acceptance is inferred
from these structural test results.

## Integrated delivery checkpoint — 2026-10-13

- Scanner/TUI/Status implementation and focused regressions are recorded in
  `evidence/scanner-tui-status-implementation.md`. TUI bounds its ordered FIFO
  at 32 plus one latest observation, services input after a finite drain and
  redraws only when dirty. Status retains at most 16 normal records plus one
  canceled in-flight record and one terminal, with lossless cancel/join tests.
- B3 native renderer pauses reproduced an IPC backlog despite the 100 ms
  producer throttle: 3 s/6 s pauses produced 20/26 messages in the first 100 ms
  after resume. The pinned Tauri implementation's private queue is unbounded;
  see `research/tauri-channel-retention.md` for source and measurement limits.
- Native scan/rescan now permit one unacknowledged progress packet. Closed
  frontend decode and listener success precede acknowledgment; cancellation
  wakes the worker, and final settlement waits for the last acknowledgment.
  Main-page reload and native destruction/exit cancel read-only scanning;
  normal minimize must still be verified to continue requested work.
- Native delivery's 14 focused tests pass. Frontend lint/typecheck, 432 Vitest
  tests, five Node tests, production build and generated types check pass with
  Node 22.23.2/npm 10.9.8 (`dist/frontend-integrated-ack-gate.txt`).
- Independent review reproduced and is correcting boundary defects in sizing
  cancellation and progress publication's interaction with the freshness
  guard. Final source freeze, root quality gate and C measurements follow those
  regressions; earlier checkpoint counts are not final integrated approval.

## Integrated delivery checkpoint — 2026-10-13

- Scanner/TUI/Status implementation and focused regressions are recorded in
  `evidence/scanner-tui-status-implementation.md`. TUI bounds its ordered FIFO
  at 32 plus one latest observation, services input after a finite drain and
  redraws only when dirty. Status retains at most 16 normal records plus one
  canceled in-flight record and one terminal, with lossless cancel/join tests.
- B3 native renderer pauses reproduced an IPC backlog despite the 100 ms
  producer throttle: 3 s/6 s pauses produced 20/26 messages in the first 100 ms
  after resume. The pinned Tauri implementation's private queue is unbounded;
  see `research/tauri-channel-retention.md` for source and measurement limits.
- Native scan/rescan now permit one unacknowledged progress packet. Closed
  frontend decode and listener success precede acknowledgment; cancellation
  wakes the worker, and final settlement waits for the last acknowledgment.
  Main-page reload and native destruction/exit cancel read-only scanning;
  normal minimize must still be verified to continue requested work.
- Native delivery's 14 focused tests pass. Frontend lint/typecheck, 432 Vitest
  tests, five Node tests, production build and generated types check pass with
  Node 22.23.2/npm 10.9.8 (`dist/frontend-integrated-ack-gate.txt`).
- Independent review reproduced and is correcting boundary defects in sizing
  cancellation and progress publication's interaction with the freshness
  guard. Final source freeze, root quality gate and C measurements follow those
  regressions; earlier checkpoint counts are not final integrated approval.
