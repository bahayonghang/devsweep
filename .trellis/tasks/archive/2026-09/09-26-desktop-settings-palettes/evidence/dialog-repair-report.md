# Support Dialog Palette Repair

Date: 2026-09-27 (America/Chicago). Task: 09-26-desktop-settings-palettes.
Status: scoped repair and automated checks complete; handed back for independent review and native validation.

## Finding and correction

`desktop/src/support/styles.css:18` set `CanvasText` and `Canvas` on
`.support-page dialog` outside a forced-colors media query. The selector
overrode the shared `dialog` palette rule. `ProtectionPage` renders both add
and remove confirmations as a `dialog` inside `.support-page`, so both
confirmation paths used system colors in normal rendering.

The same selector now uses `--text`, `--raised`, and `--control-border`.
These tokens follow the committed root palette. The existing
`@media (forced-colors: active)` rule remains unchanged at lines 26-33.
The forced-colors selector has the same specificity and appears later.
Its `CanvasText`, `Canvas`, border-color, and `forced-color-adjust: auto`
declarations therefore continue to override the normal dialog colors.

## Regression coverage

`desktop/src/styles.test.ts` now reads the support stylesheet. The regression
requires semantic colors in the normal dialog rule and rejects `Canvas` and
`CanvasText` in that rule. The regression also requires the later
forced-colors dialog override and its system-color declarations.

The existing contrast-report assertions now explicitly require these actual
dialog pairs for every resolved palette:

- Text against the dialog surface: `text` / `raised`, minimum 4.5:1.
- Border against the dialog surface: `control-border` / `raised`, minimum 3:1.
- Border against the page canvas: `control-border` / `stage-canvas`, minimum 3:1.

The contrast script already measures all three pairs. The repair does not
change the script or duplicate palette constants. System continues to resolve
to the existing dark or light palette.

| Resolved palette | Text / raised | Border / raised | Border / stage canvas |
| --- | ---: | ---: | ---: |
| dark | 14.2562 | 4.4663 | 4.9613 |
| light | 15.8222 | 4.4660 | 4.0944 |
| catppuccin_latte | 6.5660 | 4.0626 | 4.3695 |
| catppuccin_mocha | 12.1390 | 6.2196 | 5.8108 |
| codex | 14.3834 | 4.6643 | 5.1434 |
| claude | 11.3845 | 4.0180 | 4.4306 |

Ratios use the existing opaque sRGB calculation. Values above are rounded
for display. The tests evaluate unrounded values.

## Checks

Commands ran from `desktop/`, except the root Git diff check.

| Command and state | Exit | Result |
| --- | ---: | --- |
| Before CSS fix: `mise exec node@22 -- npx vitest run src/styles.test.ts --maxWorkers=1` | 1 | Expected regression failure: 13 passed, 1 failed. The new dialog test received `color: CanvasText` instead of `color: var(--text)`. |
| After CSS fix: `mise exec node@22 -- npx vitest run src/styles.test.ts --maxWorkers=1` | 0 | 14 passed. Includes all existing palette contrast and forced-colors assertions. |
| `mise exec node@22 -- npx eslint src/styles.test.ts` | 0 | Scoped lint passed. |
| `mise exec node@22 -- npm run typecheck` | 0 | Desktop TypeScript check passed. |
| Root: `git diff --check -- desktop/src/support/styles.css desktop/src/styles.test.ts` | 0 | No whitespace errors. Git reported its LF-to-CRLF checkout warning. |

The contrast values were also read from
`mise exec node@22 -- node scripts/check-palette-contrast.mjs`; the read-only
command completed with exit 0. No broad gate ran for this repair.

## Source hashes and handoff

SHA-256 of the checked source bytes, recorded at 2026-09-27 21:38 America/Chicago:

| File | SHA-256 |
| --- | --- |
| `desktop/src/support/styles.css` | `43936f3e38cec3e1c9e1640bd1b7b0326f7b0f0650ce1753197cf7a46c8a0cae` |
| `desktop/src/styles.test.ts` | `7e6b32467e640baa619280ee346bdeebfb4059afdc526c43b0e1e04c7b3e2641` |
| `desktop/src/preferences/appearance.css` (read only) | `7207f02fe6bc209e1e2e19acf29e5c9aeb738f5150cc89256e0e84a2de3fc2e3` |

