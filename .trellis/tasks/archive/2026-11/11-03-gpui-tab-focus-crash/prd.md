# GPUI mode-tab focus crash

## Problem

The debug desktop starts, but clicking a mode tab panics with
`set_active_descendant called on the focused node` when accessibility is active.

## Requirements

- All five mode tabs remain clickable without a panic in the native debug build.
- Preserve keyboard focus, activation, accessible labels, roles, and selected state.
- Keep the fix limited to the conflicting accessibility focus model and regression coverage.
- Do not change GPUI dependencies, cleanup behavior, or persisted preferences.

## Acceptance

- Reproduce the original panic and verify the same interaction after the fix.
- Check all five tabs, repeat selection, and keyboard focus/activation.
- Add a regression guard that fails on the original implementation.
- Run GPUI formatting, tests, and strict Clippy; record native evidence separately.
- Sweep other active-descendant uses for the same conflict.
