# Design: Host Fonts and V2

The parent design sections 3 and 4 are the normative catalogue, DTO, validation, migration, and rollback contracts. Implement those contracts without a second preference store or persistence path chosen by the renderer.

Ownership:

- desktop/src-tauri/src/fonts.rs (new): DirectWrite enumeration, native-owned worker, owned-string result, and read-only command.
- desktop/src-tauri/src/lib.rs: command registration and main-window-only allowlist.
- crates/devsweep-core/src/desktop_preferences/: typed V2, strict V1 reader, mapping, locked first migration, field updates, defaults, and reset validation.
- desktop/src-tauri/src/desktop_preferences.rs: same committed snapshot ordering with V2 payloads.
- desktop/src/api/: decoder, fixtures, bridge, and generated contract.
- desktop/src/preferences/: catalogue lifecycle, searchable picker, missing-font state, one CSS family serializer, and shared appearance mapping.

Keep the native font API outside core; core tests use strings and temporary files without querying host fonts. Use deterministic injected catalogues for command/picker tests and separate native evidence for actual OS discovery. Use the existing async worker pattern; keep COM objects inside the worker that created them. Request local fonts only through IDWriteFactory3 and refresh on explicit request.

V2 introduces the tagged font object and keeps all existing performance field values. The palette child extends the theme enum in the same unreleased parent series. Schema contracts, generated fixtures, decoders, and wire-parity tests must agree at every child checkpoint. Preserve the positive safe sequence and no-success-event-on-failure rule.

Catalogue refresh does not change a preference merely because a family disappeared. Root CSS applies the escaped family plus existing fallbacks, so HUD can render without receiving the font list. Catalogue errors affect only the font picker. Default/system font remains selectable. New user choices come from returned family options; arbitrary typed search is never treated as a CSS font stack.

Before final implementation, verify the approved windows crate version and required feature set against Rust 1.88 and the current Cargo.lock. Add the dependency only to the Windows target of the desktop adapter. Do not add font-kit, fontdb, a registry scan, or a cross-platform abstraction without a demonstrated scope need.

Update the backend desktop-preferences and database guidelines, desktop type/state guidance, and only relevant source-map entries for a new native module. Downgrade behavior is explicitly the parent's preserved V1 file boundary; no silent reverse migration.