Changed files: `desktop/src/support/styles.css`, `desktop/src/styles.test.ts`,
and this report. Existing edits in the test file remain intact. No other
production files, dependencies, task metadata, or parent scripts changed.

Automated source and contrast checks establish the declared CSS behavior.
The parent owns independent review, the native build, and actual support-dialog
rendering in named palettes and forced colors. Those native checks were not
performed by this repair. T-AC2 and T-AC4 native acceptance remains open.

## Authorized Rust lint scope extension

Date: 2026-09-27 21:43 (America/Chicago). The parent authorized one additional
change after reporting that `just clippy` exited 1 for `collapsible_if` at
`crates/devsweep-core/src/desktop_preferences/mod.rs:233`.

The nested Installed-font condition is now a let chain. The exact source
change is:

```diff
-        if let DesktopFont::Installed { family } = &self.font {
-            if !valid_font_family(family) {
-                return Err(DesktopPreferencesError::InvalidFontFamily);
-            }
+        if let DesktopFont::Installed { family } = &self.font
+            && !valid_font_family(family)
+        {
+            return Err(DesktopPreferencesError::InvalidFontFamily);
         }
```

The condition still calls `valid_font_family` only for Installed. System
continues to skip the family check. Invalid Installed values return the same
`InvalidFontFamily` error after the existing numeric checks; valid values
continue to `Ok(self)`. The validation helper and all validation limits are
unchanged. No lint suppression was added.

Before the edit, the backend spec index, quality requirements, and the
Installed-family preference contract were read. The repository uses Rust
edition 2024.

| Check | Result |
| --- | --- |
| `rustfmt --check --edition 2024 crates/devsweep-core/src/desktop_preferences/mod.rs` from the repository root | Exit 0; no formatting output or source write. |
| Full module before/after comparison, with CRLF normalized for comparison only | PASS; replacing exactly the displayed condition block reproduces the entire resulting source. |
| Source review of Installed/System and valid/invalid cases | Same predicate, short-circuit order, error variant, and return order. |

Module SHA-256 after the edit:
`4c8527fc56ddfa8daf8ed8d0ed041306cb1538109840d25c3d8e7273fac7bb15`.

Only the module and this report changed in the extension. CSS and TypeScript
were not changed again. Cargo compilation, Cargo tests, and Clippy were not
run by the implementer, as requested by the parent. The parent owns the
integrated Rust checks. Source writes are complete.

## Authorized Tauri test lint scope extension

Date: 2026-09-27 21:47 (America/Chicago). The parent authorized the additional
`cloned_ref_to_slice_refs` finding at
`desktop/src-tauri/src/desktop_preferences.rs:147`.

The `updated` snapshot is used after the event assertion to check sequence 2
and the reloaded preference value. The assertion now borrows the snapshot
through a one-element slice:

```diff
-        assert_eq!(events, [updated.clone()]);
+        assert_eq!(events.as_slice(), std::slice::from_ref(&updated));
```

Slice equality retains the exact event count of one and full snapshot equality.
The subsequent sequence and preference assertions remain unchanged. No lint
suppression or production behavior change was added. The Tauri coordinator
ordering and event requirements in the backend desktop-preference spec were
read before the edit.

| Check | Result |
| --- | --- |
| `rustfmt --check --edition 2024 desktop/src-tauri/src/desktop_preferences.rs` from the repository root | Exit 0; no formatting output or source write. |
| Full module before/after comparison, with CRLF normalized for comparison only | PASS; exactly the displayed assertion line changed. |
| Source review | Event count/content and all later uses of `updated` remain covered. |

Module SHA-256 after the edit:
`c4c57540bf8f54ee5c39df54b54b9bf496e995219a2bdb7968a758049012e742`.

Only this test assertion and this report changed in the extension. No Cargo
command ran. The parent owns Clippy, Rust tests, and the native build. Source
writes are complete.
