# Palette Independent Code Review

Review date: 2026-09-27 America/Chicago (2026-09-28 UTC).
Reviewer: dispatched `trellis-check`, `palettes_review_final`.
Task: `09-26-desktop-settings-palettes`.

## Decision

**Code findings resolved; integrated acceptance pending.** All three repairs
below pass independent source review. Focused frontend checks and the final
full frontend gate pass. No unresolved palette code defect was found. Full
Rust gates and native acceptance are not established by this report.

The review read the task PRD, design, implementation plan, check manifest,
curated specs, parent requirements/design/approval, and implementation evidence.
The review traced production source and test assertions independently.
Only this report was written. Production code, tests, specs, task status,
dependencies, and other agents' evidence were preserved.

## Findings (fixed)

The parent serialized these repairs through `palette_dialog_repair`. The
reviewer kept production read-only and independently verified all three repairs.

### F1 — P2: Protection confirmation bypassed the selected palette

- File: `desktop/src/support/styles.css:18`.
- Requirement: T2 / T-AC2; parent R4 explicitly includes supporting pages and
  confirmations in the shared semantic palette.
- Cause: the ordinary `.support-page dialog` rule set `color: CanvasText`,
  `background: Canvas`, and `border: 1px solid CanvasText` outside the
  `forced-colors` media block. The rule has greater specificity than the shared
  `dialog` token rule in `desktop/src/styles.css:180`.
- Actual path: `desktop/src/support/ProtectionPage.tsx:8` imports the stylesheet;
  line 66 creates `.support-page`; lines 105-110 render the Add/Remove Protection
  confirmation inside that section. Both confirmation actions reach the rule.
- Fix: the normal rule now uses `var(--text)`, `var(--raised)`, and
  `var(--control-border)`. The stronger border token provides measured boundary
  contrast of at least 3:1 on both the dialog and page surfaces. The later
  same-specificity forced-colors override at lines 26-32 retains `CanvasText`,
  `Canvas`, and `forced-color-adjust: auto`.
- Regression evidence: `desktop/src/styles.test.ts:76-92` requires the normal
  semantic declarations and the later system-color override. Lines 42-48 also
  require `text/raised`, `control-border/raised`, and
  `control-border/stage-canvas` measurements for every resolved palette. The
  reviewer inspected the actual consumer, selector order, and assertions, then
  independently ran 14/14 style tests, scoped ESLint, and desktop TypeScript.
  All exited 0. The repair owner recorded the regression failing before the
  CSS fix (13 pass, 1 fail). Both source hashes matched the handoff.
- Native acceptance: actual Protection confirmation rendering in named
  palettes and forced colors remains part of the parent matrix.

### F2 — Rust lint: nested Installed-font condition

- File: `crates/devsweep-core/src/desktop_preferences/mod.rs:233-237`.
- Issue: the parent reported `clippy::collapsible_if` in the nested
  Installed-font validation.
- Fix: a let chain keeps the same Installed match and `valid_font_family`
  predicate. System still skips the family check; invalid Installed values
  still return `InvalidFontFamily` after the numeric checks. No predicate,
  error variant, limit, or return order changed.
- Independent evidence: replacing only the repaired block in memory with the
  original block reproduces the original frozen module SHA-256
  `0afbfab91996cc161853a9289f6b2c3360d2f15360471603c55ad0e7f43b96c4`.
  `rustfmt --check --edition 2024` exited 0. The parent owns the Clippy rerun;
  this review did not start a competing Cargo process.

### F3 — Rust test lint: cloned one-element expected slice

- File: `desktop/src-tauri/src/desktop_preferences.rs:147`.
- Issue: the parent Clippy log reports `clippy::cloned_ref_to_slice_refs`
  for `assert_eq!(events, [updated.clone()])`.
- Fix: `assert_eq!(events.as_slice(), std::slice::from_ref(&updated))` borrows
  the expected snapshot. Slice equality still requires exactly one event and
  complete snapshot equality. The later sequence and preference assertions
  at lines 148-150 remain unchanged. No production behavior changed.
