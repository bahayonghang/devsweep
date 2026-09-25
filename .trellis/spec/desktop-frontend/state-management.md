# State Management

## Owner

`desktop/src/app-shell/` owns typed route registration, focus restoration, and
shell presentation state. `desktop/src/state/operation-coordinator.ts` owns the
frontend heavy-operation lifecycle seam. Each mode owns its reducer beneath its
mode directory; Clean's existing reducer remains under `desktop/src/state/` and
retains every authority invariant below. Components render state and dispatch
intents. `desktop/src/api/` owns Tauri calls and event subscription. No component
may call `invoke` or `listen` directly.

Shell state contains no targets, plans, selections, digests, confirmation
authority, or mode results. Unavailable mode registrations are absent rather
than represented by disabled placeholder state. The shell never derives counts,
sizes, or status for capsule tabs or brand-menu items.

## Shell Routing And Mode State

- Primary mode tags are the closed set Clean, Software, Optimize, Analyze, and
  Status. Protection, Rules, History, and Settings are supporting destinations.
- One typed registry owns route ids, paths, localized label keys, availability,
  and render adapters. Deep-link parsing fails closed to the first available
  route and never materializes an unavailable page.
- Browser history/back uses the same registry and does not recreate domain
  state. Each registered mode keeps its own state across route and language
  changes.
- Route transitions capture the activating navigation element and restore focus
  after composition. Mode routes restore focus to their capsule tab. Supporting
  destinations and Language open from the brand menu; the menu closes before
  navigation, so those routes restore focus to the brand button. Closing
  settings restores its opener.
- The supporting-route back control navigates to the last active mode route
  through the same registry and coordinator path as a capsule tab.
- Stage/detail view choice is mode-local presentation state. It never changes
  plans, selections, digests, confirmation state, or operation ownership.

## Operation Coordinator

The coordinator accepts only a typed operation kind, opaque operation id,
cancellation callback, and join promise. It serializes Scan, Analyze, the Analyze
Recycle Bin move (`analyze.trash`), Software, Optimize, and live Status work. Switching mode, closing/unmounting, or starting
another heavy operation requests cancellation and awaits join before new work
starts. Rapid requests are ordered; stale completions/events from superseded ids
are ignored. Close drains owned work. Effect replay and effect dependency
changes do not close the coordinator still held by App. The unmount drain is
one microtask later. Native close still drains immediately through
`requestClose`. A Clean null lease while the workbench is mounted dispatches
the existing `io` command error `The desktop operation coordinator is closed.`
Clean sets its mounted flag true in the effect setup and false in that
effect's cleanup. StrictMode replays the cleanup before the next setup, so
the setup restores the flag before a click can run.
The coordinator never grants domain
authority or sees plans, digests, selected paths, commands, or audit records.

## Presentation Settings

Core is the sole persisted-state owner. `PresentationSettingsV1` stores only a
closed `PresentationLanguageTag` (`en` or `zh-CN`) at
`%LOCALAPPDATA%\DevSweep\settings\presentation-v1.json` as exact closed JSON
`{"schema_version":1,"language":"en"|"zh-CN"|null}`. The core store uses an
OS-visible cross-process transaction lock, same-directory flushed temporary
file, and atomic replace. Missing `LOCALAPPDATA` makes persistence unavailable.
Invalid UTF-8/JSON, unknown fields/tags, and unknown/newer versions preserve the
original bytes, report unavailable, and are never overwritten or mapped to
English.

Desktop and TUI adapters exhaustively map the two core tags to the CLI-owned
runtime locale/catalogue and pure precedence resolver. Core never imports CLI.
The persisted selection affects interactive presentation only; explicit CLI
`--language` remains session-only, and JSON/NDJSON/machine schemas never consult
the persisted UI setting.

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
- Row Skip and Protect are review-local. `skippedIds` is cleared by a new scan;
  `protectedIds` is cleared only when a completed report replaces the plan.
  Both sets exclude their ids from selection, select-all, and restore. Skip,
  Protect, and Restore use the selection-invalidation path, and Protect during a
  pending dry run also drops that dry run. Protect calls
  `protectionAdd(path, true)` only after its confirmation dialog, only for rows
  with a path; it does not use the operation coordinator.
