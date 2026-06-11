# Optimize TUI startup loading performance

## Goal

Make `devsweep tui` reach a useful first screen quickly when it is launched
from the normal development path, while preserving the existing safety model
and the requirement that startup scanning eventually discovers project and
global cleanup targets without user input.

The user-reported symptom is that every TUI launch stays in the initial
loading/empty-target state for a long time. The screenshot shows the Dashboard
with `Jobs 1`, no targets, and a running startup scan.

Root-cause hypothesis for implementation:

> The TUI currently requests one startup scan that runs project scanning and all
> global provider discovery in a single worker; the visible target list remains
> empty until that worker sends one final `ScanFinished` event. On the debug
> development path, global provider size estimation recursively walks very large
> cache trees and dominates startup latency.

This is a planning task. It creates the implementation contract only; code
changes require an explicit start/implementation step.

## Confirmed Facts

- `src/tui.rs::run_event_loop` creates `App::new()`, calls
  `app.startup_effects()`, and dispatches the resulting scan before entering
  the draw loop.
- `src/tui.rs::startup_effects` always creates one `StartScan` job labeled
  `Scan current directory and globals`.
- `src/tui.rs::run_scan_worker` calls `scan_current_workspace`.
- `src/tui.rs::scan_current_workspace` first scans the current directory with
  `ProjectScanner`, then appends `GlobalProviderScanner::scan()` results.
- `src/tui.rs::ScanFinished` replaces `App.targets` only after the whole plan
  is ready. There is no partial target update event.
- `src/providers.rs` global scanning resolves npm, pip, pnpm, Yarn, and Cargo
  home providers, and estimates each available cache tree recursively.
- Debug/development timing on this machine:
  - `target\debug\devsweep.exe scan --projects`: about 0.7 s.
  - `target\debug\devsweep.exe scan --global`: observed about 47-73 s.
  - `target\debug\devsweep.exe scan`: observed about 73 s.
- Release timing on this machine:
  - `target\release\devsweep.exe scan --projects`: about 0.8 s.
  - `target\release\devsweep.exe scan --global`: about 1.3 s.
- The measured global targets are npm cache, pip cache, pnpm store, and Cargo
  home inspect-only.
- Measured cache shape on this machine:
  - npm cache: about 9.7 GB and 241k files.
  - Cargo home: about 5.6 GB and 155k files.
  - pip and pnpm are much smaller.
- Provider command lookup itself is not the dominant cost in the measured
  shell checks: npm, pip, and pnpm path commands returned in under one second.
- Existing tests already assert that startup requests one initial scan and that
  worker `ScanFinished` updates jobs and targets.

## Requirements

- Preserve automatic startup scanning. Users should not need to press `s` just
  to discover targets after opening the TUI.
- Improve first-screen usefulness. The TUI should not leave the Dashboard in a
  long-lived empty-target state when a slow global scan is still running.
- Separate fast project results from slow global size estimation where possible,
  so the UI can show useful discovered targets incrementally.
- Keep scan behavior non-mutating. Scanner and provider layers must only create
  cleanup plans or target snapshots.
- Keep cleanup execution semantics unchanged.
- Keep command-backed cleanup safety unchanged. Global provider targets must
  continue to use command actions or inspect-only actions as they do today.
- Preserve current selected-by-default safety, including the guard that prevents
  selecting the current executable target.
- Make slow work visible. The UI should show what phase is running instead of a
  generic long `Jobs 1` state.
- Include regression coverage that proves startup can surface partial or staged
  scan results without waiting for the slowest global provider path.
- Validate both focused TUI behavior and the repo's canonical gate before
  reporting implementation complete.

## Acceptance Criteria

- [ ] Launching the TUI still creates exactly one startup scan request.
- [ ] A fast project-scan result can update the visible targets before global
      provider scanning or global size estimation finishes.
- [ ] Slow global provider work no longer keeps the Dashboard in a long-lived
      empty-target state when project targets exist.
- [ ] Global provider targets still appear after the global phase completes.
- [ ] The Jobs/Logs surface names the active startup scan phase clearly enough
      to distinguish project scan, provider discovery, and size estimation.
- [ ] Manual scan with `s` still refreshes the current in-memory snapshot.
- [ ] Target replacement/merge behavior is deterministic and does not duplicate
      targets across staged scan updates.
- [ ] Cleanup execution, dry-run, audit, and model serialization behavior remain
      unchanged.
- [ ] Focused tests cover startup staged scan behavior and existing TUI scan
      tests continue to pass.
- [ ] `just ci` passes before the task is considered done.

## Notes

- The key performance problem is not terminal drawing. The first frame renders,
  but the scan result needed to populate targets is delayed.
- Debug/development launch matters because `just dev` runs `cargo run -- tui`,
  which uses the debug build by default.
- Release builds are much faster in current measurements, so the implementation
  should be careful not to overfit to debug-only micro-optimizations.

## Out Of Scope

- Executing cleanup actions during scan.
- Persistent on-disk scan cache across restarts.
- Enabling permanent delete.
- Docker cleanup.
- Rewriting the whole TUI layout.
- Changing the cleanup plan JSON contract unless implementation proves a small
  internal event type cannot solve the startup staging problem.

## Open Questions

- None blocking planning. The recommended implementation direction is staged
  startup scan events: project results first, then global provider results with
  clearer progress messages.
