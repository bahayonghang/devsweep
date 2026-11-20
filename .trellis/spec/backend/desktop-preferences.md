# Desktop Preferences Contract

## 1. Scope / Trigger

Apply this contract to the core desktop preference store
(`devsweep_core::desktop_preferences`) and its GPUI consumer
(`desktop-gpui/src/preferences.rs`, Settings and the title-bar theme toggle).
The core owns validated bytes. Settings never grant cleanup authority. The
shared `presentation-v1.json` language contract remains unchanged.

## 2. Signatures

The core owner is `desktop_preferences`. Its public types are
`DesktopPreferencesV2`, `DesktopFont`, and `DesktopPreferencesPatch`. GPUI
reads the store at startup and applies one typed patch per changed field or
group on an explicit Save.

```json
{
  "schema_version": 2,
  "theme": "dark",
  "font": {"kind": "system"},
  "text_scale_percent": 100,
  "motion": "system",
  "planet_fps": 30,
  "status_interval_seconds": 2,
  "status_process_limit": 15,
  "hud_interval_seconds": 2
}
```

## 3. Contracts

The fixed disk path is
`%LOCALAPPDATA%/DevSweep/settings/desktop-preferences-v2.json`.
The disk document accepts the fields above and optional `close_behavior`. A missing value means `exit`. Saving the default omits the optional field for compatibility with earlier V2 files.

| Field | Closed values | Default |
| --- | --- | --- |
| theme | dark, light, system, catppuccin_latte, catppuccin_mocha, codex, claude | dark |
| font | {kind: system}, {kind: installed, family: string} | {kind: system} |
| text_scale_percent | 100, 110, 125 | 100 |
| motion | system, reduced | system |
| planet_fps | 15, 30 | 30 |
| status_interval_seconds | 1, 2, 5, 10, 30, 60 | 2 |
| status_process_limit | 5, 15, 30, 50, 100 | 15 |
| hud_interval_seconds | 2, 5, 10 | 2 |
| close_behavior | exit, stay_in_tray | exit |

`planet_fps` and `hud_interval_seconds` have no consumer since the Tauri
desktop app was removed (2026-11-14). They stay in the closed V2 schema so
existing files keep validating; do not drop them without a schema version bump.

Installed families are trimmed, nonempty, at most 256 Unicode scalar values,
and contain no control characters. Quotes, backslashes, and non-Latin names
remain valid. Both tagged shapes reject unknown fields, including system.

Read V2 first. A present invalid, future, or unreadable V2 fails closed without
V1 fallback. Only absent V2 permits strict read-only conversion of the sibling
V1 file. V1 system maps to system; segoe_ui maps to installed Segoe UI;
microsoft_yahei_ui maps to installed Microsoft YaHei UI. Every unrelated field
is preserved. V1 has its own closed theme decoder so future V2 palettes cannot
enter V1. Reading creates no files. When both versions are absent, use V2 defaults.

The first explicit update/reset rereads both versions under the existing
desktop directory transaction lock, then atomically creates V2. Concurrent
first patches retain both changes. Keep V1 and language bytes unchanged, even
after write failure. An older build reads the last V1 state. New builds use V2
once present and do not resynchronize later V1 edits. No reverse migration runs.

Unknown fields/tags, unsupported versions,
invalid UTF-8/JSON, and invalid values fail closed. Missing LOCALAPPDATA reports
unavailable. Save rereads and validates existing bytes under a cross-process
transaction lock, applies one typed field/group patch, and uses a flushed
same-directory temporary file plus atomic replacement. Keep unrelated concurrent
fields. Do not overwrite corrupt or future-version files, including during reset.

Appearance reset affects theme, font family, and text scale. Performance reset
affects motion, frame cap, Status interval/rows, and the HUD interval field. Neither reset
touches the language file, Protection, audit records, or domain results.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Missing preference file | Valid defaults; a later save can create the document |
| Missing environment, unreadable/invalid/newer file | Structured unavailable error; preserve bytes; UI exposes validated session defaults and disables persistence until reload succeeds |
| Invalid patch tag/value | Reject before write; preserve committed view |
| Write/replace failure | Preserve old bytes and committed view; retry remains available |
| Concurrent unrelated patches | Both changes survive the locked read/modify/write |

## 5. Good / Base / Bad Cases

- Good: change the theme in Settings, Save, restart; the committed theme loads.
- Base: two processes patch distinct fields; locked patches retain both fields.
- Bad: saving the entire stale draft object and losing another process's update.

## 6. Tests Required

In native GPUI, the title bar and Settings preview the same theme draft through
`Session::set_theme`. Only an explicit Save commits pending edits. When testing
the shortcut, change another field first, toggle the theme, and reopen from the
same temporary preferences directory. The previous values must load until Save.
Resolve the toggle direction from the underlying theme so both directions still
work while the high-contrast palette is active.

Native GPUI font selection uses the same draft/explicit-Save contract through
`Session::set_font(DesktopFont)` and `DesktopPatch::Font`. Reading the catalogue
or selecting a family must not write preferences. `FontCatalogue` caches
`TextSystem::all_font_names()` on first Settings entry, coalesces refresh and
keeps the full searchable membership while bounding visible choices. The GPUI
API returns names, not a fallible native result: empty and no-match states must
not invent a platform error. Enumeration belongs on the background executor.
Root typography is inherited by normal content and `TextInput` shaping; native
checks must inspect actual text, placeholders and IME, not merely stored JSON.
Missing families keep their saved identity and use system fallback.

Before any multi-field GPUI save, snapshot which fields differ from the original
committed state. Each locked patch rereads the file and can return concurrent
changes to untouched fields. Do not compare later draft fields against that
refreshed snapshot and accidentally turn another process's edit into local
write intent. After each successful patch, untouched draft fields adopt that
committed snapshot while remaining user edits stay dirty. This rebase must
happen even when a later patch fails, so retry cannot overwrite concurrent
untouched fields. Tests must cover partial-save retry and a font-only edit concurrent with theme and
Status edits, no-op Save, restart restoration, invalid/future files and failed
persistence retaining the draft.

- Core: exact schema/default bytes, validation, missing/unreadable/newer files,
  write rollback, group reset boundaries, concurrent updates, and preservation
  of existing language bytes.
- Native/visual: actual window interactions remain a separate evidence level
  from unit tests; record both locales, themes and scale boundaries.

## 7. Wrong vs Correct

- Wrong: write the whole in-memory preferences object on Save, so a field
  another process changed since load is overwritten.
- Correct: compute the fields that differ from the committed snapshot and apply
  one typed `DesktopPreferencesPatch` per field under the store lock.
