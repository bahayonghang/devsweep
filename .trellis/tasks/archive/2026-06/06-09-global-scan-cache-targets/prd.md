# global scan cache targets

## Goal

Make the TUI scan on startup so global and project cleanup targets are
available without pressing `s`, and keep those results cached in the current
session state for the Dashboard and Global views.

## Requirements

- The TUI must trigger an initial scan automatically on startup.
- Scan results must be stored in app state and survive redraws and tab switches
  for the current session.
- The Global view must show discovered global cleanup targets after the scan
  completes.
- Manual scan via `s` must still work and replace the current in-memory
  snapshot.
- The change must stay non-mutating; scanning only discovers targets.
- This task does not add persistent on-disk caching across restarts.
- This task does not change executor behavior, cleanup actions, or target
  definitions.

## Acceptance Criteria

- [ ] Launching `cargo run -- tui` triggers a background scan without any key
      input.
- [ ] After the scan finishes, the Dashboard and Global views reflect the
      discovered targets when providers are available.
- [ ] Switching between views does not clear the discovered targets.
- [ ] Pressing `s` still starts a fresh scan and refreshes the visible
      snapshot.
- [ ] Existing cleanup plan and executor behavior remain unchanged.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
