# Host Fonts Implementation

Date: 2026-09-26. Active child: `09-26-desktop-settings-host-fonts`.
Implementation and focused checks are complete. The parent owns independent
review, final integration gates, and native Windows acceptance after palettes.
The child task status and acceptance checkboxes remain unchanged.

## Changes and ownership

- Core preference files: `crates/devsweep-core/src/desktop_preferences/mod.rs`,
  `store.rs`, `tests.rs`, and new `legacy.rs`. V2 stores a tagged system or
  installed-family font. Installed families preserve Unicode and punctuation;
  validation rejects empty, untrimmed, control-containing, or over-256-scalar
  names. The private strict V1 decoder preserves the old theme and font enums.
  Read conversion maps all three old presets and preserves every other field.
- Storage reads the fixed V2 path first. Only a missing V2 file permits V1
  lookup. Read conversion writes nothing. The first update rereads under the
  existing shared directory transaction lock, applies the patch, and atomically
  creates V2. V1 and language bytes remain unchanged. V2 failures never fall
  back to V1. Downgrade uses the preserved V1 snapshot; no reverse migration
  copies later V2 changes into V1.
- Native adapter: new `desktop/src-tauri/src/fonts.rs` plus command registration
  in `lib.rs` and the explicit source registry in `service_boundary.rs`.
  The async command runs DirectWrite enumeration on a blocking worker.
  `IDWriteFactory3::GetSystemFontCollection` disables downloadable fonts and
  requests current local collection data. Native objects remain on that worker;
  only owned family and localized-name strings cross the worker boundary.
  Families use the en-US name when present, otherwise the first nonempty native
  name. Case-insensitive deduplication merges aliases without a result cap.
  Explicit unavailable outcomes cover enumeration failure and unsupported hosts.
- `desktop_preferences.rs`, `hud.rs`, and `tray.rs` consume the V2 payload.
  Committed snapshot sequencing, update ordering, and HUD permissions retain
  the existing authority boundary. The new font command is main-window-only.
  Font discovery does not start Status or HUD sampling.
- IPC files: `desktop/scripts/generate-types.mjs`, `desktop/src/api/contract.ts`,
  `bridge.ts`, `bridge.test.ts`, generated `types.gen.ts`, V2 preference and patch
  fixtures, contract variants, new font result fixtures, and Rust wire-parity
  tests. The renderer rejects unknown fields and invalid font strings.
  Unicode White_Space validation matches Rust trimming, including retention of
  an allowed leading BOM scalar. The font bridge sends no caller parameters.
- UI files: new `desktop/src/preferences/fonts.ts` and `FontPicker.tsx`;
  `PreferencesProvider.tsx`, `SettingsPage.tsx`, `appearance.ts`, `fixture.ts`;
  `App.tsx` and `main.tsx`; and the existing `SettingsCombobox.tsx` wrapper.
  Settings lazily loads one cached catalogue per bridge, coalesces requests,
  refreshes only on explicit refresh, retains known choices after failure, and
  discards disposed completions. Main and HUD preference consumption alone
  does not enumerate fonts. The picker searches canonical and localized names.
  System remains available during loading or failure. Arbitrary query text
  never becomes a saved font family. Missing saved names remain visible.
- Shared appearance serializes one quoted CSS family, escaping quotes and
  backslashes, then appends the existing system fallbacks. Main and HUD use
  the same serializer and root variable. Technical monospace remains separate.
  `desktop/src/styles.css` adds only the font-action row and notice layout.
  `resources/i18n/en.json` and `zh-CN.json` add font lifecycle messages.
- Tests: new `preferences/fonts.test.tsx`; updated preference, bridge, and
  choice tests. Relevant backend preference/database and desktop state/type
  specs plus `code_map.md` document the V2 and native-font owners. Earlier
  navigation and controls changes remain in the shared working tree.

## Dependency evidence

The approved dependency is `windows = =0.61.3`, restricted to the Windows target
of the desktop adapter with `Win32_Graphics_DirectWrite`. Cargo.lock already
contained that exact package; the lockfile change adds only its direct desktop
dependency edge. The installed registry manifest reports Rust 1.74, compatible
with the repository's Rust 1.88 baseline, and `MIT OR Apache-2.0`. The installed
feature entry maps DirectWrite to `Win32_Graphics`. The Windows build in the
desktop test gate compiled the exact binding and method signatures.

## Checks run

