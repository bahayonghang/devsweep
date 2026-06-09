# Hook Guidelines

> Stateful UI logic conventions for this project.

---

## Overview

This Rust TUI project does not use React hooks. The equivalent boundary is the
event/update layer in `src/tui.rs`: keyboard events and worker messages enter
`App::update`, which mutates state and returns side-effect requests.

---

## Event Logic Patterns

Interactive code follows a simple state/update/render split:

- input events are converted into actions
- update code mutates app state and returns worker/effect commands
- render code reads state only

The terminal event loop is responsible for interpreting effects such as start
scan, start clean, cancel job, and quit.

---

## Data Fetching

There is no server state or web data fetching. Scanner calls are local
filesystem operations and must run outside render code through the event loop /
worker path so the UI can stay responsive.

---

## Naming Conventions

- Do not use `use_*` naming; this is not a React codebase.
- Prefer `handle_*` for input handlers and `update_*` for state transitions
  when those functions are added.
- Worker event enums should use explicit names such as `WorkerEvent` and
  `ScanProgress`.
- Side-effect requests should use an explicit enum such as `Effect`, not
  stringly typed action names.

---

## Common Mistakes

- Do not introduce a generic hook framework for a single TUI interaction.
- Do not make render functions perform side effects because they are convenient
  places to access UI state.
- Do not scatter event-to-state transitions across individual widgets once the
  TUI has a central app state.
- Do not call scanner/provider/executor APIs directly from key handlers; return
  an effect and let the event loop run it.
