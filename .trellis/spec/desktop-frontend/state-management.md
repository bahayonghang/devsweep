# State Management

## Owner

`desktop/src/state/` owns the reducer and workflow invariants. Components render
state and dispatch intents. `desktop/src/api/` owns all Tauri calls and event
subscription. No component may call `invoke` or `listen` directly.

## Workflow

The reducer represents these meaningful states:

```text
idle -> scanning -> reviewed -> dry_run -> confirming -> executing -> reported
```

Failures retain the last safe report/selection where recovery is possible and
store one structured user-facing error. Cancel is a request while scanning; the
scan result or backend error remains authoritative.

## Invariants

- A completed scan replaces the plan and projects selection exactly from
  `selected_by_default`, excluding inspect-only targets.
- Core progress partials are internal only. The desktop shell redacts them before
  emit, and the webview decoder accepts only `partial: null`; they never become
  executable authority.
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

- Register `scan://progress` once for the lifetime of the application effect and
  always call the returned unlisten function on cleanup.
- Keep command promises in `App` effect handlers. Dispatch request/success/fail
  actions around each call; do not store promises in state.
- Disable conflicting commands while scanning, dry-running, or executing.
- Ignore late progress events unless the reducer is currently scanning.

## Tests

Reducer tests must cover default selection, inspect-only rejection, digest
invalidation on selection and rescan, stale-confirmation recovery, and execution
gating. App/API tests use an injected bridge and must cover progress, cancel,
structured errors, dry-run, confirmation, and result rendering without real
cleanup side effects.
