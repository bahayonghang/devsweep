# Desktop Preferences Contract

## 1. Scope / Trigger

Apply this contract to the core desktop preference store, Tauri preference
commands, main/HUD appearance, and the related Status/Planet/HUD inputs.
The core owns validated bytes. Settings never grant cleanup authority.
The shared `presentation-v1.json` language contract remains unchanged.

## 2. Signatures

The core owner is `desktop_preferences`. Its public types are
`DesktopPreferencesV2`, `DesktopFont`, and `DesktopPreferencesPatch`. The Tauri adapter returns
`DesktopPreferencesSnapshot` from `desktop_preferences_get` and
`desktop_preferences_update`; update receives one `patch` argument.

```json
{
  "sequence": 1,
  "preferences": {
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
}
```

Patch payloads use a closed tagged shape:

```json
{"patch":{"field":"theme","value":"light"}}
{"patch":{"field":"font","value":{"kind":"installed","family":"Segoe UI"}}}
{"patch":{"field":"reset_appearance"}}
{"patch":{"field":"reset_performance"}}
```

The `desktop-preferences-changed` event carries the same snapshot envelope.
`desktop/src/api/bridge.ts` is the native frontend adapter. Components do not
import Tauri. New response/event types and fixtures use the existing generator
and Rust wire-parity test.

## 3. Contracts

The fixed disk path is
`%LOCALAPPDATA%/DevSweep/settings/desktop-preferences-v2.json`.
The disk document is the exact `preferences` object above. The snapshot
sequence belongs to one running app process and is not persisted.

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
affects motion, frame cap, Status interval/rows, and HUD interval. Neither reset
touches the language file, Protection, audit records, or domain results.

One Tauri coordinator serializes reads and updates. Successful responses carry
a monotonically increasing, positive JavaScript-safe integer sequence. A
successful update publishes its committed snapshot; failed updates publish
nothing. The main window can get/update. HUD can get only, in addition to its
existing language read; HUD cannot mutate settings or invoke domain operations.

Each window subscribes before loading, accepts only snapshots at least as new
as its committed sequence, ignores disposed requests, and removes listeners.
Reloading the same bridge preserves the committed sequence, including after a
failed read. Only a different bridge resets the sequence namespace. A delayed
event or read from before reload must not replace a newer committed value.
Use the command response to update the initiating window after commit. Reload
on HUD show so an existing hidden HUD also receives current persisted settings.
An event delivery failure cannot roll back committed disk bytes; the next get
returns authoritative settings. Cross-process live notifications are out of scope.

Apply theme/font/motion in main and HUD after commit. The typed theme catalogue
owns IDs, label keys, schemes, and preview IDs. System resolves only to the
legacy dark/light palettes and listens for OS media-query changes only while
selected. Write the resolved palette ID to data-theme and only light or dark
to color-scheme. Reset appearance retains the dark default. The independent V1
decoder still accepts only dark/light/system; unknown V2 themes fail closed
without overwriting bytes or falling back to V1. Response and patch fixtures
cover all seven values and participate in generation and Rust wire parity.
Reduced motion and forced colors
remain accessibility overrides. Preserve main/HUD base sizes and local font
fallbacks. Do not download fonts or change OS display settings.

The desktop-owned `fonts.rs` exposes main-only `desktop_fonts_list`. The HUD
allowlist remains language/preference reads only. On Windows, the approved
target-specific windows 0.61.3 dependency enables Win32_Graphics_DirectWrite.
IDWriteFactory3 runs in a blocking worker with downloadable fonts disabled and
check-for-updates enabled. Native objects remain on that worker; only owned
family/locale/name strings cross the boundary. Families prefer en-US, otherwise
the first nonempty native name. Deduplicate family keys case-insensitively and
retain localized aliases. Never return paths/bytes, scan font folders, invoke
shells, download fonts, or start Status/HUD sampling.

The closed catalogue response is available with a families array (including
empty) or unavailable with unsupported_platform/enumeration_failed. Renderer
catalogues cache by bridge/session, load only with Settings, coalesce refresh,
ignore stale/disposed completions, and retain prior choices after refresh
failure. System stays selectable. Missing saved names remain committed and
receive a notice. Main/HUD share one CSS string serializer and set only the
font custom property plus existing system fallbacks.

Status snapshot and live requests both consume the selected row limit. Convert
the stored interval from seconds to milliseconds at the command boundary. An
active in-mode interval edit saves before cancel/join/restart. A Settings edit
does not start Status automatically. HUD consumes its interval on its next show;
a hidden HUD never samples. Frame cap changes affect the real Planet scheduler,
with the existing source cap, turn duration, and hidden-document stop preserved.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Missing preference file | Valid defaults; a later save can create the document |
| Missing environment, unreadable/invalid/newer file | Structured unavailable error; preserve bytes; UI exposes validated session defaults and disables persistence until reload succeeds |
| Invalid patch tag/value/extra field | Reject before write or event; preserve committed view |
| Write/replace failure | Preserve old bytes and committed view; retry remains available |
| Concurrent unrelated patches | Both changes survive the locked read/modify/write |
| Delayed startup/read event | Newest committed snapshot wins; disposed result is ignored |
| Reload after sequence N is committed | Retain N; reject older events and read responses during and after reload |
| Save fails during active Status interval edit | Existing run and selected interval remain active |
| HUD hides or exits | Cancel and join its one sampler |
| HUD invokes update/domain command | Window gate rejects the call |

## 5. Good / Base / Bad Cases

- Good: Save light theme, then open HUD; both use the committed light tokens.
- Base: Two windows update distinct fields; locked patches retain both fields.
- Bad: A renderer saves its entire stale object and loses another field update.
- Bad: A Settings slider changes local text while the sampler still uses its old constant.

## 6. Tests Required

- Core: exact schema/default bytes, validation, missing/unreadable/newer files,
  write rollback, group reset boundaries, concurrent updates, and preservation
  of existing language bytes.
- Tauri: ordered get/update snapshots, command/event fixture parity, main/HUD
  invoke boundary, and actual HUD interval input with one sampler.
- Frontend: closed decoding, subscribe-before-load ordering, late/disposed
  responses, failures, group reset, main/HUD theme/font application, OS media
  changes, and actual Status/frame-loop consumer arguments.
  Commit sequence 5, reload the same bridge, and deliver an older event while
  the read is pending. Assert that the committed values remain until the
  sequence 6 read resolves.
- Native/visual: actual window interactions remain a separate evidence level
  from fixtures; record both locales, themes, scale boundaries, and HUD states.

## 7. Wrong vs Correct

```ts
// Wrong: reports an uncommitted choice and leaves another consumer stale.
setTheme(next);
void invoke("save_settings", { settings: staleLocalSnapshot });

// Correct: one typed patch; both windows consume the committed snapshot.
const snapshot = await bridge.desktopPreferencesUpdate({ field: "theme", value: next });
acceptCommittedSnapshot(snapshot);
```
