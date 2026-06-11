# Optimize TUI startup loading performance - Design

## Diagnosis

The screenshot fits the current startup scan contract:

1. `run_event_loop` creates the app and immediately dispatches
   `startup_effects`.
2. `startup_effects` creates one scan job.
3. `run_scan_worker` calls `scan_current_workspace`.
4. `scan_current_workspace` builds a single `CleanupPlan` by running project
   scanning and global provider scanning.
5. `ScanFinished` updates `App.targets` only after the entire plan is ready.

The UI is not hard-frozen; it redraws and shows `Jobs 1`. The useful target
state is gated on the slowest part of the scan worker.

Measured on this machine, project scanning is fast while global scanning is the
dominant development-path delay:

- Debug project-only scan: about 0.7 s.
- Debug global scan: observed about 47-73 s.
- Release global scan: about 1.3 s.

Global provider commands are fast enough in shell checks. The large local cache
trees make recursive size estimation the main suspect:

- npm cache: about 9.7 GB, 241k files.
- Cargo home: about 5.6 GB, 155k files.

## Design Direction

Keep automatic startup scan, but make it staged.

The worker should be able to send useful partial results before global scanning
or global size estimation completes. The app should merge or replace scan
results by phase deterministically, while Jobs/Logs shows the active phase.

Recommended event shape:

- Add a scan-progress event that can carry a phase label and, when available,
  a partial `CleanupPlan`.
- Emit project results immediately after `ProjectScanner::scan_roots`.
- Emit progress before global provider discovery.
- Emit final global results when `GlobalProviderScanner::scan()` completes.
- Keep the existing final `ScanFinished` event as the completion marker, or
  evolve it to carry the final merged plan after staged updates.

The staged result boundary should stay internal to the TUI worker and `App`
state. `CleanupPlan`, `CleanTarget`, executor behavior, and JSON serialization
do not need to change for this task.

## Merge Contract

The app needs deterministic scan state so partial updates do not create
duplicates:

- Project-phase update owns all project-scoped targets from the active scan.
- Global-phase update owns all global-scoped targets from the active scan.
- Final state is the union of the latest project and global targets for the
  active scan job.
- Updates from stale scan jobs must not overwrite a newer scan job.
- Selection should be recomputed from the current visible target set using the
  existing `default_selected_ids` safety guard unless a manual selection
  preservation rule is intentionally added.

The simplest implementation is to keep per-job or app-level staged target
vectors for the current scan:

- `pending_project_targets`
- `pending_global_targets`

When either phase updates, rebuild `App.targets` from those slices in stable
order.

## Progress Contract

Jobs/Logs should show specific phases, for example:

- `Scanning current directory`
- `Project scan finished: N target(s)`
- `Scanning global providers`
- `Estimating global cache sizes`
- `Global scan finished: N target(s)`

The exact copy can be adjusted during implementation, but it should distinguish
project discovery from global provider work.

## Alternatives Considered

1. Remove startup global scanning.
   - This would make startup fast, but it breaks the earlier accepted task that
     made global targets available without pressing `s`.

2. Keep one final scan result and only improve the loading text.
   - This is low risk, but it does not solve the long empty-target state.

3. Add persistent disk caching.
   - This could improve repeated launches, but it adds invalidation,
     compatibility, and stale-size risks that are larger than needed for the
     current complaint.

4. Optimize `estimate_tree` only.
   - This may help debug builds, but it still keeps all visible results gated on
     the slowest global provider. It can be a follow-up if staged results are
     not enough.

## Boundaries

Allowed:

- `src/tui.rs` scan worker events, app scan state, job/log messages, and tests.
- Small private helper types in `src/tui.rs` if they keep staged scan state
  understandable.
- Focused scanner/provider tests only if implementation touches those modules.

Not allowed:

- Cleanup execution changes.
- Cleanup plan model/schema changes unless unavoidable.
- Persistent scan cache.
- Permanent delete behavior.
- Moving cleanup logic into render helpers.

## Verification Shape

- Unit tests for staged startup scan state in `src/tui.rs`.
- Existing startup scan and worker-event tests remain green.
- Focused timing smoke using `target\debug\devsweep.exe scan --projects` and
  `target\debug\devsweep.exe scan --global` can be recorded as diagnostic
  evidence, but pass/fail should rely on deterministic state tests.
- Finish with `just ci`.

## Rollback

Rollback should be local to `src/tui.rs` and its tests if the implementation
keeps the model and scanner/provider contracts unchanged. Reverting the staged
event/state additions should restore the current single-result startup scan.