- Independent evidence: the reviewer read the complete test path, matched the
  handoff SHA-256, and ran `rustfmt --check --edition 2024` with exit 0. The
  parent owns the final Clippy and Rust test reruns.

## Findings (not fixed)

No unresolved palette code defect was found in the reviewed source paths.
Full Rust verification and native acceptance remain incomplete. The core
test executable failure and one `0xc0000005` termination have no confirmed
cause at report time. Those parent-owned investigations were not duplicated.

## Source and test review

| Area | Result and evidence |
| --- | --- |
| Seven closed V2 IDs | PASS. `crates/devsweep-core/src/desktop_preferences/mod.rs:21-40` defines dark/light/system/Latte/Mocha/Codex/Claude. The frontend decoder at `desktop/src/api/contract.ts:170-191` enforces the same IDs, V2 fields, and snapshot envelope. `desktop/src/preferences/appearance.ts:5-19` exhausts the generated union and resolves System to legacy dark/light. |
| Fixtures and generation | PASS. All seven snapshots and theme patches are present in `desktop/src/api/fixtures/desktop-preferences-themes.json` and `desktop-preferences-patches.json`. `desktop/scripts/generate-types.mjs:306-327` ingests each snapshot and groups every patch sample by tag without dropping repeated theme variants. `desktop/src/api/types.gen.ts:505` contains the seven-ID union. `desktop/src-tauri/src/wire_parity.rs:45-86` checks exact Rust decode/encode parity. `desktop/src/preferences/palettes.test.tsx:21-29` checks catalogue/fixture completeness and rejects unknown IDs. |
| V1 and persisted integrity | PASS. `crates/devsweep-core/src/desktop_preferences/legacy.rs:4-10` keeps its independent three-theme enum. `store.rs:136-160` only falls back to V1 when V2 is absent; `store.rs:190-195` validates under the transaction lock before atomic replacement. `tests.rs:10-49` saves and reloads all seven themes, preserves V1 bytes, and checks the dark reset. `tests.rs:487-515` rejects each new ID in V1 and preserves bytes. |
| Committed state and failure | PASS. `desktop/src/preferences/SettingsPage.tsx:40-48` renders seven native controlled radios. Hover/focus handlers do not write. `store.ts:17-21,43-60,63-81` preserves snapshot order, subscribes before read, reloads visible windows, and applies only successful commits. Tests at `palettes.test.tsx:49-94` cover both locales, one checked radio, explicit writes, reset, and failed named-palette saves. `preferences.test.tsx:67-100,117-171` checks reload ordering, failure/retry, and reset boundaries. |
| Main, HUD, and System | PASS at source/test level. `PreferencesProvider.tsx:9-14` subscribes to OS color-scheme changes only for System. `appearance.ts:45-51` writes palette ID separately from the catalogue's light/dark scheme and keeps reduced motion independent. Main uses the provider at `desktop/src/App.tsx:79`; HUD uses it at `desktop/src/hud/main.tsx:43`. `palettes.test.tsx:31-47,96-142` covers all IDs, three scales, saved HUD snapshots, listener changes, and motion. `desktop/src-tauri/src/tray.rs:309-325` reloads/publishes preferences before showing HUD. Actual restart and native window behavior remain pending. |
| Semantic tokens and preview inheritance | PASS for the reviewed shared consumers; F1 is repaired. `appearance.css:4-25` defines the explicit shared aliases. All six resolved palettes define the same direct tokens. Native radio previews reuse the palette selectors through `data-palette-preview` at `SettingsPage.tsx:45-46`. Main `styles.css:1` and `hud/hud.css:1` load the same palette owner. |
| Portals, titlebar, risk, dialogs, and diagrams | Shared consumers use tokens at `styles.css:23-32,51-68,105-110,137-140,180-188,254-261`. Settings Select/Combobox portals inherit document tokens at `components/SettingsSelect.tsx:37-49` and `SettingsCombobox.tsx:51-63`. Mode stylesheet aliases preserve semantic surfaces. Analyze uses palette/canvas text and stronger focused strokes at `modes/analyze/styles.css:27-31`. Protection confirmation now consumes semantic colors after the F1 repair. |
| Non-color cues and forced colors | PASS at source/test level. Theme selection has native checked state and a visible check (`SettingsPage.tsx:43-47`, `styles.css:270-281`). Popup selection has check indicators and a distinct highlight outline (`SettingsSelect.tsx:42-44`, `SettingsCombobox.tsx:56-58`, `styles.css:260-261`). Risk uses explicit labels (`components/TargetTable.tsx:89-90`); errors keep visible messages and alert semantics. `appearance.css:276-314` follows all presets with matching root specificity. `styles.css:288-293` preserves popup/theme indicators. Analyze forced-color rules at `modes/analyze/styles.css:68-74` cover incomplete/aggregated/focused tiles and adjacent labels. Native rendering is pending. |
| Sampling, planets, and cleanup | No palette-owned behavior change found. Theme patches only assign the theme at core `mod.rs:179`; appearance reset only changes its three appearance fields at `mod.rs:193-197`. Planet scheduling dependencies at `stage/Planet.tsx:125` exclude theme, and its renderer/palette files have no working-tree diff. Status consumes only the selected cadence/row limit at `modes/status/StatusWorkbench.tsx:253-257,314-331`. Cleanup selection, digest, confirmation, execution, and reducer paths have no palette changes. HUD invoke permissions remain read-only at `desktop/src-tauri/src/lib.rs:71`. |
| Spec and source provenance | Relevant preference/component/type-safety specs already describe the seven-ID contract, preview reuse, valid CSS schemes, and accessibility overrides. `desktop/public/THIRD_PARTY_NOTICES.txt:3-15` pins Catppuccin revision `07d02aa110ef9eb7e7427afca5c73ba9cf7f8ebd`, states local semantic adaptations, and identifies Codex/Claude as DevSweep-authored inspired designs. The complete MIT text matches the saved upstream license. Imported base/mantle/crust-or-surface0/text/subtext1/blue-or-mauve values match `research/catppuccin-pinned.json`. No palette dependency was introduced. |