- The Clean cumulative total comes only from `historyCleanTotals()` after a
  matching execution report. A failed read hides the line; it never blocks the
  result.
- Analyze keeps the operation id of its terminal snapshot as
  `snapshotOperationId`. Reveal and trash calls send only that id and node
  ids; they never send a path. Analyze is read-only by default; the only
  mutation is a Recycle Bin move through a core-built plan, a live digest,
  and the second confirmation.
- The Analyze trash flow is `previewing -> reviewed -> confirming ->
  executing -> reported`. A preview is accepted only for the current
  `snapshotOperationId`. Execute is ignored unless a reviewed preview with
  at least one item exists and the confirmation dialog is open. The digest
  sent to `analyze_trash_execute` is copied only from that preview.
- `movedIds` holds node ids from `moved_node_ids` of an execution report. The
  display projection sets moved subtrees to zero bytes and subtracts their
  bytes from ancestors. It never edits the retained snapshot; a new analysis
  clears `movedIds`, and the stage suggests a new analysis for exact totals.
- Refusal reasons from a preview are review-local hints for the context menu.
  Before a preview, the menu derives a hint from the node kind and depth
  only; that hint is never authority.

## Async Effects

- Create one Tauri `Channel<DesktopScanProgress>` for each `scan_start` command.
  Do not use the application-global event bus for scan progress.
- Keep command promises in `App` effect handlers. Dispatch request/success/fail
  actions around each call; do not store promises in state.
- Disable conflicting commands while scanning, dry-running, or executing.
- Ignore late, duplicate, out-of-order, or mismatched progress and terminal events.
- Register every heavy mode effect with the shared coordinator before invoking
  its mode-owned service. Cancellation requests ownership; only joined terminal
  settlement releases the permit. A route change is not proof of cancellation.
- On language change, persist through the single core-backed command, update the
  presentation adapter only after a successful write, and retain all mode-local
  workflow state. Persistence failure is visible and cannot overwrite unreadable
  bytes.
- A channel decode failure requests cancellation but keeps the run active until
  its command promise settles. Only then may the reducer store a failed stopped
  preview, preventing a replacement scan from racing the still-running backend.

## Status Process Table And Tray HUD

- Status sort and pins are mode-local reducer state. `processes.ts` owns pure
  selectors: sort by CPU, private bytes, name, or PID in either direction, and
  expose the active column through `aria-sort` on its header cell.
- Pins hold at most 5 `(pid, name)` pairs. Each snapshot reconciles them: a
  PID absent from a complete row set, or reused by another name, shows
  `exited` once and is removed on the next snapshot; a PID absent from a
  truncated or partial row set shows `unsampled`. Pins never kill a process or
  change its priority.
- The tray HUD is a second Vite entry (`hud.html`, `src/hud/`). It renders
  only closed-decoded `hud-status` events through `listenHudStatus` and
  invokes one command, `presentation_settings_get`, through the i18n settings
  bridge. The Tauri invoke gate rejects every other app command from the
  `hud` window, and its capability grants only event listen/unlisten. The HUD
  does not use the operation coordinator; the backend `HudSampler` owns its
  sampling lifecycle.

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

Shell/coordinator tests additionally cover typed route/deep-link/back behavior,
unavailable-route absence, focus restore, mode-local state retention across
locale changes, rapid switching, cancel request, join-before-start, stale
events/completions, unmount/close, and zero surviving owned work. Store contract
tests cover exact V1 bytes/path, both frontend adapters, concurrent writers,
missing `LOCALAPPDATA`, corrupt/unknown/newer byte preservation, exhaustive tag
mapping, and machine-output isolation.