| Command | Result |
| --- | --- |
| `cargo test --locked -p devsweep-core desktop_preferences` | Exit 0; 13 passed |
| `just desktop-test` | Exit 0; 83 passed, 2 intentionally ignored process fixtures; binary/doc targets passed |
| `mise exec node@22 -- npm run types:generate` in `desktop/` | Exit 0; generated V2 and font contracts |
| `mise exec node@22 -- npx vitest run src/preferences/fonts.test.tsx src/preferences/preferences.test.tsx src/components/settings-choices.test.tsx src/api src/i18n src/styles.test.ts --reporter=default --reporter=json --outputFile.json=../.trellis/tasks/09-26-desktop-settings-host-fonts/evidence/focused-tests.json` | Exit 0; 11 files, 116 tests passed |
| `mise exec node@22 -- npm run lint` in `desktop/` | Exit 0 |
| `mise exec node@22 -- npm run typecheck` in `desktop/` | Exit 0 |
| Scoped Rust formatting and `git diff --check` | Formatting completed; diff check exit 0 |

`focused-tests.json` and `focused-tests.log` contain the final web run.
`font-interaction-measurement.json` contains the diagnostic large-list sample.
The measured environment was Windows x64, Node v22.23.2, Vitest/jsdom. All
2,000 families plus System were reachable; first/middle/last and Chinese alias
queries passed with one bridge enumeration call. The sample recorded 2089.10ms
to open/render the full list and query samples of 92.45ms, 28.21ms, and 16.94ms.
These are diagnostic jsdom measurements, not native WebView latency evidence.

## Failures corrected during implementation

- A Serde internally tagged unit variant accepted extra fields. System now
  uses an empty struct variant under `deny_unknown_fields`. Rust and renderer
  tests reject extra fields on the System font.
- The new native module required an entry in the exhaustive source-boundary
  registry. The final desktop test gate includes that entry and passes.
- Under concurrent Vitest files, an immediate Select option assertion could
  run before Base UI synchronized its portal state. A temporary open-state
  trace showed the trigger open event with no close event while the immediate
  DOM assertion still saw the prior state. The tests now await the observable
  option with `findByRole`. The temporary trace is removed; no production Select
  behavior change or retry loop was added. The final combined test run passes.
- A request completion and final listener disposal in the same turn could
  leave the cached loading state active. Disposal now invalidates an active
  loading snapshot even if the Promise already cleared its pending flag.
  A dedicated settle/dispose/remount regression passes.

## Acceptance evidence and remaining parent work

| Criterion | Child evidence | Remaining evidence |
| --- | --- | --- |
| F-AC1 | Native family-level enumeration, deterministic deduplication and 2,000-family unit coverage | Actual OS catalogue count, first/middle/last native selections, and any available per-user families |
| F-AC2 | 2,000-family reachability, canonical/alias search, single enumeration during typing, loading/empty/error, retry, refresh coalescing, disposed and same-turn settlement tests | Native interaction latency and keyboard/IME behavior |
| F-AC3 | Shared main/HUD CSS application tests, escaping and injection tests, retained missing names, committed-value retention through errors | Real main/HUD rendering, persistence across process restart, and native fallback rendering |
| F-AC4 | All V1 presets, unrelated-field equality, no write on read, first save preserving V1/language bytes | Complete in focused core tests |
| F-AC5 | Valid V2 precedence; strict V1/V2 failure matrix; read-only refusal; atomic replacement failure; concurrent first-save patches; failed commit publishes no event | Complete for the tested Windows store and command paths |
| F-AC6 | Positive safe sequence; pending read/event ordering; disposal/reload/HUD-show hooks; group-only resets | Real native main/HUD integration after palettes |
| F-AC7 | Explicit command allowlist tests deny HUD fonts/update; owned-string-only DTOs; parameterless bridge; separate non-Windows unavailable implementation | Non-Windows build/test was not run on this Windows host |

No fonts were installed or removed. User preference files and OS display
settings were not changed. The child did not run actual OS font discovery or
claim fixture data as native evidence. No commit, archive, task-status change,
new theme, or extra dependency was made. Parent instructions assign the final
`just ci`, full frontend gate, production build, and isolated native evidence
to the integration phase after the palette child.

## Palette child handoff

Extend only the V2 theme enum, matching fixture catalogue, decoder, and generated
TypeScript. `legacy.rs` has its own strict Dark/Light/System enum so V2 themes
cannot be accepted by the V1 reader. Keep the V2 filename and migration rules.
The tagged font object is final for this unreleased series. Do not release the
intermediate three-theme V2 independently. The font fixture bridge is injected
through `PreferencesProvider` for renderer tests; the real bridge invokes
`desktop_fonts_list` only when Settings mounts the picker or the user refreshes.