## Contrast and frozen-source audit

All 32 files in `evidence/production-freeze.json` matched their recorded SHA-256
values before repairs. The final audit matched 30 original hashes. The only
two changed manifest entries are the authorized core module and styles test.
The support stylesheet and Tauri test module are additional reviewed repair
files outside the original manifest. All four final hashes matched the repair
handoff:

| File | Final SHA-256 |
| --- | --- |
| `desktop/src/support/styles.css` | `43936f3e38cec3e1c9e1640bd1b7b0326f7b0f0650ce1753197cf7a46c8a0cae` |
| `desktop/src/styles.test.ts` | `7e6b32467e640baa619280ee346bdeebfb4059afdc526c43b0e1e04c7b3e2641` |
| `crates/devsweep-core/src/desktop_preferences/mod.rs` | `4c8527fc56ddfa8daf8ed8d0ed041306cb1538109840d25c3d8e7273fac7bb15` |
| `desktop/src-tauri/src/desktop_preferences.rs` | `c4c57540bf8f54ee5c39df54b54b9bf496e995219a2bdb7968a758049012e742` |

The reviewer executed the contrast script without an output flag. The command
wrote no files:

```text
cd desktop
mise exec node@22 -- node scripts/check-palette-contrast.mjs
```

Exit code: 0. The parsed report exactly matched `evidence/contrast.json`. The
report contains 55 required semantic tokens, six resolved palettes, 468 pairs,
and zero failures. The `appearance.css` SHA-256 is
`7207f02fe6bc209e1e2e19acf29e5c9aeb738f5150cc89256e0e84a2de3fc2e3`.

The reviewer also read the script's luminance conversion, alias resolution,
cycle checks, thresholds, and pair definitions (`check-palette-contrast.mjs:9-85`).
The supplied pairs cover shared body/table/stage surfaces, popup selections,
status surfaces, HUD/titlebar text, primary action labels, Analyze labels,
control borders, and focus surfaces. Measurements use declared opaque sRGB
values. After F1, the actual support-dialog consumer uses the measured
`text/raised`, `control-border/raised`, and `control-border/stage-canvas` pairs.
The new regression test requires each pair for every resolved palette. The
numerical report does not establish native rendering or full surface acceptance.

