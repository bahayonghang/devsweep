# Fix TUI cleanup confirmation feedback - Implementation Plan

## Checklist

1. Load applicable frontend/backend specs before editing.
   - Verify: task artifacts and guideline files are read.
2. Add inline confirmation feedback state.
   - Verify: invalid Enter updates `ConfirmState` and renders feedback.
3. Clarify confirmation modal copy.
   - Verify: render text explains typing the exact phrase before Enter.
4. Set optimistic cleanup progress on valid confirmation.
   - Verify: accepted confirmation renders `0 / total` cleanup progress before
     worker events.
5. Preserve completed cleanup progress after fast completion.
   - Verify: progress followed by finish still renders a completed result.
6. Add focused regression tests.
   - Verify: new tests fail on the old behavior and pass with the fix.
7. Run focused tests.
   - Verify: relevant TUI tests pass.
8. Run the canonical gate.
   - Verify: `just ci` passes.

## Validation Commands

```powershell
cargo test tui --all-targets
cargo test executor --all-targets
just ci
```

Use narrower filters while iterating, but finish with `just ci`.

## Risk Points

- Do not bypass the required phrase for irreversible command-backed cleanup.
- Do not clear final progress before the user can see it.
- Do not change executor or model contracts unless implementation proves it is
  unavoidable.
- Keep render helpers pure; rendering must not mutate `App`.

## Implementation Status

Completed and validated with focused TUI/executor tests plus `just ci`.
