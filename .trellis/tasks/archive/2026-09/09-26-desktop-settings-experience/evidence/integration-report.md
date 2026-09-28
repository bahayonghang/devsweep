# Desktop Settings Integration

Updated: 2026-09-28 UTC (2026-09-27 America/Chicago).
Status: implementation and targeted integration checks passed; canonical CI blocked. The user requested commits and archival with the disclosed limits; see closure-decision.md.

## Result and scope

The four approved children were implemented in order: Navigation, Controls, Host Fonts, and Palettes. Settings is the sixth primary tab, immediately after Status. Seven fixed Select controls and a searchable font Combobox share themed popup surfaces. DirectWrite returns local font families without a preset cap. This host returned 248 families. Seven theme choices include Catppuccin Latte, Catppuccin Mocha, Codex, and Claude.

Native testing found a Chinese composition bug: composing Enter could select a font in Base UI. The repair skips the Base UI key handler for native composition and legacy key code 229 while preserving browser default action and event propagation. Independent review found no remaining code defect. Full task closure remains blocked by the core CI gate.

The user subsequently requested local commits and archival. The feature commit is 5e73a4fb2e329c305d400efbb7670684f998d709. See closure-decision.md for the recorded instruction and validation limits. No push or installer execution occurred. The separate operation-performance and native-acceptance tasks retain their own scope.

## Acceptance evidence

Paths in the table are relative to this task's evidence directory. Child implementation/check reports supply the corresponding unit and source review evidence.

| ID | Result and evidence | Boundary |
| --- | --- | --- |
| AC1 | PASS: native-final/native-navigation.json records six ordered tabs, selected state, and panel association in both locales. | Native renderer evidence. |
| AC2 | PASS: keyboard focus, End/Enter activation, history, and 390 CSS px visibility; Navigation tests cover drain failure, stale transitions, and operational state. | No cleanup operation was started for navigation testing. |
| AC3 | PASS: 156 native popup cases, keyboard/component checks, dismissal, and failed-save recovery. | Matrix geometry uses renderer viewport emulation. |
| AC4 | Renderer checks PASS: both locales, 390/800/1024/1440 widths, all three text scales, Chinese composition, forced colors, and reduced motion. | Actual Windows IME candidate-window interaction remains unverified; the user requested archival without supplying a manual result. OS display scale was not changed. |
| AC5 | PASS: 248 installed families plus System; first/middle/last families searchable and selectable; refresh retains the count. Uncapped enumeration and family deduplication have code and fixture coverage. | The 2,000-family measurement is a jsdom fixture, not native latency. |
| AC6 | PASS in recorded scopes: saved font reaches main/HUD and restart; missing-family, CSS serialization, and discovery failure have focused coverage. | Final binary covers main/restart; actual HUD evidence precedes the isolated keyboard repair. |
| AC7 | PASS in recorded scopes: seven IDs, six resolved palettes, 55 semantic tokens, and four final-binary Settings screenshots. Actual HUD and protection-dialog captures cover all four new palettes. | HUD/dialog captures use the pre-IME binary; their source hashes are unchanged. |
| AC8 | PASS: 468 calculated contrast pairs; native forced-colors and reduced-motion cases; selected/risk states retain non-color signals. | Accessibility media were emulated in WebView2, not toggled in Windows Settings. |
| AC9 | PASS: saved theme/font survive restart; failed save retains committed theme; System resolution checked in main and HUD. | Main is final binary; HUD is pre-IME binary. |
| AC10 | PASS in recorded tests: strict read-only V1 migration, first V2 save, and V1/language byte preservation. All 14 preference tests passed in an earlier partial core run. | The full core process later crashed; that partial run does not pass the core gate. |
| AC11 | Integrity tests and independent source review passed in recorded scopes; final desktop Rust tests passed. | Required canonical core CI is BLOCKED. |
| AC12 | Combined native navigation/font/theme/restart flow passes; actual HUD synchronization was captured before the isolated keyboard repair. | Required full CI prevents task completion. No final-binary HUD replay is claimed. |

Independent reports: integration-check-report.md and ../../09-26-desktop-settings-controls/evidence/ime-check-report.md.

## Final native build and source identity

integrated-source-final.json records 59 source-file hashes and both release artifacts. Compared with integrated-source.json, only two files changed:

- desktop/src/components/SettingsCombobox.tsx: composition key guard.
- desktop/src/components/settings-choices.test.tsx: three added regressions.

All palette, CSS, HUD, and Rust source hashes in that snapshot are unchanged. The final executable is target/release/devsweep-desktop.exe, 13,552,128 bytes, SHA-256 03c5b34fa650696f026b1ec982fc70ab8d93e2a8f9665b5b4df4b36f2d2b567d. The NSIS installer was built but not installed. Its identity is in the same manifest. Generated binaries remain under ignored target/.

