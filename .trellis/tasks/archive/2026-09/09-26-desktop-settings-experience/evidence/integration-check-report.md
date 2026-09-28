# Independent Integration Check

Review date: 2026-09-28 UTC.
Reviewer: dispatched trellis-check, palettes_review_final.
Task: 09-26-desktop-settings-experience, with palette and controls repair scope.
Evidence paths below are relative to this evidence directory unless stated.
Source paths are relative to the repository root.

## Decision

**Reviewed repairs and recorded desktop checks: PASS. Canonical CI: BLOCKED.**
No unresolved code defect was found in the independently reviewed palette and
IME repairs. The final frontend gate, desktop Rust gate, Clippy, release build
and recorded native flows pass. The required full core test gate remains
unexecuted or incomplete. The cause remains unknown. These results do not
establish complete parent acceptance or authorize task archival.

This report updates integration evidence after the IME repair. The prior
palette review is retained at
../../09-26-desktop-settings-palettes/evidence/check-report.md; its historical
pending-gate statements are superseded by the dated gate results below.
The IME source/event audit is at
../../09-26-desktop-settings-controls/evidence/ime-check-report.md.
The reviewer did not repeat the already reviewed palette, core validation or
Tauri assertion repairs. Production, tests, specs, dependencies and task status
remained read-only. Only these authorized review reports were written.

## Findings (fixed)

| Finding | Verified resolution |
| --- | --- |
| Protection confirmation palette override | The support dialog uses text/raised/control-border tokens in normal rendering and system colors under forced colors. The previous independent source/style review passes. Archived native visual evidence includes the four named-palette dialogs. |
| Core collapsible_if lint | The Installed-font condition uses a behavior-preserving let chain. Prior independent reconstruction matched the original module hash. Final Clippy passes. |
| Tauri cloned_ref_to_slice_refs lint | The event assertion borrows a one-element expected slice and preserves full count/content equality. Final Clippy and desktop Rust tests pass. |
| IME composing keys persisted a font | SettingsCombobox.tsx:47-51 skips Base UI keyboard handlers for native isComposing or keyCode 229, while preserving browser defaults and explicit pointer choice. Independent 41-test coverage and rebuilt native composition evidence pass. |

## Frozen source and artifacts

The reviewer rehashed all 59 files in integrated-source-final.json. All 59
match. Independent comparison with integrated-source.json found exactly two
changed files:

- desktop/src/components/SettingsCombobox.tsx: the IME guard.
- desktop/src/components/settings-choices.test.tsx: regression coverage.

The only production difference is the Combobox guard. CSS, palette tokens,
HUD and backend sources are unchanged from the pre-IME integrated snapshot.
The previously reviewed source and visual evidence for those unchanged areas
remains applicable within its original test boundaries.

The final source hashes match the repair handoff:

| Source | SHA-256 |
| --- | --- |
| SettingsCombobox.tsx | 060c994390351faec055472cea68dde7a2f33a975c3edb1f047aa23fd6260a72 |
| settings-choices.test.tsx | 436f026b90dfb5c2fdf32ff367cec3305d69de13c8269b86fb9af267b720f746 |

Both artifact hashes and byte sizes also match the final manifest:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| target/release/devsweep-desktop.exe | 13552128 | 03c5b34fa650696f026b1ec982fc70ab8d93e2a8f9665b5b4df4b36f2d2b567d |
| target/release/bundle/nsis/devsweep_0.3.0_x64-setup.exe | 4064534 | a21784f549c83d8a22ff735dfde980b7bd2e8c237cef2d262ecf75303943cb1c |

The native-final/session.json and native-composition.json records identify
that same final executable hash. The installer was built; this review did not
install it.

## Verification

| Gate | Independently checked evidence and result |
| --- | --- |
| Final desktop-web-check | desktop-web-check.json and log: exit 0, 03:19:53.663-03:20:25.329 UTC. Generated types, full ESLint, TypeScript, 51 Vitest files / 387 tests, 5 Node tests and production web build all pass. |
| Independent IME checks | Reviewer ran controls, fonts and preferences tests: 3 files / 41 tests pass. Scoped ESLint, desktop TypeScript and diff whitespace checks exit 0. Source hashes match the handoff. |
| Rust Clippy | clippy-corrected.json and log: just clippy exits 0 at 02:52:38.636 UTC. No Rust source changed after that run. |
| Desktop Rust tests | desktop-test.json and log: exit 0 at 02:53:13.709 UTC; 83 pass, 2 ignored. The record is after both mechanical Rust repairs and before the frontend-only IME repair. |
| Release build | desktop-build.json: exit 0, 03:20:25.347-03:23:32.079 UTC. Executable and NSIS hashes were rechecked. |
| Contrast | Prior independent recomputation: 468/468 declared opaque sRGB pairs pass. Palette source hashes remain unchanged. |
| Canonical just ci | ci.json/log: exit 1. The core executable was never executed because of os error 2. A prior direct-core run ended with 0xc0000005. No final full core test pass exists. |
| Isolated core diagnostic | core-isolated.json records the actual toolchain Cargo, a separate Temp target, buildExit=0, and existsBefore=false for the reported core test executable. Result is BLOCKED. No cause is established by the diagnostic. |

