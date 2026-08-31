# Design - Status Presentation

Mode-local state stores the latest tagged snapshot, at most 60 optional chart
points per supported rate metric, process sort, requested interval, and operation
id. Missing/partial samples create chart gaps, not zeroes. Cards render only
supported metrics; a compact capability note explains unavailable hardware.

The adapter accepts only the CLI-owned `StatusSnapshotV1` and the four exact
live event objects. It converts basis points/bytes/timestamps only for display,
never changes the DTO or derives zero from non-available variants. Process rows
surface `enumerated_count`, `returned_count`, `truncated_by_limit`, and
`budget_exhausted` explanations. `status_terminal` closes the matching operation
id; skipped/duplicate/nonmonotonic sequence or an unknown event/availability
variant is a decoder error, not silently ignored. The same fixtures feed TUI,
Tauri generation/runtime decoding, and React.

Live starts from an explicit control and uses the shell coordinator. Interval
changes cancel/join then restart one stream. Leaving or closing clears buffers.
The process table contains only approved fields and accessible text equivalents
for bars/charts. Rollback unregisters Status and discards ephemeral state; there
is no persistence migration.

## File-level change list

- `crates/devsweep-cli/src/tui/modes/status/`: add snapshot cards, bounded chart
  buffers, process sorting, interval controls, and cancel-on-leave behavior.
- `desktop/src-tauri/src/status.rs` and `desktop/src-tauri/src/lib.rs`: expose the
  frozen Status commands/events and unregister them together on rollback.
- `desktop/src/modes/status/`: add the typed Status store, cards, chart/text
  alternatives, process table, unavailable states, and responsive layout.
- `desktop/src/app-shell/registry.ts`: register the mode only after the shell
  registry and Status backend contract are accepted.
- `crates/devsweep-cli/tests/fixtures/status/` and `desktop/src/modes/status/*.test.tsx`:
  add locale-invariant exact wire streams, truncation/budget, churn,
  missing/unknown-field rejection, terminal/broken-pipe, accessibility, and
  lifecycle fixtures.
- `desktop/scripts/generate-types.mjs`: register the frozen Status fixtures and
  regenerate checked-in bindings through the existing package script; the
  deterministic generation test must leave no unexplained output drift.
- `docs/validation/status-native.md`: record the standard-user native matrix and
  raw resource-trace artifact names.

This child consumes the desktop component/state conventions written and accepted
by `08-29-desktop-shell-navigation-brand`; it does not define a competing desktop
specification.
