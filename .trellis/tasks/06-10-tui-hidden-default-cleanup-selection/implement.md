# Implementation Plan

## Phase 1: Guard Execution

1. Add a small helper in `src/executor.rs` to determine whether a target path
   contains `std::env::current_exe()`.
   - Verify with unit tests using temporary paths or test-only helper inputs.
   - Done in `src/path_safety.rs` and reused by executor/TUI.
2. In `execute_target`, before command execution, return
   `ActionStatus::Skipped` when the selected target contains the running
   executable.
   - Verify progress/audit tests report `skipped`, not `failed`.
   - Done in `src/executor.rs`.

## Phase 2: Fix Interactive Selection

3. Add a TUI helper for startup default selection.
   - It should preserve intentional safe defaults.
   - It should exclude the current self-clean Rust target.
4. Update `App::with_plan` to use the helper instead of blindly collecting all
   `selected_by_default` targets.
   - Verify with a TUI test that a project `target/` containing the current exe
     is not selected on startup.
   - Done for startup and scan-finished state updates in `src/tui.rs`.

## Phase 3: Make Cross-Scope Selection Visible

5. Improve selected-target review lines or confirmation command previews so
   cross-tab selected targets include enough scope/path context to identify
   hidden selections.
   - Keep the UI compact for terminal widths used by existing render tests.
6. Add or update `ratatui::backend::TestBackend` render tests for the
   confirmation/dry-run surface.
   - Done in `tui::tests::confirmation_and_dry_run_show_selected_targets_across_scopes`.

## Phase 4: Validation

7. Run focused tests:

```powershell
cargo test --all-targets executor
cargo test --all-targets tui
```

Result: passed.

8. Run the canonical gate:

```powershell
just ci
```

Result: passed.

9. Optional runtime check:

```powershell
just dev
```

Manual checklist:

- Header selected count does not include the current checkout `target/` by
  surprise.
- Selecting pnpm store and confirming does not attempt
  `cargo clean --manifest-path` for the current checkout.
- If the self-clean target is manually selected, progress reports it as skipped
  rather than failed.

## Rollback

- If TUI selection filtering causes regressions, keep only the executor
  self-clean guard as the safety fix and revisit default-selection policy in a
  separate task.
- If confirmation rendering becomes crowded, revert to dry-run/confirmation text
  changes and rely on the executor guard plus default-selection fix.
