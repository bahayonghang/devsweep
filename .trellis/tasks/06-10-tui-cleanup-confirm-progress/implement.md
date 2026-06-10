# Improve TUI cleanup confirmation and progress - Implementation Plan

## Checklist

1. Inspect TUI and executor specs before editing.
   - Verify: relevant Trellis frontend/backend guideline files are read.
2. Add command preview data to confirmation state.
   - Verify: unit test confirms command-backed selections expose argv preview.
3. Render command preview and modal key hints.
   - Verify: `TestBackend` render text includes command preview and confirm /
     cancel key hints.
4. Add executor progress observation without changing CLI cleanup behavior.
   - Verify: executor tests still pass and existing `run_plan` callers compile.
5. Forward cleanup progress from the TUI worker.
   - Verify: app state records numeric progress during cleanup worker events.
6. Render cleanup progress bar in the approved surface.
   - Verify: render test asserts visible `completed / total` cleanup progress.
7. Run focused tests while iterating.
   - Verify: `cargo test tui --all-targets` or the narrowest equivalent passes.
8. Run the canonical gate.
   - Verify: `just ci` passes.

## Validation Commands

```powershell
cargo fmt --all -- --check
cargo test --all-targets
just ci
```

Use narrower `cargo test` filters while iterating, but finish with `just ci`.

## Risk Points

- Do not treat display previews as shell commands for execution.
- Do not change cleanup-plan serialization unless implementation proves it is
  unavoidable.
- Do not alter permanent-delete behavior.
- Keep render helpers pure; rendering must not mutate `App`.

## Implementation Status

Implementation started after user approval. The checklist has been completed
and validated with focused tests plus `just ci`.
