# Design: keep the desktop coordinator open across the dev remount

## Boundary

`OperationCoordinator` stays the only heavy-operation permit. `close()`
still means the mounted tree is gone or the native window is closing.
`DesktopLifecycleController.drain()` stays the single close entry. The
native close listener keeps calling `requestClose()` directly
(`desktop/src/lifecycle.ts:57-59`). That path stays immediate.

The defect is the effect cleanup in `desktop/src/App.tsx:133-136`.
Development `StrictMode` runs that cleanup and then runs the effect again
on the same `App` state, including the same coordinator from `useMemo`.
`drain()` latches `drainPromise`, so the second setup cannot undo the
close.

## Drain guard

Keep a generation ref next to the effect. Increment it at the start of
setup. Cleanup remembers that generation and schedules the drain on a
microtask. The microtask calls `drain()` only when the ref still equals
the closed generation.

React 19 runs the StrictMode cleanup and the second setup in the same
flush, before microtasks. The second setup increments the ref, so the
microtask sees a newer generation and skips `drain()`. A real unmount has
no later setup, so the microtask drains. A dependency change does the
same skip: the replacement effect still owns the coordinator.

```ts
const generation = ++lifecycleGeneration.current;
// setup uses `generation` when deciding to keep `unlisten`
return () => {
  disposed = true;
  unlisten?.();
  const closedGeneration = generation;
  queueMicrotask(() => {
    if (lifecycleGeneration.current === closedGeneration) {
      void lifecycleController.drain();
    }
  });
};
```

Keep the existing `disposed` handling for a close listener that resolves
after cleanup. Do not move the native `requestClose()` path onto this
microtask.

## Null lease on the mounted Clean workbench

`start()` returning `null` remains "do not call the bridge". While
`mounted.current` is true, Clean also dispatches `command_failed` with
`commandError(new Error("The desktop operation coordinator is closed."))`.
`ErrorBanner` already renders `io` as
`Desktop operation failed: ${error.message}`
(`desktop/src/components/ErrorBanner.tsx:25`).

Apply that dispatch at all three Clean lease checks:

- `desktop/src/modes/clean/CleanWorkbench.tsx:92` scan
- `desktop/src/modes/clean/CleanWorkbench.tsx:132` dry-run
- `desktop/src/modes/clean/CleanWorkbench.tsx:162` execute

After unmount, `mounted.current` is false and the dispatch is skipped.
No new error code. `desktop/src/api/types.gen.ts` stays unchanged
(`.trellis/spec/desktop-frontend/type-safety.md`).

Software, Optimize, Analyze, and Status keep their current `if (!lease)
return` behavior. Their reducers drop `operation_failed` /
`analysis_failed` when `operationId` does not match
(`desktop/src/modes/software/state.ts:342-343`,
`desktop/src/modes/optimize/state.ts:230-231`,
`desktop/src/modes/analyze/state.ts:139-142`,
`desktop/src/modes/status/state.ts:263-264`). A null lease never sets
that id, so a new banner there needs a new idle action. R4 covers those
modes by keeping the coordinator open, which is the shared cause.

## Tradeoff

Deferring the unmount drain by one microtask is the whole compatibility
cost. `waitFor` in the existing unmount tests already retries across
microtasks. The native close test calls `requestClose()` and must stay
one `close()` without waiting for the effect cleanup.

Removing `StrictMode` would hide only the current entry. The next effect
re-run would close the session again. A new coordinator after close would
split permit identity across a tree that still holds the old one.

## Rollback

Revert the `App` cleanup guard and the three Clean dispatches. The
coordinator returns to closing inside the effect cleanup. No data
migration and no IPC change.
