# global scan cache targets

## Scope

This is an in-session UI state change, not persistence work.

The current TUI already keeps scan results in `App.targets`; the missing piece
is that `run()` starts with `CleanupPlan::empty()` and waits for user input
before scanning. The fix is to trigger one initial scan when the event loop
starts, then keep the resulting plan in `App` exactly as the manual scan path
does.

## Design

1. Add a startup effect from the TUI bootstrap path.
2. Reuse the existing `StartScan` worker effect and `ScanFinished` handler.
3. Keep all discovered targets in `App.targets` as the cached session
   snapshot.
4. Leave tab switching and filtering unchanged so views continue to render from
   the cached snapshot.

## Boundaries

- No new persistence layer.
- No disk cache.
- No new cleanup semantics.
- No changes to scanner/provider discovery logic.

## Risks

- The initial scan may not finish before the first frame renders. That is fine;
  the UI should show the scan job until results arrive.
- Startup should not queue repeated scans. The bootstrap path must trigger the
  scan once per TUI session.

## Verification Shape

- A unit test should prove the startup path requests an initial scan.
- Existing worker-event tests should continue to prove scan results replace the
  in-memory target list.
- Manual TUI check should show the global targets after the scan completes.