The 14 previously observed preference-test successes and the focused wire
parity result are historical evidence. They do not replace the failed full CI
or the final focused retry that could not execute. No Cargo diagnosis was
repeated by the reviewer.

## Native evidence audit

The parent executed the native tests on an isolated release profile. The
reviewer independently inspected the archived records and relevant helper
assertions. All final raw records are under native-final/; earlier raw records
are under native-pre-ime/.

| Record | Result and checked scope |
| --- | --- |
| native-final/native-flow-first.json | PASS, seven recorded checks. Covers native catalogue, read/query/Escape without persistence, first/middle/last font choices, V1/language preservation, explicit refresh, failed-save preservation and restart preparation. |
| native-final/native-flow-restart.json | PASS, restarted process applies the committed font and palette. |
| native-final/native-navigation.json | PASS, en and zh-CN cases with keyboard navigation and history. No operational command was started by this probe. |
| native-final/native-matrix.json | PASS. 44 page records, 156 popup records and 28 media records. Reviewer scan found zero recorded page overflow/text/error failures and zero popup boundary/portal/font failures. Widths: 390/800/1024/1440 CSS pixels. Text scales: 100/110/125 percent. Both locales covered. Media cases: 14 scheme, 7 forced-colors and 7 reduced-motion. |
| native-final/native-composition.json | PASS, 03:25:53.893-03:25:54.144 UTC. True native composing ArrowDown/Enter; Segoe UI remains committed; before/during/after preference hashes are equal. Explicit renderer text commit emits compositionend and narrows 249 options to 12 matching aliases. |
| native-final/native-settings-screenshots.json | PASS record with four Settings captures from the final binary, one per new named palette. The reviewer checked the record and binary identity; no independent pixel-by-pixel review is claimed. |
| native/native-visuals.json | Pre-IME PASS record: four named themes, four support dialogs, 14 main/HUD scheme checks and 13 captures. Main and HUD scheme values agree in all 14 recorded cases. The record belongs to native/session-pre-ime.json. It is retained as pre-IME evidence, with unchanged CSS/HUD/palette sources verified by hash. |

The matrix helper uses Emulation.setDeviceMetricsOverride and
Emulation.setEmulatedMedia. Its geometry and media results are native WebView2
renderer evidence under emulated conditions. They do not establish a physical
Windows display-scale or accessibility-theme matrix. Accessibility trees are
recorded in native-final/settings-accessibility-en.json and
settings-accessibility-zh-CN.json; screen-reader speech was not exercised by
this reviewer.

## IME evidence chronology

- The retained old-binary reproduction in
  ../../09-26-desktop-settings-controls/evidence/native-ime-before-fix.json
  ran at 03:19:46.962-03:19:53.519 UTC. It records true composing keys,
  Segoe UI changing to System, unequal hashes and the explicit error
  Composing Enter persisted a font choice.
- The initial 03:10 probe had a later popup-close timeout after recording an
  earlier hash change. That JSON was superseded; no retained copy is known.
  It is not cited as an available artifact.
- native-final/native-composition-no-explicit-commit.json records a final-binary
  timeout waiting for compositionend. The three preference hashes are equal.
  CDP key dispatch alone did not perform an IME text commit.
- native-final/native-composition-english-label-assertion.json records a second
  helper timeout: the query matched aliases while display labels were English.
  Compositionend was recorded and all preference hashes remained equal.
- The final helper commits renderer text through Input.insertText after the
  composing-Enter assertion. It checks that the query is 宋 and results are
  nonempty and narrower than the original list. All persistence checks remain.
  No product code changed between these final-binary probe attempts.

## Findings (not fixed) and acceptance limits

No unresolved code defect was found in the reviewed changes. The following
verification boundaries remain open:

1. Canonical core CI is blocked. The missing executable and earlier native
   termination have no established cause. Passing frontend, Clippy, desktop
   tests and isolated build exit codes do not close that gate.
2. Windows IME candidate-window interaction was not exercised. The successful
   probe tests native renderer composition and explicit CDP text commit.
   Manual candidate-window feedback remains pending from the user.
3. Viewport, color-scheme, forced-colors and reduced-motion conditions were
   emulated. Physical Windows scaling changes and human screen-reader speech
   were not verified here.

Do not mark the parent or all controls acceptance criteria complete from this
report. No task status, commit, archive, install or push was performed.
The blocked CI does not permit a task-completion or commit-plan transition.
