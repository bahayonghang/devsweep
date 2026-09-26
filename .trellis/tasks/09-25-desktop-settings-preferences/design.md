# Design: Settings And Runtime Preferences

## Ownership And Data Flow

Core owns DesktopPreferencesV1 validation, defaults, the fixed file path, and atomic storage. The Tauri adapter owns serialized get/update commands and committed notifications. The existing api/bridge.ts owns native calls and subscriptions. Components consume decoded values. A shared pure appearance module applies tokens to each document. Mode reducers keep domain state and operation ownership.

Settings control -> typed field update -> Rust validation and locked read/modify/write -> committed snapshot -> main/HUD appearance or next-operation runtime input.

Language retains its existing core store, IPC, i18n bridge, precedence, and save semantics. Do not add a fourth frontend Tauri import site or duplicate the persisted language in desktop preferences.

## Configuration Matrix

The matrix is the sole owner of the proposed field values and defaults.

| Field | Allowed values | Default | Consumer and apply timing |
| --- | --- | --- | --- |
| theme | dark, light, system | dark | Shared main/HUD semantic tokens; apply after commit; system listens for OS changes. |
| font_family | system, segoe_ui, microsoft_yahei_ui | system | Shared font stack tokens; apply after commit in both entries. Every choice has Chinese/English system fallbacks. |
| text_scale_percent | 100, 110, 125 | 100 | Scale actual font tokens after commit; main base remains 14 px and HUD base remains 13 px. |
| motion | system, reduced | system | Planet and decorative CSS transitions; apply after commit. OS reduced motion always wins. |
| planet_fps | 15, 30 | 30 | Planet scheduler; apply after commit. Disable the UI control with a reason when effective motion is reduced. |
| status_interval_seconds | 1, 2, 5, 10, 30, 60 | 2 | Persisted live interval; convert to milliseconds at the Status bridge boundary. Next explicit start uses it. An in-mode interval change retains the existing cancel/join/restart behavior after a successful save. |
| status_process_limit | 5, 15, 30, 50, 100 | 15 | Pass to both snapshot and live commands on the next start. Controls returned rows; does not claim a proportional collection CPU reduction. |
| hud_interval_seconds | 2, 5, 10 | 2 | Rust HudSampler input on the next HUD show. An open HUD keeps its current interval until hidden and shown again; the setting states that timing. |

The language selector keeps current en/zh-CN behavior. A new Follow system language option is outside scope. Font previews contain Chinese, English, and digits. Font choices use a closed local mapping; no arbitrary CSS value, font path, or network font enters the store.

Appearance defaults comprise theme, font_family, and text_scale_percent. Performance defaults comprise the other five fields. Each reset is one transaction in the desktop preference file. Language is excluded from both resets.

## Persistence Contract

Proposed path: %LOCALAPPDATA%/DevSweep/settings/desktop-preferences-v1.json.

The disk document has schema_version = 1 and the eight fields above. Core owns the closed Serde shape and refuses unknown fields, invalid enum values, unsupported schema versions, and values outside the selected sets. A missing file returns the documented defaults. Existing valid presentation-v1.json bytes are never rewritten by a desktop preference update.

Use the existing presentation store's locking and atomic-write pattern: process serialization, cross-process file lock, reread/validate current bytes while locked, apply a typed patch, write a same-directory temporary file, flush/sync, and atomic replacement. Reuse suitable existing private helpers only when that reduces duplication without widening the old store's contract. Do not introduce a database or a generic settings framework.

A typed patch changes one preference or resets one group. Rereading inside the lock preserves unrelated concurrent updates. Same-field concurrent changes follow commit order. The IPC never accepts a file path or arbitrary JSON keys.

Read failures preserve disk bytes and expose an unavailable warning. The desktop may render validated in-memory defaults for the new preference subsystem, with persistence controls disabled until a successful reload. Keep the existing language load gate unchanged. A write failure keeps the last committed values, offers retry, and does not emit a success notification. Reset cannot overwrite malformed or future-version bytes.

