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
pub fn render_placeholder(frame: &mut Frame<'_>) {
    let widget =
        Paragraph::new("Foundation build\n\nScanning and cleanup actions are not implemented yet.")
            .block(Block::bordered().title("devsweep"))
            .alignment(Alignment::Center);

    frame.render_widget(widget, frame.area());
}
```

Future components should take `&AppState` or a smaller view-state reference
instead of owning application data.

---

## Props Conventions

Rust TUI code does not use web-style props. Use function parameters:

- `&mut Frame<'_>` for rendering target.
- `Rect` for constrained child areas once layouts are split.
- `&ViewState` or slices of domain data for read-only display input.
- Return `()` from pure render functions.

Avoid passing raw JSON or untyped maps into components. Decode into model/state
types before rendering.

---

## Styling Patterns

- Use ratatui widgets and style APIs directly.
- Prefer simple, readable layout first. The current placeholder uses
  `Paragraph`, `Block::bordered()`, and `Alignment::Center`.
- Keep styling local to view code until repeated style decisions justify a small
  helper.

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
