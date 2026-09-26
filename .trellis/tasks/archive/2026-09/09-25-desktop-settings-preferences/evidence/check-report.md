# Settings Preferences: Check Report

Date: 2026-09-26. Review scope includes this child and its custom-titlebar integration.
Node: 22.23.2 through `mise exec node@22`.

## Findings (fixed)

- File: `desktop/src/preferences/store.ts:13` and `:25`.
  Issue: The reload effect reset the committed sequence to zero. A delayed old
  event received after resubscription could overwrite newer committed values.
  Fix: Retain the committed snapshot when the bridge identity is unchanged.
  Reset the sequence only when a different bridge owns the resource.
- File: `desktop/src/preferences/preferences.test.tsx:67`.
  Fix: Add a regression case that commits sequence 5, reloads, rejects an older
  event, and then accepts the sequence 6 read. The focused file passes 11 tests.

## Findings (not fixed)

No additional product defect was identified in the reviewed paths. The full
Rust gate remains incomplete because the core unit-test executable could not
be started. The operating system reported a missing file before any core unit
test ran. No core assertion failure was observed. The cause is unknown.

The failure occurred with both the PATH Cargo wrapper and the native Cargo
binary. A separate native Cargo invocation for all core targets also failed
before starting the core unit-test executable. No security setting, build
cache, test threshold, or product code was changed to address that failure.

Native window, DPI, tray, and live cross-window acceptance remain unverified
in this review. The parent records the text-only browser evidence and native
limitations in `.trellis/tasks/09-25-desktop-window-settings/evidence/`. No screenshot,
image capture, image generation, or image inspection was used. Existing
performance and native-acceptance task findings remain unchanged.

## Verified Code Paths

- Core owns the exact closed V1 document, numeric choices, group resets, fixed
  path, locked read/modify/write, and same-directory atomic replacement. The
  existing language store remains separate. Invalid stored bytes cannot be
  overwritten by a field patch or reset.
- Tauri serializes reads, commits, and notifications. Failed writes publish no
  snapshot. Event delivery failure preserves the successful commit result.
  Main can read/update; HUD can read preferences and language only.
- The frontend subscribes before loading, checks sequence and disposal, keeps
  committed values during failed saves, and uses one shared appearance adapter
  for main and HUD. System theme and reduced motion have media listeners.
- Settings and Status share the committed interval. Both Status request paths
  receive the selected row limit. Active interval replacement commits first,
  then uses the existing cancel/join path before starting a replacement.
- The selected frame cap reaches the Planet loop. Motion, visibility, and
  disposal preserve loop cleanup and the existing source and rotation bounds.
- HUD captures its interval on show. Hide/exit invalidates pending shows and
  cancels/joins the sampler. The lifecycle lock prevents overlapping samplers.
- Custom titlebar controls retain the native-state adapter, exact main-window
  permissions, and close-after-drain path. Appearance updates add no cleanup
  authority or domain-state owner.

The main session added the reload sequence case to
`.trellis/spec/backend/desktop-preferences.md`. No further spec change was
identified by this review.

## Verification

Commands with `mise` ran from `desktop/`. Rust commands ran from the repository
root. Native Cargo commands used `C:/Users/lyh/.cargo/bin/cargo.exe`.

| Command | Result |
| --- | --- |
| `mise exec node@22 -- npm exec -- vitest run src/preferences/preferences.test.tsx` | Pass: 11 tests, 1 file |
| `mise exec node@22 -- npm run types:generate` | Pass: generated from 44 named fixture files |
| `mise exec node@22 -- npm run lint` | Pass |
| `mise exec node@22 -- npm run typecheck` | Pass |
| `mise exec node@22 -- npm run test` | Final run passed: 330 Vitest tests in 48 files, plus 5 Node tests in 2 suites |
| `mise exec node@22 -- npm run build` | Pass: main/HUD production entries, 148 modules transformed |
| `git diff --check -- desktop crates/devsweep-core/src/desktop_preferences crates/devsweep-core/src/lib.rs resources/i18n` | Pass; Git emitted only line-ending notices |
| `just ci` with the default PATH | Incomplete: fmt/check passed; CLI tests passed; core executable missing before test start |
| `$env:PATH = 'C:\Users\lyh\.cargo\bin;' + $env:PATH; just ci` | Same incomplete result with native Cargo |
| Native Cargo: `test --locked -p devsweep-core --all-targets` | Could not start core unit-test executable; no core unit tests executed |
| Native Cargo: `test --locked -p devsweep-core --test public_api` | Pass: 1 test |
| Native Cargo: `test --locked -p devsweep-desktop --all-targets` | Pass: 81 tests; 2 process-fixture entrypoints ignored; binary target has 0 tests |
| Native Cargo: `clippy --workspace --locked --all-targets -- -D warnings` | Pass |

The first complete frontend test attempt ran while Rust compiled. That attempt
passed 329 tests and timed out in
`src/api/types-generation.test.ts > changes when a named fixture changes`
at the unchanged 5000 ms limit. The independent rerun passed every test. No
frontend source or timeout changed between those two full-suite attempts.

Both `just ci` attempts passed CLI unit tests (184), CLI contract tests (12),
and five-mode contract tests (7). `just ci` stopped before running Tauri tests
or Clippy; the separate commands above completed those checks.

Missing core unit-test executables, all reported as `never executed` with
`The system cannot find the file specified. (os error 2)`:

- Default PATH: `target/debug/deps/devsweep_core-0942b4e2b82cca0a.exe`.
- Native Cargo through `just ci`: `target/debug/deps/devsweep_core-9d4fd824cd3f7dc0.exe`.
- Native Cargo core-only command: `target/debug/deps/devsweep_core-2ca6c9bc5b0fca68.exe`.

`just ci` uses `--workspace --all-targets`, so its Rust scope includes
`desktop/src-tauri`. The command does not build a final Tauri release
executable or installer. This review produced only the requested web
production build. No commit or task archive was performed.