The isolated final profile is target/settings-native-final-20260928/. First and restarted processes have separate session records. Tests use rendered controls and inspect only that profile. No product IPC test hook, font installation, system preference change, or cleanup execution was used.

## Native results

| Evidence | Result |
| --- | --- |
| native-final/native-flow-first.json | PASS: 248 families; Agency FB, Microsoft Himalaya, and Zhuque Fangsong (technical preview) selected; read/query/Escape do not write; refresh works; V1/language bytes preserved; failed save retains committed theme. |
| native-final/native-flow-restart.json | PASS: saved Codex theme and installed font survive a process restart. |
| native-final/native-navigation.json | PASS: English and Simplified Chinese, six tabs, keyboard focus, Settings activation, panel semantics, and back/forward at 390 CSS px. |
| native-final/native-matrix.json | PASS: 44 page cases, 156 popup cases, and 28 media cases. English and Chinese AX trees are retained beside the report. |
| native-final/native-composition.json | PASS: native composing ArrowDown/Enter do not save; explicit renderer text commit emits compositionend; Chinese alias query narrows 249 options to 12 families; Escape preserves the saved font. |
| native-final/native-settings-screenshots.json | PASS: four final-binary Settings captures with open themed popups. All four images were visually inspected; no horizontal clipping was observed. |
| native/native-visuals.json | PASS on pre-IME binary: four main/HUD pairs, four protection confirmations opened and canceled, 14 shared scheme checks, and 13 screenshots. User opened the real HUD through the tray. |

Pre-IME native records are preserved under native-pre-ime/. The actual HUD WebView reported visible and unfocused when captured. Its renderer capture does not establish OS tray placement or native-window visibility after blur. Matching pre-IME binary identity: native/session-pre-ime.json.

## Composition regression and harness corrections

The old-binary regression is retained in the Controls child as evidence/native-ime-before-fix.json. Native Enter had isComposing=true; saved font changed from Segoe UI to System. The script failed with Composing Enter persisted a font choice. The independent repair report records red tests and 41/41 passing focused tests after repair.

The final-binary report records identical before/during/after preference hashes, with Segoe UI retained. CDP starts renderer composition but does not drive a Windows candidate window. The harness uses Input.insertText after the composing-Enter assertion to commit text. English display names can match Chinese aliases; the test records the actual narrowed list.

Two earlier final-binary harness failures are retained. The native-composition-no-explicit-commit.json report timed out waiting for a commit that the harness had not issued. The native-composition-english-label-assertion.json report incorrectly required a Chinese character in an English display name. Both records show unchanged saved-font hashes. No product change followed those two harness corrections. Real Windows candidate-window interaction is not covered by CDP.

## Product gates

| Command | Result |
| --- | --- |
| mise exec node@22 -- just desktop-web-check | PASS after IME repair: 51 Vitest files, 387 tests, 5 Node tests, generated types, ESLint, typecheck, production build. |
| just desktop-test | PASS: 83 tests, 2 intentionally ignored fixture entrypoints. Rust source remained unchanged after this run. |
| just clippy | PASS after the two recorded lint repairs; see clippy-corrected.json/.log. |
| mise exec node@22 -- just desktop-build | PASS after IME repair: release executable and NSIS installer. |
| just ci | BLOCKED: two attempts reached core tests but could not execute the missing binary. Format/check and CLI suites passed. |

CLI suites in the failed canonical gate passed 184 unit tests, 12 CLI contract tests, and 7 five-mode contract tests. They do not substitute for the missing core result. An earlier direct core run recorded 203 passed tests, including all 14 preference tests, before STATUS_ACCESS_VIOLATION. No crash stack identifies a cause.

A separate real-toolchain --no-run build used a new ordinary target directory outside the shared target link. Cargo exited 0 and reported a test executable; the executable was absent immediately afterward. No test body ran in that attempt. Existing readable Defender, CodeIntegrity, and AppLocker logs contain no event in the inspected interval; Security access was denied. These observations do not identify or exclude a security actor. The cause remains undetermined. See core-gate-diagnostic.md and core-isolated.json. No security configuration was changed.

## Remaining boundaries and next step

- Restore a runnable core test artifact through an evidence-backed build or filesystem diagnosis, then rerun canonical just ci. Do not infer an application defect from the last printed test or bypass host protection.
- Retain the manual Windows candidate-window boundary until a user result is available. Do not claim OS-scale or native high-contrast interaction from renderer emulation.
- The frontend build retains the shared-chunk size warning (655.16 kB, 167.68 kB build-reported gzip). No unrelated chunk refactor is included. Non-Windows native builds were not run on this host.
- The reviewed checkpoint kept the task open because required checks were blocked. The subsequent user instruction requests commits and archival; closure-decision.md records that administrative closure while preserving the unpassed checks.
