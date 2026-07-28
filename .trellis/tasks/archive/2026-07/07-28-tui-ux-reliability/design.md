# Design: TUI Lifecycle, Viewport, and Stale Targets

## Terminal lifecycle

Replace the linear enter/restore pair with `TerminalSession`, an RAII owner.
It records raw mode, alternate screen, cursor state, and the ratatui terminal.
Construction rolls back every completed setup step if a later step fails.
`Drop` separately attempts disable-raw, leave-alternate-screen, and show-cursor.
One failed restoration operation must not prevent the remaining attempts.

`tui::run` wraps the existing panic hook, restores best-effort state, and then
delegates to the previous hook. Normal unwinding also reaches `Drop`.
Restoration is idempotent because the hook and `Drop` can both run it.
Unit tests use an injectable terminal-control seam rather than a real terminal.

## Viewport and input

`App` owns a target-list scroll offset in addition to its cursor.
Rendering derives a visible row window from panel height and offset.
Rendering must not mutate scroll state. Navigation clamps cursor and offset
together after filtering, scanning, tombstoning, and resize.
PgUp/PgDn move one viewport minus overlap and clamp at list boundaries.

## Tombstones

On a successful target-level clean result, the app projects that target as
`Cleaned`. It remains visible until the next rescan as a disabled tombstone row.
The row says `cleaned; rescan to refresh` and cannot be selected or executed.
Its id is removed from selected ids, selected-byte totals, confirmation manifests,
and future execution requests. Failed, skipped, unknown, and canceled targets
are never marked cleaned. A completed scan replaces the projection and clears
old tombstones. This uses target-level worker progress, not aggregate reports.

## Display hygiene

One display-text helper removes or escapes terminal controls from paths and
diagnostics before rendering. Process output reuses the runner sanitizer.
Argv preview quotes and escapes each argument independently and is labelled as
an argv preview, not a shell-copyable command.

Use the maintained `unicode-width` crate for terminal-cell truncation.
It is a direct production dependency requiring approval before activation.
The truncator preserves Unicode boundaries, fits an ellipsis only when possible,
and prevents wide CJK cells from overflowing their allocated width.

## Compatibility

Rendering remains free of filesystem, process, and cleanup side effects.
Tombstones are session-only and never serialized. Rollback needs no migration,
but terminal and display-sanitization regression tests remain required.
