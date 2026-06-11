# Optimize TUI visual hierarchy and layout - Implementation Plan

## Checklist

1. Use the approved supported terminal-size target.
   - Verify: implementation keeps full layout usable at `100x28` and graceful
     degraded layout usable at `80x24`.
2. Re-read required frontend specs before editing.
   - Verify: `quality-guidelines.md`, `component-guidelines.md`,
     `directory-structure.md`, `state-management.md`, `hook-guidelines.md`,
     and `type-safety.md` are accounted for.
3. Add focused baseline render tests for representative sizes.
   - Verify: tests fail only where current layout behavior is being improved,
     not because render functions panic.
4. Introduce semantic style/layout helpers.
   - Verify: new or changed render code uses named roles instead of scattering
     raw RGB values.
5. Make body layout width-aware.
   - Verify: wide, medium, and narrow `TestBackend` renders show the intended
     surfaces without panics or unreadable critical labels.
6. Improve target list scanning.
   - Verify: render output includes stable selected/risk/size/target labels or
     columns, and selected state is visible without color-only reliance.
7. Tighten header and footer density.
   - Verify: critical state and mode-specific actions fit at supported sizes;
     secondary actions may be omitted before wrapping/clipping.
8. Polish overlays and secondary views only as needed for consistency.
   - Verify: help, details, dry-run, confirmation, cleanup progress, and
     Jobs/Logs still render at representative sizes.
9. Run focused TUI tests while iterating.
   - Verify: `cargo test tui::tests --all-targets` passes.
10. Run formatting and the canonical gate.
    - Verify: `cargo fmt --all -- --check` and `just ci` pass.

## Validation Commands

```powershell
cargo test tui::tests --all-targets
cargo fmt --all -- --check
just ci
```

Use narrower test filters while iterating, but finish with `just ci`.

## Risk Points

- Do not move scanner, provider, executor, or audit behavior into render code.
- Do not change cleanup semantics while changing presentation.
- Do not let a visual style helper normalize or mutate model data used by
  execution.
- Do not split `src/tui.rs` just for aesthetics. Split only if cohesive
  layout/style sections become easier to review as modules.
- Do not make tests brittle by asserting complete frame dumps unless the full
  frame is the contract.

## Review Gate Before `task.py start`

- The user accepted the supported terminal-size target: full layout at
  `100x28`, graceful degraded layout at `80x24`.
- `prd.md`, `design.md`, and `implement.md` have no blocking open questions.
- Implementation remains scoped to TUI presentation and tests.
