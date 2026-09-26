# Backend Implementation Report

## Scope

The backend implements the frozen desktop preference contract. The shared
presentation-language store, cleanup authority, Status collection budgets,
HUD process-row limit, and window-control capabilities remain unchanged.

## Files Changed

- `crates/devsweep-core/src/lib.rs`: exports the desktop preference owner.
- `crates/devsweep-core/src/desktop_preferences/mod.rs`: closed V1 document,
  numeric choices, enum tags, typed patches, and group resets.
- `crates/devsweep-core/src/desktop_preferences/store.rs`: fixed path,
  fail-closed reads, locked read/modify/write, and atomic replacement.
- `crates/devsweep-core/src/desktop_preferences/tests.rs`: storage and schema
  behavior tests.
- `desktop/src-tauri/src/desktop_preferences.rs`: serialized reads and updates,
  positive safe-integer snapshot sequence, and committed window notifications.
- `desktop/src-tauri/src/lib.rs`: state registration, command inventory, and
  HUD read-only command gate. Existing custom-window capability changes remain.
- `desktop/src-tauri/src/service_boundary.rs`: includes the new adapter in the
  existing source-boundary inventory.
- `desktop/src-tauri/src/wire_parity.rs`: verifies the three frontend-owned
  preference fixtures against the actual Rust types.
- `desktop/src-tauri/src/hud.rs`: receives the interval at start; retains the
  lifecycle lock until cancellation and join complete.
- `desktop/src-tauri/src/tray.rs`: reloads preferences on each HUD show,
  captures the selected interval, and invalidates pending shows on hide/exit.

## Store And IPC

The disk path is `%LOCALAPPDATA%/DevSweep/settings/desktop-preferences-v1.json`.
The document includes `schema_version: 1` and exactly the eight preference
fields. Missing storage returns defaults. Unknown fields, unknown tags,
invalid numbers, malformed bytes, and unsupported versions return an error.
Reset patches have no `value` field; even `value: null` is rejected.

Windows transactions use the named OS mutex
`Local\DevSweep.DesktopPreferences.V1`. Unix transactions lock the settings
directory with `flock`, following the existing language-store mechanism. The
transaction rereads under the lock, applies one patch, writes a same-directory
temporary file, flushes and syncs the file, and atomically replaces the target.
The language-store implementation was not changed or widened.

`desktop_preferences_get` and `desktop_preferences_update({ patch })` return
`DesktopPreferencesSnapshot { sequence, preferences }`. The coordinator checks
sequence capacity before storage work. Failed operations neither advance the
sequence nor publish an event. Successful updates publish
`desktop-preferences-changed` to main and HUD under the coordinator lock. A
notification failure does not turn a committed write into a failed response.
The next get remains authoritative.

The main window can read and update. HUD can read desktop preferences and the
existing presentation language. HUD cannot invoke update or domain commands.
Storage failures use the existing structured `CommandError::Io { message }`.

Frontend-owned fixtures used by Rust parity:

- `desktop/src/api/fixtures/desktop-preferences.json`
- `desktop/src/api/fixtures/desktop-preferences-updated.json`
- `desktop/src/api/fixtures/desktop-preferences-patches.json`

## HUD Lifecycle

The actual sampler receives `Duration::from_secs(hud_interval_seconds)` on
show. A successful change does not replace a currently running sampler. The
next show reloads persisted preferences and publishes the ordered snapshot.
No appearance event starts or replaces sampling. The HUD still requests one
process row from the existing Status collector.

Window creation and preference I/O run outside the event-loop thread. Showing
the window and starting the sampler run together on the event loop. A
generation check refuses a late pending show after hide or exit. Stop retains
the sampler lifecycle mutex until the old thread joins; another start cannot
overlap the draining thread.

## Focused Checks

| Command | Result |
| --- | --- |
| `C:/Users/lyh/.cargo/bin/cargo.exe test --locked -p devsweep-core desktop_preferences --lib` | 8 passed |
| `cargo test --locked -p devsweep-desktop --lib desktop_preference` | 6 passed, including Rust/fixture parity |
| `cargo test --locked -p devsweep-desktop --lib hud` | 12 passed; actual 2/5/10-second cadence tests included |
| `cargo test --locked -p devsweep-desktop --lib hide_or_exit_invalidates_pending_preference_reload_before_show` | 1 passed |
| `cargo test --locked -p devsweep-desktop --lib only_the_main_window_reaches_mutating_commands` | 1 passed |
| `cargo test --locked -p devsweep-desktop --lib service_boundary` | 3 passed |
| Targeted `rustfmt --edition 2024 --config skip_children=true` on owned Rust files | Passed |

Core coverage includes all integer values from 0 through 255 for each numeric
field, exact default bytes, all closed tags, missing environment/storage,
unreadable entries, invalid/future-byte preservation during every patch/reset,
exact language-byte preservation, injected replacement failure with retry,
group reset scope, and eight simultaneous unrelated field patches.

Coordinator coverage includes ordered responses/events, failed writes, failed
event delivery after commit, concurrent ordering, and safe-integer exhaustion.
HUD coverage includes cancel/join, second-start refusal, no post-stop samples,
the selected actual cadence, and stale pending-show rejection.

## Evidence Limits And Follow-up

- Main-session final gates own `just ci`, the final desktop build, and combined
  frontend/native checks. These focused results do not replace those gates.
- Native tray/window behavior is not claimed from the unit tests. Main/HUD
  appearance, system-theme changes, reopen behavior, and both-locale layouts
  require the planned native/visual evidence.
- An additional self-spawning process test could not reopen the live test
  executable: Windows returned AccessDenied, then NotFound. The executable
  artifact was absent after the run with both cargo launchers. The cause is
  unconfirmed. The added subprocess test was removed. No system setting was
  changed. Threaded transaction tests exercise the production OS lock, but
  independent-process test evidence is not claimed.
- Existing performance and native-acceptance findings remain unchanged.

No commit or task archival was performed.
