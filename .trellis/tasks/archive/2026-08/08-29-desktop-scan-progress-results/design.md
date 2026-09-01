# Design: continuous safe scan previews

## 1. Decision Summary

The desktop will show phase-aware indeterminate Scan Progress and cumulative,
continuously updated Scan Previews. A preview is an incomplete read-only projection;
it is not a Scan Report, Cleanup Plan, or source of cleanup authority. The final
report alone enables default selection and the existing dry-run/confirmation flow.

This design implements the accepted boundary in
`docs/adr/0001-separate-scan-preview-from-cleanup-authority.md` and uses the domain
language in `CONTEXT.md`.

## 2. Ownership And Data Flow

```text
ProjectScanner / GlobalProviderScanner
  emit fully constructed, post-safety target checkpoints
                    |
                    v
Sweeper progress accumulator
  cumulative + deduplicated + deterministic order
  internal CleanupPlan remains trusted process state
             |                         |
             |                         +--> CLI: ignore progress, final report unchanged
             +--> TUI worker: job-scoped internal snapshots
             |
             v
Core-owned safe ScanPreview projection
  purpose-built observation facts; no plan/intent/default selection/CleanAction
                    |
                    v
command-scoped Tauri Channel<DesktopScanProgress>
  scan_id + sequence + phase + message + preview?
                    |
                    v
strict TS decoder -> reducer.preview (never reducer.scan)
                    |
                    v
read-only grouped preview -> final ScanReport atomically replaces it
```

Ownership rules:

- Scanners discover and size targets; they never render, select, or execute.
- `Sweeper` remains the sole production owner of merge, deduplication, ranking,
  and cumulative snapshot semantics. Consumers replace snapshots; they do not
  append/merge target deltas.
- Core owns the conversion from internal `CleanupPlan`/`CleanAction` to the
  display-safe preview shape. Tauri transports it; React only decodes and renders.
- Tauri owns scan-run correlation and IPC delivery. TUI continues to add its
  existing job id outside the core contract.
- React state owns the preview/final-report state machine. Components never call
  Tauri directly and never reinterpret cleanup authority.

## 3. Contracts

### 3.1 Internal progress

Keep `ScanProgress` frontend-neutral and capable of carrying an internal cumulative
`CleanupPlan`. Extend project and global scanning so `Sweeper` can publish useful
snapshots before a whole phase completes. Every published target must already have
passed the scanner's path/reparse/rule checks and completed its current bounded size
observation.

Progress invariants:

1. Snapshots are cumulative for one scan and contain unique target ids.
2. Each snapshot is deduplicated and deterministically ordered by core.
3. The first useful target is eligible for immediate publication; later changes
   are coalesced by a bounded deterministic policy, with a forced latest flush at
   phase completion, cancellation, and terminal completion.
4. Phase/message events may carry no preview when only lifecycle text changed.
5. Thread the cooperative cancellation token through ordinary project target
   sizing as well as traversal and global probing, so one large bounded size walk
   cannot freeze preview or cancellation.
6. Return an explicit terminal kind for completed versus canceled work. Cancellation
   does not promote an incomplete preview to a complete report; a completed report
   with partial health remains distinct from cancellation. Add a post-global
   cancellation regression because the current global interface carries only a plan.

The implementation may refactor private scanner callback signatures, but it must
not make individual consumers merge target deltas or rank independently.

### 3.2 Safe preview projection

Add a serializable core-owned display DTO equivalent to:

```rust
pub struct ScanPreviewSnapshot {
    pub targets: Vec<ScanPreviewTarget>,
    pub totals: ScanPreviewTotals,
}

// Observation-only: id, scope, ecosystem, kind, path, observed capacity and
// confidence, mtime, risk, evidence, and warnings.
// Deliberately omits CleanAction, cleanup intent, and selected_by_default.
pub struct ScanPreviewTarget { /* typed display facts only */ }
```

