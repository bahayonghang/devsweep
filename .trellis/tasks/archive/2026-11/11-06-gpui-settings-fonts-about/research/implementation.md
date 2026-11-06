# Settings implementation evidence

## Delivered

- `fonts.rs` caches the pinned GPUI text-system family catalogue. Enumeration is
  lazy on Settings entry and runs on GPUI's background executor; an in-flight
  request coalesces refresh, so there is no older overlapping result to apply.
- The picker retains all searchable valid family names, normalizes case-only
  duplicates, and mounts at most 24 installed choices per page plus System.
  A bounded scrolling region and pagination keep large catalogues manageable.
- Settings exposes selected family, multilingual preview, search, explicit
  Refresh, missing-family fallback and selected-state UIA semantics. Opening,
  choosing, paging and saving expose explicit UIA actions. Font-filter Enter
  cannot activate a business row or save settings. Its label follows locale.
- `Session` now loads, validates, previews, tracks and saves DesktopFont through
  existing DesktopPatch::Font and V2 bytes. Root typography inherits the chosen
  family into ordinary controls and TextInput shaping. Missing known families
  use the GPUI system-font default while keeping their stored name unchanged.
- Saving snapshots dirty intent before applying patches. A font-only save can
  no longer interpret a concurrent theme change as another locally dirty field
  and overwrite it; untouched fields adopt the reread committed values.
- About shows actual `CARGO_PKG_VERSION` as GPUI build, the package MIT license,
  repository metadata, project/issues links and an always-available copy action.
  No version bump, new dependency, schema change or Tauri import.

## Checks

- `just gpui-check`: passed after all final edits (format + clippy all targets).
- `just gpui-test`: all 170 tests passed. Includes new full-membership/24-row
  catalogue, coalescing/cache, non-Latin/duplicate/invalid names, missing family,
  explicit preview/save/restart, concurrent theme/status preservation, invalid
  family rejection and corrupt/future-byte preservation tests. The final minor
  selection styling and Save UIA forwarding refinement was checked by final
  clippy; main owns the final integrated test rerun.
- Existing V1 no-write-on-load test now checks migrated Segoe UI font as well.
- Release build and native Windows rendering/keyboard/UIA checks are owned by
  the coordinating agent. No new native result is claimed here.

## API-driven evidence limits

`TextSystem::all_font_names()` returns a Vec, not Result. The UI distinguishes
loading, empty catalogue, no filtered matches and missing saved family; it cannot
invent a platform error reason that GPUI does not report. Refresh re-reads GPUI's
catalogue and does not promise live OS font-install discovery beyond the backend.
`App::open_url()` likewise returns no success/failure value. About does not
announce success; Copy project link is always available as a fallback.

Font rendering before the Settings catalogue loads is delegated to GPUI's normal
font resolution/fallback. The explicit missing-family notice appears after
enumeration. Actual Latin/CJK/emoji/IME rendering requires native evidence.