| Palette | Minimum measured text ratio | Minimum measured boundary ratio |
| --- | ---: | ---: |
| Dark | 4.605748 | 3.859832 |
| Light | 5.025914 | 3.665672 |
| Catppuccin Latte | 4.725754 | 3.734182 |
| Catppuccin Mocha | 5.429795 | 4.454259 |
| Codex | 6.624160 | 4.101427 |
| Claude | 4.932241 | 3.705719 |

## Verification

The reviewer inspected the final parent machine result and log at
`../09-26-desktop-settings-experience/evidence/desktop-web-check.json` and
`desktop-web-check.log`. The post-dialog-repair gate started at
`2026-09-28T02:43:38.9022633Z` and finished at
`2026-09-28T02:44:10.7545885Z`, exit 0.

- Frontend lint: PASS. Final parent gate ran `eslint .` (log line 4).
  The reviewer also independently ran scoped ESLint after the dialog repair.
- TypeCheck: PASS. Final parent gate ran `tsc --noEmit` (log line 6).
  The reviewer independently repeated desktop TypeScript after the repair.
- Generated types: PASS. Final parent generator `--check` passed (line 2).
- Frontend tests: PASS. Final full gate: 51 files / 384 Vitest tests, plus
  5 Node tests (lines 143-144 and 194-197). The reviewer independently ran the
  repaired styles suite: 14/14, exit 0, at 21:42:58 America/Chicago.
- Web build: PASS. Final parent production frontend build passed (line 222).
- Focused saved reports: independently parsed as 96/96 and 25/25 passing.
  The relevant source assertions were reviewed as listed above.
- Contrast recomputation: PASS. 468/468 declared pairs; no output-file mutation.
- Rust format: PASS for both repaired modules. The reviewer ran
  `rustfmt --check --edition 2024` separately on each, exit 0.
- Rust Clippy: the parent found and serialized the F2 and F3 repairs. Final
  post-repair Clippy execution is parent-owned and pending at report time.
- Desktop Rust tests: the reviewer inspected `desktop-test.json` and
  `desktop-test.log`. The earlier gate exited 0 at
  `2026-09-28T02:37:44.3864963Z`: 83 passed, 2 ignored (log line 99).
  This run predates the two mechanical Rust lint repairs; the parent owns the
  post-repair rerun.
- Core preference and wire-parity focused checks: implementation evidence
  records 14 and 1 passing tests. The assertions were reviewed. Earlier core
  diagnostic output also printed 14 preference cases as `ok`, before an
  incomplete full-core run. These observations do not establish a final pass.
- Canonical `just ci`: FAIL / blocked at core execution. The latest machine
  result records exit 1 at `2026-09-28T02:45:20.6858488Z`. Its log states the
  core test executable was `never executed` because of `os error 2`. The
  parent reports the final direct-Cargo scoped preference retry also never
  executed. An earlier direct-core run terminated with `0xc0000005`. The
  cause remains unknown. No duplicate Cargo diagnosis was started here.
- Native build and visual/keyboard/high-contrast acceptance: PENDING. No native
  screenshots, actual restart, hidden-HUD, OS scale, or native palette matrix
  were captured by this reviewer.

## Acceptance disposition

| Criterion | Code/test checkpoint | Remaining acceptance |
| --- | --- | --- |
| T-AC1 | PASS | Native preview/selection confirmation. |
| T-AC2 | PASS at source/test level after F1 repair | Native operational/support/dialog/error/HUD matrix. |
| T-AC3 | PASS | Native restart and hidden-HUD behavior. |
| T-AC4 | PASS for declared token measurements and reviewed cues | Verify native forced colors and keyboard focus, including the repaired support dialog. |
| T-AC5 | PASS at source/test level | Native bilingual layouts at 100/110/125 percent and reduced-motion review. |

Do not mark the child fully accepted or archive the task from this report.
The parent owns the remaining integrated gates and native acceptance.
