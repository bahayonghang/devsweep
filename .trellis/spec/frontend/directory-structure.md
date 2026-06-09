# Directory Structure

> How frontend code is organized in this project.

---

## Overview

The frontend for `devsweep` is a Rust terminal UI built with `ratatui`, not a
web frontend. Current TUI code lives in `src/tui.rs` as a small placeholder.
Keep it there until the TUI grows enough to justify a `src/tui/` directory.

---

## Directory Layout

Current layout:

```text
src/
└── tui.rs  # ratatui entrypoint and render function
```

Expected split point for future work:

```text
src/tui/
├── mod.rs       # public run/render entrypoints
├── layout.rs    # frame layout helpers
├── widgets.rs   # reusable small widgets
├── dashboard.rs # dashboard tab rendering
├── projects.rs  # project-target list rendering
├── global.rs    # global-provider list rendering
├── details.rs   # target details panel
├── confirm.rs   # confirmation modal
└── help.rs      # keyboard help
```

Only create this directory when multiple views exist. Do not split the current
placeholder for its own sake.

---

## Module Organization

- `run()` owns terminal backend setup and the draw loop.
- `render_*` functions own pure drawing from already-computed state.
- Scanner, cleanup execution, and size calculation do not belong in TUI render
  modules.
- Shared cleanup data comes from `src/model.rs`; TUI code should not define a
  second target schema.

Current example from `src/tui.rs`:

```rust
pub fn run() -> Result<()> {
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(render_placeholder)?;
    Ok(())
}
```

---

## Naming Conventions

- Render functions use `render_<view>` names.
- TUI modules use `snake_case`.
- View state types should be named after the UI concept they own, for example
  `DashboardState`, `SelectionState`, or `ConfirmState` when those are added.

---

## Examples

- `src/tui.rs::render_placeholder` shows the current pure render function.
- `src/tui.rs::tests::placeholder_renders_with_test_backend` shows the current
  test pattern for rendering without a real terminal.