The preview envelope is purpose-built and must not be structurally reusable as an
`UntrustedPlan`: it has no plan version, cleanup intent, selection default, or
selection collection. Core may reuse canonical validation/sanitization helpers, but
Tauri and TypeScript must not reconstruct or strip trusted actions. `totals` reports
observation counts and capacity confidence; it does not claim selected or estimated
recoverable capacity before the final report.

Projection failure is fail-closed: do not emit a malformed or partially stripped
preview. Preserve the first projection error and fail the scan command through its
structured error path if the final scan cannot establish the same invariant.

### 3.3 Desktop progress envelope

Use a desktop-specific DTO over a per-command Tauri `Channel` rather than
serializing core `ScanProgress` onto the application-global event bus:

```ts
interface DesktopScanProgress {
  scan_id: string;
  sequence: number;
  phase: "projects" | "global";
  message: string;
  preview: ScanPreview | null;
}
```

- React creates a fresh opaque scan id for `scan_start`; the id is correlation
  metadata only and grants no authority.
- Tauri assigns a strictly increasing sequence per accepted scan and includes the
  id/sequence in every channel message.
- The strict decoder rejects unknown fields, unsafe integers, malformed previews,
  and any authority-bearing fields through the existing closed-world target
  decoder.
- The reducer accepts an event only when `scan_id` equals the active run and
  `sequence` is greater than the last accepted sequence.
- `scan_start` returns a correlated terminal result with an explicit `completed`
  or `canceled` kind; structured failures also retain scan correlation. A completed
  result carries the final `ScanReport`, while canceled/failed results never promote
  the last preview.
- The existing `ScanReport` and saved JSON versions do not change. The manifest's
  `tauri = "2.11.3"` caret requirement resolves to Tauri 2.11.5 in `Cargo.lock`;
  that lock-resolved channel signature and final-message-before-command-resolution
  ordering are implementation gates, not assumed planning evidence.

### 3.4 Delivery policy

Transport must be latest-preserving and bounded. Use the pinned Tauri channel plus
existing standard-library primitives; no new production dependency is approved. The implementation
must choose a named, testable cap (no faster than ten preview emissions per second
under sustained discovery), publish the first useful preview promptly, and force
the latest pending snapshot at semantic/terminal boundaries. Time-sensitive code
must have an injectable or deterministic test seam; avoid flaky wall-clock sleeps.

## 4. Desktop State Machine

Extend state with separate active, stopped-preview, and completed-report owners:

```text
idle/completedScan?
  -> scanning { scanId, lastSequence, progress, preview?, cancelRequested }
                 + retained completedScan? (hidden fallback, never mixed)
  -> reviewed { completedScan: final ScanReport, selectedIds }
  -> dry_run -> confirming -> executing -> reported

scanning --failure/cancel without final report-->
  idle/recoverable { stoppedPreview?, structured error }
```

Reducer rules:

- `scan_requested(scanId)` clears selections, dry run, confirmation, execution,
  error, and older stopped preview, then seeds the active run. It retains any prior
  completed report as a fallback until matching final success atomically replaces it.
- `scan_progressed` applies only to the matching run and higher sequence. Preview
  state replaces the previous cumulative snapshot; it is never copied into `scan`.
- `scan_cancel_requested` changes copy/controls but retains progress and preview.
- `scan_completed(scanId, report)` must match the active run; it atomically clears
  preview identity, replaces the prior completed report, and projects default
  selection from final executable targets only. Partial scan health is still a
  completed report.
- `scan_canceled(scanId)` and `scan_failed(scanId, error)` retain the latest preview
  as stopped read-only evidence, clear active-run authority, and keep any prior
  completed report available as fallback. A later rescan discards only the stopped
  preview.
- Progress listener/decoder failures are distinguished from unrelated dry-run or
  execution failures so they cannot accidentally move the wrong workflow state.
- Late success, failure, or progress from older ids is ignored.

## 5. Presentation

### 5.1 Scan controls and progress rail

Retain the existing scope checkboxes and fixed Scan/Cancel action. Add a compact
second row or vertically composed center region containing:

