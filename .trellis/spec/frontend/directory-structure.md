# Directory Structure

> How frontend code is organized in this project.

---

## Overview

The frontend for `devsweep` is a Rust terminal UI built with `ratatui`, not a
web frontend. Current TUI code lives in `src/tui.rs` and contains the terminal
entrypoint, app state, update/effect boundary, render helpers, and TUI tests.
Keep the single file while it remains reviewable; split to `src/tui/` only when
moving cohesive render/state sections removes real complexity.

---

## Directory Layout

Current layout:

```text
src/
└── tui.rs  # ratatui entrypoint, app state, update/effects, render helpers
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
- `App` owns active tab, target selection, filters, overlays, jobs, logs, and
  the quit flag.
- `App::update` consumes key/worker events and returns side-effect requests.
- `render_*` functions own pure drawing from already-computed state.
- Scanner, cleanup execution, and size calculation do not belong in TUI render
  modules.
- Shared cleanup data comes from `src/model.rs`; TUI code should not define a
  second target schema.

Current shape from `src/tui.rs`:

```rust
pub fn run() -> Result<()> {
    // Terminal setup and event loop orchestration only.
}

fn render_app(frame: &mut Frame<'_>, app: &App) {
    // Pure view from App state.
}
```

---

## Naming Conventions

- Render functions use `render_<view>` names.
- TUI modules use `snake_case`.
- View state types should be named after the UI concept they own, for example
  `App`, `ConfirmState`, and `JobRecord`.

---

## Examples

- `src/tui.rs::render_app` shows the current pure root render function.
- `src/tui.rs::tests::representative_state_renders_with_targets_details_and_jobs`
  shows the current `TestBackend` render pattern.
