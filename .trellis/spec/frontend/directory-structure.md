# Directory Structure

> How frontend code is organized in this project.

---

## Overview

The frontend for `devsweep` is a Rust terminal UI built with `ratatui`, not a
web frontend. TUI code lives in the `src/tui/` module directory, split along
its natural seams: terminal lifecycle, threaded runtime, app state/reducer,
and rendering. The module's only public symbol is `tui::run()`.

---

## Directory Layout

Current layout:

```text
src/tui/
├── mod.rs          # module wiring, pub fn run(), default service construction
├── terminal.rs     # raw-mode/alt-screen lifecycle
├── runtime.rs      # event loop, mpsc channels, scan/clean workers,
│                   # ScanService/CleanService injection seam + real adapters
├── app.rs          # App state, pure reducer (update/handle_key), TUI domain types
├── render.rs       # render_* functions, styles, formatting helpers
└── test_support.rs # cfg(test)-only shared fixtures (key, render_text, plans)
```

Do not split `render.rs` further while the `render_*` prefix keeps it
navigable; create per-view files only when a view accumulates real complexity.

---

## Module Organization

- `mod.rs::run()` composes terminal setup with the runtime event loop and
  constructs the default services; it holds no other logic.
- `runtime.rs` owns threads and channels. Workers receive their dependencies
  through the `ScanService`/`CleanService` traits; `Sweeper::default()` and
  `Executor::default()` are constructed only inside the real adapters
  (`SweepScanService`, `ExecutorCleanService`). Tests drive workers with fake
  services and assert the `WorkerEvent` translation.
- `App` (app.rs) owns active tab, target selection, filters, overlays, jobs,
  logs, and the quit flag.
- `App::update` consumes key/worker events and returns side-effect requests.
- `render_*` functions (render.rs) own pure drawing from already-computed
  state.
- Scanner, cleanup execution, and size calculation do not belong in TUI
  modules; scanning goes through `sweep::full_scan` via the runtime services.
- Shared cleanup data comes from `src/model.rs`; TUI code should not define a
  second target schema.
- Visibility discipline: submodule items are `pub(super)` or private; nothing
  is `pub` beyond `run()`.

---

## Naming Conventions

- Render functions use `render_<view>` names.
- TUI modules use `snake_case`.
- View state types should be named after the UI concept they own, for example
  `App`, `ConfirmState`, and `JobRecord`.

---

## Examples

- `src/tui/render.rs::render_app` shows the pure root render function.
- `src/tui/render.rs::tests::representative_state_renders_with_targets_details_and_jobs`
  shows the `TestBackend` render pattern.
- `src/tui/runtime.rs::tests::scan_worker_emits_started_progress_finished`
  shows the fake-service worker translation pattern.
