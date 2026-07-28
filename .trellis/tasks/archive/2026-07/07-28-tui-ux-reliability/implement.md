# Implementation Plan: TUI Lifecycle, Viewport, and Stale Targets

## Preconditions

- Approve `unicode-width` for accurate terminal cell measurement.
- Merge or rebase after `07-28-tui-race-hardening` finalizes job state.
- Reuse ProcessRunner output sanitization for command diagnostics.

## Ordered work

1. Add injectable terminal-control tests for partial enter, restore failures,
   normal Drop, and panic-hook restoration.
2. Implement `TerminalSession`, idempotent best-effort restoration, and panic
   hook chaining. Replace direct terminal enter/restore calls.
3. Add app-owned viewport state and tests for visibility, resize, filtering,
   PgUp/PgDn, and first/last boundaries.
4. Render a pure visible-row window and add long-list TestBackend snapshots.
5. Add the target lifecycle projection. Drive tombstones from clean progress.
   Exclude cleaned rows from selections, totals, manifests, and effects.
6. Add display sanitation, argv escaping, and cell-width truncation tests for
   whitespace, controls, combining text, and long CJK paths.
7. Run focused terminal/app/render tests, dynamic terminal checks, and `just ci`.

## Verification matrix

| Scenario | Required evidence |
| --- | --- |
| partial setup failure | completed setup steps are all restored |
| restore failure | later cleanup attempts run despite earlier error |
| panic | restore runs idempotently and original hook still receives panic |
| long list | selected row stays inside viewport during movement and resize |
| clean success | tombstone is visible but id/bytes/manifest/effect exclude it |
| other outcome | failed/skipped/unknown/canceled target is not shown as cleaned |
| display text | controls are neutralized, argv boundaries are explicit, CJK fits cells |

Run `cargo test --all-targets` and `just ci`. Record dynamic behavior from one
supported Windows terminal and one Unix terminal. That check complements, but
does not replace, deterministic unit and TestBackend coverage.

## Review points

- Preserve previous panic-hook behavior after terminal restoration.
- Keep render functions pure and keep lifecycle state in `App`.
- Verify every selection/confirmation/effect path excludes tombstones.
- Do not activate until `unicode-width` approval and race-state hand-off exist.
