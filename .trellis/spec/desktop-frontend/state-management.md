# State Management

## Owner

`desktop/src/state/` owns the reducer and workflow invariants. Components render
state and dispatch intents. `desktop/src/api/` owns all Tauri calls and event
subscription. No component may call `invoke` or `listen` directly.

## Workflow

The reducer represents these meaningful states, with preview observations and
completed cleanup authority owned separately:

```text
idle -> scanning(active preview) -> reviewed(completed report) -> dry_run -> confirming -> executing -> reported
                  -> canceled/failed(stopped read-only preview)
```

Failures retain the last safe report/selection where recovery is possible and
store one structured user-facing error. Cancel is a request while scanning; the
scan result or backend error remains authoritative.

## Invariants

- Starting a scan creates a fresh opaque scan id, clears current selection and
  dry-run/confirmation authority, and may retain a previous completed report only
  as a hidden fallback.
- Progress is accepted only for the active scan id and a sequence greater than
  the last accepted sequence. Cumulative previews replace one another; consumers
  never append target deltas.
- Active and stopped previews are observation-only. They never enter `scan`,
  `selectedIds`, dry-run, confirmation, or execution state.
- A completed scan replaces the plan and projects selection exactly from
  `selected_by_default`, excluding inspect-only targets.
- Cancellation and failure retain the latest safe preview, but only a matching
  completed report can promote targets to review authority. A failed rescan does
  not destroy the previous completed report; stopped-preview UI provides an
  explicit transition back to that report without promoting preview targets.
- Selection is a set of target IDs intersected with current executable targets.
- Any selection mutation clears `dryRun`, confirmation state, and execution
  result synchronously in the same reducer transition.
- Starting or completing a rescan clears the previous dry-run digest before the
  asynchronous command starts.
- `ExecuteRequested` is ignored unless a current dry run exists, selection is
  non-empty, and the confirmation dialog is open.
- The digest sent to `plan_execute` is copied only from the current dry-run
  response. The frontend never computes or edits a digest.
- `stale_confirmation` clears dry-run state and returns to review with explicit
  recovery copy.
- Inspect-only targets are rejected again by reducer helpers even if a component
  dispatches an invalid selection intent.

## Async Effects

- Create one Tauri `Channel<DesktopScanProgress>` for each `scan_start` command.
  Do not use the application-global event bus for scan progress.
- Keep command promises in `App` effect handlers. Dispatch request/success/fail
  actions around each call; do not store promises in state.
- Disable conflicting commands while scanning, dry-running, or executing.
- Ignore late, duplicate, out-of-order, or mismatched progress and terminal events.
- A channel decode failure requests cancellation but keeps the run active until
  its command promise settles. Only then may the reducer store a failed stopped
  preview, preventing a replacement scan from racing the still-running backend.

## Scenario: Command-scoped scan preview

### 1. Scope / Trigger

Apply this contract whenever a desktop scan exposes in-progress discoveries to
the webview. It preserves the boundary between observation-only preview data and
the completed report that alone authorizes selection, dry-run, or execution.

### 2. Signatures

```rust
async fn scan_start(
    state: State<'_, ScanCoordinator>,
    scan_id: String,
    options: ScanOptions,
    on_progress: Channel<DesktopScanProgress>,
) -> Result<DesktopScanResult, CommandError>;

async fn scan_cancel(
    state: State<'_, ScanCoordinator>,
    scan_id: String,
) -> Result<(), CommandError>;
```

`DesktopScanProgress` contains `scan_id`, monotonically increasing `sequence`,
`phase`, `message`, and an optional cumulative `ScanPreviewSnapshot`.
`DesktopScanResult` is tagged as `completed { scan_id, report }` or
`canceled { scan_id }`.

### 3. Contracts

- `scan_id` is non-empty, opaque, and unique per frontend invocation.
- `sequence` is positive and strictly increases within one scan.
- Each preview is a complete cumulative replacement, not a target delta.
- The preview contains display identity, scope, ecosystem, capacity, risk,
  evidence, disposition, and aggregate totals. It omits plan/action fields and
  default selection.
- The Tauri boundary may coalesce preview delivery to at most 10 sends per
  second, but must flush the latest pending preview before phase changes and the
  terminal command result.
- The reducer accepts progress and terminal results only for the active scan id.
  Only a matching completed result promotes a report into cleanup authority.
- If channel decoding fails, request cancellation and await the command result
  before clearing the active run or allowing a replacement scan.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Empty `scan_id`, zero/unsafe sequence, unknown field, duplicate target id, or inconsistent totals | Decoder rejects the envelope closed; request cancellation and wait for terminal settlement |
| Progress id differs from the active id | Ignore without mutating preview or report state |
| Sequence is duplicate or older | Ignore without mutating preview or report state |
| User cancels | Keep the latest preview read-only until the matching terminal result arrives |
| Backend returns `canceled` | Store a stopped read-only preview; never project selection |
| Backend returns `completed` | Atomically replace the preview with the completed report and project valid defaults |
| Rescan stops while an older report exists | Offer an explicit return to that report; never merge the stopped preview into it |

### 5. Good/Base/Bad Cases

- Good: project targets appear incrementally as cumulative previews, global
  scanning starts afterward, and the final report atomically enables review.
- Good: a failed rescan keeps its discoveries read-only and lets the operator
  return to the previous completed report.
- Base: a cancellation request races completion; the matching terminal result,
  not the button click, determines whether the run completed or canceled.
- Bad: a malformed channel message immediately clears `scanning` and permits a
  second scan while the backend coordinator still owns the first run.
- Bad: preview rows inherit `selected_by_default` or produce a cleanup plan.

### 6. Tests Required

- Rust tests assert per-target cumulative publication, strictly increasing
  sequences, bounded channel sends, pending-preview terminal flush, and truthful
  requested phases under cancellation.
- Contract tests reject unknown/authority-bearing fields, duplicate ids,
  malformed totals, invalid scan ids, and invalid sequences.
- Reducer tests assert id/sequence rejection, preview/report isolation, stopped
  preview retention, previous-report recovery, and final-only promotion.
- App tests assert decode failure cancels and awaits settlement before another
  scan is enabled.
- Component tests assert the visual stream may update at the transport rate,
  while same-phase polite announcements are coalesced to at most 1 Hz.

### 7. Wrong vs Correct

```ts
// Wrong: releases the frontend while the backend scan may still be running.
catch (error) {
  dispatch({ type: "ScanFailed", error });
}

// Correct: cancel ownership, await the terminal command, then classify failure.
catch (error) {
  await requestCancel(activeScanId);
  await scanCommandSettlement;
  dispatch({ type: "ScanFailed", scanId: activeScanId, error });
}
```

## Tests

Reducer tests must cover scan-id/sequence rejection, preview/report isolation,
cancel/failure retention, prior-report fallback, atomic final promotion, default
selection, inspect-only rejection, digest invalidation, stale-confirmation
recovery, and execution gating. App/API tests use an injected bridge and must
cover progress, cancel, structured errors, dry-run, confirmation, and result
rendering without real cleanup side effects.
