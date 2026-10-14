# Desktop Clean scan-root preferences

## 1. Scope / Trigger

Use when changing remembered desktop scan roots, scope editing, or the related
Tauri commands. Core `scan_preferences` owns validation and stored bytes.
Preferences select read-only discovery scope; they never authorize cleanup and
never change the CLI's explicit/default root semantics.

## 2. Signatures

Core exposes `load_scan_roots`, `save_scan_roots`, `validate_scan_roots`, and
`scan_root_suggestions`. Tauri exposes these closed responses:

```text
clean_scan_preferences_get() -> {version: 1, roots: string[], suggestions: string[]}
clean_scan_preferences_set(roots) -> the same snapshot shape
clean_scan_roots_validate(roots) -> the same snapshot shape, without persistence
```

The fixed Windows settings file is
`%LOCALAPPDATA%/DevSweep/settings/clean-scan-roots-v1.json`:

```json
{"schema_version":1,"roots":[]}
```

## 3. Contracts

- Roots are explicit absolute paths normalized and deduplicated by core. New
  selections must be existing directories with safe, non-reparse ancestry.
  Preserve path spelling as input until backend validation; the UI cannot grant
  authority through trimming, joining, or normalization.
- Empty preferences are valid. Starting a project scan requires at least one
  selected root; a global-only scan may have none. No hidden `.` default exists.
- Keep distinct nested selections. Removing a nested selection does not exclude
  it from a remaining parent root. This feature has no exclusion grammar.
- Reading creates no file. A missing file yields an empty list. Preserve valid
  stored paths that no longer exist so the user can inspect/remove them.
- Save validates the complete replacement, rereads existing bytes under the
  shared settings transaction lock, and uses the existing flushed atomic replace
  primitive. No generic preference-store framework or appearance reset behavior
  changes are needed.
- Corrupt, unknown-field, unsupported-version, or unreadable documents remain
  untouched. Session-only validation allows an explicit temporary scope while a
  visible persistence error explains why it cannot be remembered.
- Suggestions only perform metadata checks of known project/worktree containers.
  They are not recursive scans, automatic selections, or whole-worktree actions.
- Serialize edits and ignore stale startup/validation responses. Visible scope
  and last committed preferences cannot silently diverge after rapid edits.
  Scope changes immediately invalidate reviewed digest/confirmation state.
- Native calls remain in `api/bridge.ts`. HUD has no mutation/domain permission.
  DTO generation and Rust wire parity cover the actual closed response shape.

## 4. Validation & Error Matrix

| Input/state | Result |
| --- | --- |
| Missing store | Empty selection; no write |
| Absolute existing safe directories, duplicates | Normalized unique selections |
| Relative path, missing new root, reparse ancestry | Visible validation error |
| Valid stored root disappears | Keep it visible/removable; scan records actual failure |
| Corrupt/future/unknown-field store | Error and preserve bytes; session validation remains read-only |
| Save failure | Retain usable explicit session state and visible persistence error |
| Late startup read after edit | Ignore stale response |
| Only nested root removed | Parent selection still covers descendants |

## 5. Good / Base / Bad Cases

Base: first launch starts with an empty visible list and offers existing folders.
Good: a corrupt store is preserved while the user validates a session-only root.
Bad: silently falling back to the process CWD, whole-home scanning, or overwriting
a future store to make the UI look successful.

## 6. Tests Required

Use isolated temporary settings/project fixtures. Cover no-write reads, closed
schema, future/corrupt preservation, missing stored paths, duplicates, nested
roots, spaces/Unicode, unsafe ancestry, atomic-write failures and concurrent
edits. Frontend tests exercise delayed startup, serialized saves, failed saves,
project/global validation, and immediate preview invalidation. Native smoke
uses a packaged app launched from an unrelated working directory.

## 7. Wrong vs Correct

Wrong: a remembered folder automatically becomes a selected cleanup action.

Correct: remembered folders become explicit scan inputs; newly produced targets
still require trusted plan validation, user selection, live preview and confirm.
