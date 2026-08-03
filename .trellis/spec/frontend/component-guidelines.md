# Component Guidelines

> How terminal UI components are built in this project.

---

## Overview

Components are ratatui render functions or small widget builders. They should be
pure with respect to cleanup behavior: render functions may format state and
draw widgets, but must not scan directories, execute commands, move files, or
perform expensive size calculations.

---

## Component Structure

Current component shape:

```rust
fn render_app(frame: &mut Frame<'_>, app: &App) {
    render_header(frame, header_area, app);
    render_body(frame, body_area, app);
    render_footer(frame, footer_area, app);
    render_overlay(frame, app);
}
```

Components should take `&App` or a smaller view-state reference instead of
owning application data.

`render/mod.rs` owns root layout and routing. Cohesive view functions live in
`targets.rs`, `inventory.rs`, `jobs.rs`, and `overlays.rs`; they must not call
one another to coordinate state transitions. Shared semantic styles live in
`theme.rs`, and render-only typed value formatting lives in `format.rs`.

---

## Props Conventions

Rust TUI code does not use web-style props. Use function parameters:

- `&mut Frame<'_>` for rendering target.
- `Rect` for constrained child areas once layouts are split.
- `&ViewState` or slices of domain data for read-only display input.
- `&App` is acceptable for root or broad layout render helpers; prefer narrower
  references once helpers become reusable.
- Return `()` from pure render functions.

Avoid passing raw JSON or untyped maps into components. Decode into model/state
types before rendering.

---

## Styling Patterns

- Use ratatui widgets and style APIs directly.
- Prefer simple, readable layouts and keep view-specific composition in the
  owning view module.
- Route repeated semantic styles through `render/theme.rs`; do not turn it into
  a configurable design system.
- Keep shared display-only path cleanup in `tui/display.rs`. Windows verbatim
  prefixes such as `\\?\D:\...` and `\\?\UNC\server\share\...` may surface from
  typed model paths or path-like target IDs, so normalize those strings only
  when drawing them. Do not write normalized strings back into `CleanupPlan`,
  `CleanTarget`, executor requests, or audit data.

---

## Accessibility And Usability

- TUI screens must preserve readable labels in narrow terminals.
- Keyboard actions should be visible in help/status areas once interactive TUI
  work begins.
- Do not rely on color alone to communicate risk; also display text such as
  `Low`, `Medium`, `High`, or `Dangerous`.

---

## Common Mistakes

- Do not start a scan or cleanup job from a render function.
- Do not block the draw loop with directory size calculation.
- Do not duplicate cleanup target formatting logic across multiple panels once
  a shared formatter becomes necessary.
- Do not call `Path::display()` directly in multiple TUI panels when the value
  may come from canonical Windows paths; route it through one display helper so
  target lists, details, command previews, footer/status text, and modal copy
  stay consistent.
- Do not call `App::update`, scanner/provider APIs, or `Executor` from
  `render_*` helpers.
- Do not import runtime services or worker functions from render modules.
