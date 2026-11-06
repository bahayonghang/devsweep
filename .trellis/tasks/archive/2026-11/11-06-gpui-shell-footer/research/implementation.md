# Footer implementation evidence

## Change boundary

Implemented the approved footer/focus child in `desktop-gpui/src/view.rs`,
`input.rs` and `copy.rs`. No domain state machine, persistence contract or
cleanup authority changed.

- Removed the entire status bar, Note entity, Note constructor/accessibility
  identity and Note-only Tab/Shift-Tab bridge.
- Startup and lost-focus fallback now target the shell. Fallback checks for
  absent focus, so a focused input/control is not redirected.
- Tab and Shift-Tab use the existing visible-control order; Alt+Left/Right
  bind globally as well as inside the shared input context. Caret arrows and
  search IME/buffer implementation remain intact.
- Working status appears only while busy in existing top-bar chrome. Existing
  page error/status banners remain. Keyboard guidance is available from the
  brand-menu tooltip instead of a permanent bottom strip.
- Updated affected accessibility, binding and copy assertions. Deleted the
  obsolete Note-placeholder test; retained shared buffer/input scale coverage.

## Checks

- `cargo fmt --manifest-path desktop-gpui/Cargo.toml --all`: passed.
- `just gpui-check`: passed (format and clippy, all targets).
- `just gpui-test`: passed, 166 tests. The first run found an English hint
  wording mismatch (`search field`); the copy was corrected and the full suite
  passed on rerun.
- `git diff --check`: passed before ownership handoff.
- `just gpui-build`: passed (optimized release build).

The managed mbx cache was inaccessible in the ordinary sandbox. Standard
check/test/build commands used approved elevated execution without changing
repository configuration.

## Evidence limits and handoff

Source-level accessibility/binding assertions and buffer tests do not establish
native keyboard, UIA or IME success. Main-session integrated Windows QA must
verify startup Tab, route-unmount fallback, menu return, blank click, search
editing/IME, no Note UIA node and busy header layout at narrow sizes. Product
source ownership was handed to the Settings child after the test rerun.
