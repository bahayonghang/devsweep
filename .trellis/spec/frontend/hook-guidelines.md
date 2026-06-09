# Hook Guidelines

> Stateful UI logic conventions for this project.

---

## Overview

This Rust TUI project does not use React hooks. The equivalent boundary is the
future event/update layer that will own keyboard events, worker messages, scan
requests, and cancellation.

Until that layer exists, do not invent hook-like abstractions. Keep the current
TUI placeholder simple.

---

## Event Logic Patterns

Future interactive code should follow a simple state/update/render split:

- input events are converted into actions
- update code mutates app state or sends worker commands
- render code reads state only

The root design calls for TEA-style organization. Apply it when the TUI becomes
interactive, not before.

---

## Data Fetching

There is no server state or web data fetching. Scanner calls are local
filesystem operations and should run outside render code, eventually through a
worker so the UI can stay responsive.

---

## Naming Conventions

- Do not use `use_*` naming; this is not a React codebase.
- Prefer `handle_*` for input handlers and `update_*` for state transitions
  when those functions are added.
- Worker event enums should use explicit names such as `WorkerEvent` and
  `ScanProgress`.

---

## Common Mistakes

- Do not introduce a generic hook framework for a single TUI interaction.
- Do not make render functions perform side effects because they are convenient
  places to access UI state.
- Do not scatter event-to-state transitions across individual widgets once the
  TUI has a central app state.
