# Quality Guidelines

> Code quality standards for frontend development.

---

## Overview

Frontend quality for this project means terminal UI quality: render functions
must be pure, responsive, testable with ratatui `TestBackend`, and separated
from scanner/executor side effects.

---

## Forbidden Patterns

- Do not scan directories, execute commands, move files, or write audit logs
  from TUI render functions.
- Do not block redraws with large directory walks or size estimation.
- Do not print diagnostics to `stdout` from paths that can also emit JSON.
- Do not duplicate domain model fields in UI-only structs without a clear
  projection reason.

---

## Required Patterns

- Keep render functions deterministic for a given state.
- Use ratatui `TestBackend` for render smoke tests.
- Display risk and action semantics from typed model fields, not from path-name
  guesses.
- Keep cleanup confirmation explicit once execution support is added.

Current test example:

```rust
let backend = TestBackend::new(80, 8);
let mut terminal = Terminal::new(backend).expect("test terminal");

terminal
    .draw(render_placeholder)
    .expect("placeholder renders");
```

---

## Testing Requirements

- Every new view should have at least a smoke render test with `TestBackend`.
- State update logic should have unit tests independent of terminal rendering.
- Worker/event code should test cancellation and failure events before cleanup
  execution is connected.

---

## Code Review Checklist

- Is render code free of filesystem, process, and cleanup side effects?
- Does the view consume typed model/state instead of raw JSON?
- Does the UI remain usable without relying on color alone?
- Are new render paths covered by `TestBackend` or state-level tests?
