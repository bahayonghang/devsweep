# TUI Internal Architecture Implementation Plan

## Prerequisite

- Core and backend children are completed, committed, and archived.
- Read the then-current frontend specs and backend interfaces before editing.
- Start this child, not the parent.

## Ordered Checklist

- [ ] Record baseline TUI app, runtime, terminal, render, and display-hygiene
      test results.
- [ ] Extract neutral `tui::display` helpers currently imported by both app and
      render; remove the app-to-render dependency and rerun affected tests.
- [ ] Form `tui/app/`; move event protocol, input/overlay transitions,
      selection projections, worker transitions, and job/confirmation/log
      behavior in small compiling steps.
- [ ] Keep root `App::update` and one App state owner; remove visibility added
      only to work around the old flat file.
- [ ] Form `tui/runtime/`; separate service adapters and worker translation
      from event/effect dispatch without changing thread ownership.
- [ ] Form `tui/render/`; extract theme, cohesive views, overlays, and
      render-only formatting while keeping one `render_app` interface.
- [ ] Relocate tests to the owning modules and retain cross-module state/runtime
      tests where they exercise the public internal seam.
- [ ] Update affected `.trellis/spec/frontend/` documents to match the final
      tree, dependency direction, and render split.
- [ ] Search for forbidden app-to-render and render-to-backend dependencies.
- [ ] Run focused TUI tests and `just ci`; inspect the full diff for behavior or
      copy drift.
- [ ] Commit only this child, archive it, and record the session before child 4.

## Targeted Validation

```powershell
cargo test --locked tui::app
cargo test --locked tui::runtime
cargo test --locked tui::render
cargo test --locked tui::terminal
just ci
```

Structural checks:

```powershell
rg -n "use .*render" src/tui/app
rg -n "(scanner|provider|Executor|std::process|std::fs)" src/tui/render
rg -n "serde_json::Value" src/tui
```

Expected matches must be tests, documentation, or deliberately typed display
data; production reducer/render matches require review.

## Review Checklist

- One `App` state and one reducer routing interface remain.
- Confirmation owns an immutable validated snapshot and digest.
- Selection overrides survive staged scans; inventory cannot become cleanup
  selection.
- Runtime alone owns worker threads, channels, cancellation, and single-flight.
- Render is pure and display normalization never mutates domain paths.
- Full/degraded/narrow views retain readable action/risk labels and controls.
- Frontend specs describe the implemented directories rather than the retired
  `app.rs`/`render.rs` layout.

