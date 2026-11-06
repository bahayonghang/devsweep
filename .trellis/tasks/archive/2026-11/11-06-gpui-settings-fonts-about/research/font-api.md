# Pinned GPUI font API evidence

Inspected local dependency checkout at
`C:/Users/lyh/.cargo/git/checkouts/zed-a70e2ad075855582/322cd50`.

- `crates/gpui/src/text_system.rs:284`: public
  `TextSystem::all_font_names(&self) -> Vec<String>` returns sorted unique names
  from the platform text system, including explicitly registered custom fonts.
- `crates/gpui/src/app.rs:2418`: `App::text_system()` exposes
  `&Arc<TextSystem>`, which can be cloned before background work.
- `crates/gpui/src/platform.rs:1498`: `PlatformTextSystem: Send + Sync`.
- Vendored `desktop-gpui/vendor/gpui_windows/src/direct_write.rs:1285`
  uses its existing system/custom collections. No new DirectWrite dependency or
  Tauri import is needed for initial enumeration.
- The method returns a vector, not a fallible Result. Preserve empty-catalogue
  semantics and do not invent native error details. Background task failures can
  expose an honest unavailable state if the actual runtime provides such errors.
- `input.rs` shapes input and placeholder with `window.text_style().font()`;
  root inheritance should cover both, subject to native rendering verification.

Use existing GPUI worker execution with owned names and a session cache; do not
enumerate from every render. Explicit Refresh re-enumerates the text-system
catalogue; do not promise hot installation detection beyond backend behavior.