Language and desktop preferences use separate transactions. The UI saves fields independently and must not show a combined atomic Save All result. No automatic file migration is required for the existing language store. A future desktop schema requires explicit compatibility work.

## IPC And Multi-Window Updates

Add desktop_preferences_get and desktop_preferences_update app commands with core-owned payload types and fixtures. The main window may invoke both. HUD may invoke get only; retain the existing presentation read permission. All other HUD app commands remain denied.

Use a committed snapshot envelope with a process-local monotonic safe-integer sequence and the validated preference value. Tauri serializes reads and updates through one coordinator. A successful update emits the same ordered snapshot to both app windows. Failed updates emit nothing. The sequence is runtime ordering metadata, not a new disk version or cleanup identity.

Subscribe before the initial get. Each window accepts only a snapshot with a sequence at least as new as its current committed snapshot and rejects late initialization results after disposal. A newly created HUD loads the current snapshot; an already open HUD consumes committed appearance changes. Register/remove listeners with the existing async cleanup pattern. No continuous polling is introduced. Cross-process writers are protected by the store transaction; cross-process live broadcast is outside scope.

Keep unknown-payload decoding closed. Extend the existing fixture generator and Rust wire parity inventory for the new contract instead of hand-editing generated types. Update the explicit invoke/capability tests.

## Theme And Typography Application

Extract shared preference-to-appearance mapping used by both main and HUD entries. Resolve dark/light from theme plus matchMedia only when system is selected. Set a semantic theme attribute, color-scheme, font stack, text scale, and effective motion before showing each entry's usable content. Remove media listeners on disposal.

Define complete dark and light token sets. Audit existing literal text, border, error, warning, table, chart, capsule, dialog, and HUD colors that bypass tokens. Preserve mode accents and semantic meaning. Forced colors override decorative colors; do not use inversion filters or viewport-derived font sizes.

The many fixed pixel font sizes must consume scale-aware tokens. Preserve separate base sizes and numerical/tabular styles. Check controls, overflow, focus, long paths, and confirmation labels at 125 percent. Keep the native HUD window size unless evidence requires a scoped adjustment; use accessible overflow so larger text remains reachable.

Keep the existing procedural Planet renderer and palette ownership. Motion and FPS changes trigger effect cleanup and rescheduling; they do not change the source-resolution cap, turn duration, hidden-document stop, or forced-colors outline. Suppress matching decorative CSS transitions when effective motion is reduced.

## Status And HUD Runtime Consumption

StatusWorkbench receives committed preferences and a typed interval-update callback. Initialization uses the committed interval and process limit. The existing mode interval control writes through the same preference owner. Do not leave an independent persistent default in mode state.

On a live interval edit, save first. After success, request cancellation, join the owned run, and start a replacement with the committed interval. Preserve stale-event and request-generation rejection. A failed save keeps the current run and selection. A Settings route transition already drains work through the shell coordinator; saving a default there does not start Status automatically.

Supply status_process_limit to both snapshot and live requests. Core's collection ceiling, detail budget, process refresh cadence, power/volume cadence, and authority remain fixed. Returned rows are a presentation/result bound, not a process collection quota.

Make the HUD sampler interval an explicit validated input at show/start. Hide continues to cancel and join the sampler. Reading new settings must never start a hidden HUD or duplicate a sampler. Retain the HUD's current process-limit behavior; do not apply the Status process-row setting to HUD collection.

## Existing Work And Rollback

Do not change Clean/Analyze worker limits, traversal caps, safety policy, or existing performance thresholds. Longer intervals and lower animation caps are user options, not evidence that an existing default-workload failure is fixed. Record preferences with future measurements.

Update only affected desktop visual/state/type specs and the persistence owner's applicable guidance. New main/HUD behavior requires fresh focused evidence; old binary-bound acceptance results remain owned by the existing acceptance task.

Rollback removes the new preference consumers, commands, and schema owner as one unit while retaining user preference bytes. The old language document remains readable by the prior app and TUI. Do not delete user settings during code rollback.
