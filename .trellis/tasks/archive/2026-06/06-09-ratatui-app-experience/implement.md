# Ratatui app experience implementation plan

## Checklist

1. Load relevant frontend and backend specs.
   - Verify: required spec docs have been read before code edits.
2. Replace the placeholder TUI with TEA-style app state, events, update effects,
   and render functions.
   - Verify: update tests cover scan, selection, details, confirmation, clean,
     filter, help, cancellation, worker events, and quit.
3. Add a real terminal event loop for the TUI command.
   - Verify: `cargo check --all-targets` compiles the crossterm event loop.
4. Render dashboard/global/projects/rules/jobs tabs, details panel, help, dry-run
   preview, and confirmation overlay.
   - Verify: `TestBackend` smoke test renders a representative app state.
5. Keep side effects outside render code.
   - Verify: tests render a state containing command-backed targets without
     invoking scanner, executor, or filesystem mutation.
6. Run quality gates.
   - Verify: `cargo fmt --all -- --check`, `cargo test --all-targets`, and
     `just ci` pass.

## Expected Files

- `Cargo.toml` / `Cargo.lock`: add a direct `crossterm` dependency only if the
  event loop needs it.
- `src/tui.rs`: TUI model, update, rendering, event loop, and tests.
- Trellis task files: update check/finish artifacts only through the workflow.

## Rollback Points

- If terminal setup becomes too broad, keep `run()` as a single draw and retain
  the tested model/update/view path for this child.
- If executor integration risks accidental cleanup, keep the clean effect queued
  but do not call executor until confirmation and selected-plan conversion are
  covered by tests.
- If `src/tui.rs` grows past reviewable size, split only the render helpers into
  a `src/tui/` module set and preserve public behavior.

## Validation Commands

```powershell
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
just ci
```
