# Directory Structure

> How frontend code is organized in this project.

---

## Overview

The frontend for `devsweep` is a Rust terminal UI built with `ratatui`, not a
web frontend. TUI code lives in the `src/tui/` module directory, split along
its natural seams: terminal lifecycle, threaded runtime, app state/reducer,
and rendering. Its crate-private composition seam is `tui::run()`; the only
external Rust interface is `devsweep::run()`.

---

## Directory Layout

Current layout:

```text
src/tui/
├── mod.rs             # pub(crate) fn run() composition and default services
├── terminal.rs        # raw-mode/alt-screen lifecycle
├── display.rs         # pure shared path/text/action/command presentation
├── app/
│   ├── mod.rs         # App owner, constructor, root update routing
│   ├── events.rs      # JobId, UiEvent, Effect, WorkerEvent protocol
│   ├── input.rs       # normal/filter/overlay/confirm/quit input
│   ├── selection.rs   # rows, filters, groups, cursor, selection projections
│   ├── worker.rs      # scan/inventory/clean worker-result transitions
│   ├── jobs.rs        # jobs, logs, progress, frozen confirmation
│   └── tests.rs       # reducer and state-machine coverage
├── runtime/
│   ├── mod.rs         # event loop, effect dispatch, channels, cancellation
│   ├── services.rs    # injected service traits and production adapters
│   ├── workers.rs     # worker execution and WorkerEvent translation
│   └── tests.rs       # fake-service and single-flight coverage
├── render/
│   ├── mod.rs         # render_app, root layout, header/tabs/footer routing
│   ├── theme.rs       # shared semantic colors and styles
│   ├── format.rs      # render-only typed value formatting
│   ├── targets.rs     # categories, targets, details, rules
│   ├── inventory.rs   # read-only capacity observations and details
│   ├── jobs.rs        # jobs, logs, typed scan diagnostics
│   ├── overlays.rs    # confirmation, progress, help/error/quit modals
│   └── tests.rs       # TestBackend behavior coverage
└── test_support.rs    # cfg(test)-only keys, plans, and render helpers
```

---

## Module Organization

- `mod.rs::run()` composes terminal setup with the runtime event loop and
  constructs the default services; it holds no other logic.
- `app/mod.rs::App` is the only mutable UI state owner. Child modules contain
  cohesive implementation blocks; `App::update(UiEvent) -> Vec<Effect>` remains
  the single reducer routing interface.
- `runtime/mod.rs` is the only owner of threads, channels, cancellation tokens,
  clean-worker single-flight, and effect dispatch. Service construction and
  backend validation live in `services.rs`; worker translation lives in
  `workers.rs`.
- `render/mod.rs::render_app` is the only root render interface. View modules
  consume immutable typed state; `theme.rs` and `format.rs` contain shared
  rendering concerns without becoming configurable frameworks.
- `display.rs` is neutral, pure presentation shared by app and render. App
  modules must not import render modules.
- Scanner, cleanup execution, and size calculation do not belong in TUI
  modules; scanning goes through `scan::Sweeper` via
  `runtime/services.rs::SweepScanService`.
- Shared cleanup data comes from `src/model/`; TUI code should not define a
  second target schema.
- Visibility discipline: `run()` is `pub(crate)` for application dispatch;
  child-module collaboration uses `pub(super)` or private items. The private
  `tui` module exposes no external Rust API.

---

## Naming Conventions

- Render functions use `render_<view>` names.
- TUI modules use `snake_case`.
- View state types should be named after the UI concept they own, for example
  `App`, `ConfirmState`, and `JobRecord`.

---

## Examples

- `src/tui/render/mod.rs::render_app` shows the pure root render function.
- `src/tui/render/tests.rs::representative_state_renders_with_targets_details_and_jobs`
  shows the `TestBackend` render pattern.
- `src/tui/runtime/tests.rs::scan_worker_emits_started_progress_finished`
  shows the fake-service worker translation pattern.
