# Optimize TUI startup loading performance - Progress

## 2026-06-11

- Created the Trellis task after the user reported that the TUI stayed in the
  startup loading/empty-target state for a long time and requested an
  optimization task.
- Diagnosed the startup path:
  - `App::startup_effects` starts one scan job on launch.
  - `run_scan_worker` previously returned only one final `ScanFinished` event.
  - `scan_current_workspace` previously combined project scanning and global
    provider scanning before updating `App.targets`.
- Measured the performance shape on this machine:
  - Debug project-only scan: about 0.7 s during planning; 0.56 s after
    implementation smoke.
  - Debug global scan: observed about 47-73 s during planning.
  - Release global scan: about 1.3 s during planning.
  - npm cache and Cargo home are the large local cache trees.
- Wrote `prd.md`, `design.md`, and `implement.md`.
- Started the task after explicit user approval.
- Loaded Trellis pre-development guidance plus frontend/backend specs for TUI
  render purity, event/update boundaries, state management, type safety, and
  global provider safety.
- Implemented staged startup scan handling in `src/tui.rs`:
  - `run_scan_worker` now emits scan progress through project and global phases.
  - Project scan results are sent as a partial plan before global provider work.
  - Global scan results are sent as a later phase update and still feed the
    final `ScanFinished` completion.
  - `App` now tracks an internal per-scan `ScanSnapshot` so project and global
    targets merge deterministically.
  - Manual scan starts a fresh scan snapshot.
  - Stale scan jobs cannot overwrite a newer scan's visible targets.
  - Jobs/Logs now receive clearer phase messages such as project completion and
    global size-estimation progress.
- Added focused TUI regression tests for:
  - showing project targets before global scan finishes
  - merging project and global scan phases without duplicates
  - ignoring stale scan completions after a newer scan starts
- Updated `.trellis/spec/frontend/hook-guidelines.md` with the reusable staged
  worker progress contract and test requirements.
- Validation passed:
  - `cargo test tui::tests --all-targets`
  - `cargo fmt --all -- --check`
  - `git diff --check`
  - `just ci`