- only the requested phase steps (`Projects`, `Global caches`), each with text and
  pending/current/complete semantics;
- a native `<progress>` element without `value` for the active phase;
- the exact backend message and a compact `N discovered so far` count;
- `Cancel requested; finishing the current safe boundary` after cancellation.

The live region announces phase/message/count changes, not target rows. Reduced
motion removes animation while phase text and progress semantics remain complete.

### 5.2 Preview workspace

When scanning with zero targets, render a purposeful active state such as
`Discovering cleanup targets…`; never render completed-empty copy. Once targets
arrive, render:

- heading `Discovered so far` plus incomplete/read-only explanation;
- scope sections `Projects` and `Global caches` with counts;
- compact ecosystem counts within the section/header;
- the existing target comparison columns for target, kind/category, capacity,
  risk, and evidence/warnings.

Refactor `TargetTable` around a discriminated `preview` versus `review` mode so
preview mode cannot receive selection callbacks at the type boundary. Preview mode
omits the selection column and action footer; safe filtering/group expansion may
remain interactive because it does not alter cleanup authority.

On completion, the normal `ReviewPage` replaces the preview. A zero-target final
report renders explicit completed-empty copy (`Scan complete; no cleanup targets
found`) rather than the pre-scan or active-scan state.

### 5.3 Responsive layout

- At 800×600, keep controls and progress visible without pushing the first preview
  rows below an oversized status panel.
- At 390×844, stack scope controls above the status/action row, make progress full
  width, keep the Cancel hit area stable, and contain table overflow within its
  frame.
- Do not introduce nested cards, a dashboard tile grid, viewport-sized type, or a
  page-level horizontal scrollbar.

## 6. Compatibility And Security

- Non-interactive CLI final text and JSON remain unchanged; it may continue to
  ignore progress callbacks.
- TUI receives more useful cumulative internal snapshots and retains job-id guards,
  replacement semantics, and final parity. While its scan job is active, staged
  targets are preview-only: selection, dry-run, and cleanup confirmation are blocked.
  `ScanFinished` alone promotes the final plan to selectable state.
- Desktop capabilities remain event listen/unlisten only; no filesystem or shell
  frontend capability is required.
- Preview data is still untrusted input if sent back by a compromised webview;
  existing Rust validation/digest/live authorization remain mandatory. UI locking
  is defense in depth and product correctness, not the backend trust boundary.
- No persisted plan/report migration, feature flag, or data backfill is required.

## 7. Validation Matrix

| Layer | Required proof |
| --- | --- |
| Core | multiple in-phase cumulative snapshots; dedupe/order; safe projection serialization; bounded/forced delivery; project/global cancel health |
| CLI/TUI | unchanged final CLI output; TUI replaces cumulative snapshots, ignores stale jobs, blocks active-preview authority, promotes only the final plan |
| Tauri | command-scoped channel; scan id/sequence forwarding; explicit terminal kind; single-flight/cancel behavior; no plan/intent/default-selection/action fields; terminal flush |
| API | deterministic generated types/fixtures; strict preview/id/sequence decoder; unknown/unsafe/authority field rejection |
| Reducer | matching id + monotonic sequence; preview/report separation; cancel/failure retention; prior completed-report preservation; final atomic replacement; stale terminal rejection |
| React | starting/live/cancel/error/empty-final/result-final states; categories/counts; absent preview selection/dry-run; accessibility |
| Visual | controlled fixture at 800×600 and 390×844; reduced motion; long paths/large list; no page overflow |
| Full tree | desktop generate/lint/typecheck/test/build, relevant Rust tests, `git diff --check`, and `just ci` |

## 8. Rollback

The final report and persisted plan contracts remain unchanged, so rollback is
code-only: remove the desktop preview envelope/projection and restore phase/message-
only events plus the old active-scan presentation. No user data migration or cleanup
rollback is involved. Preserve the ADR as historical context if the decision is
later superseded.
