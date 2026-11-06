# Implementation verification ledger

## Authorization and baseline

The user approved the final plan on 2026-11-06: "按照规范和规划开始实施".
Baseline: dev at 63a4245; only this task tree was initially untracked.
The four child plans are approved; no cleanup or uninstall is authorized.

## Checks completed

- Root `just ci`: passed formatting, workspace check, all tests and clippy.
  First sandbox attempt failed in the installed mbx compiler wrapper with
  Access denied. The approved elevated retry completed with exit code 0.
  This is the shared-workspace baseline gate; GPUI is a standalone workspace
  and has separate checks. No root/core/Tauri product changes have been made.
- Pinned GPUI public font catalogue API verified from the dependency checkout;
  see the settings child's research/font-api.md.

## Intermediate native footer smoke

An isolated copy of the footer-stage release executable was launched from
dist/gpui-usability-qa with separate LOCALAPPDATA and APPDATA. Computer Use's
first app approval timed out; the user authorized a retry and access succeeded.
The native window was 1240x820 logical pixels. Observed no bottom strip or Note.
Startup Tab focused the DevSweep menu; Alt+Right switched Clean to Software;
UIA Settings activation opened Settings. This intermediate binary predates the
font/Clean/Software changes and is not final integration evidence.

## First integrated native pass (before review fixes)

Build SHA-256: 4FA1DEB798C6E05851A92CB82C52C8538D6364ED4C7F83E8D40EC769A17AF2EC.
This binary predates the final Software synthetic-selection fix and independent
query/input/save-retry corrections. Do not treat it as final regression evidence.

- Clean immediately shows Project/Global scope and Choose folder. Global scope
  selection did not start work; clicking Scan global caches opened its separate
  Back/Scan confirmation. No Global scan was executed.
- Settings enumerated real installed families (11 pages); normal key input
  filtered the list. Selecting Cascadia Code visibly changed headings, navigation
  and the search text. Preview left the V2 file absent; Save wrote the installed
  family. Screenshots: evidence/settings-font-preview.png and settings-about.png.
- About displayed GPUI build 0.1.0, MIT and the canonical repository. Clicking
  Project repository opened a browser titled bahayonghang/devsweep.
- First Software entry showed read-only inventory progress and then real rows,
  one segment strip, toolbar and qualified sizes. No host operations executed.
  Host inventory screenshots are not saved into repository evidence; subsequent
  visual captures use explicitly labelled synthetic, manual-only display rows.
- Native defect found: ordinary key input worked, but pasted text and UIA
  SetValue did not update TextInput. Independent checker owns the fix and final
  native rerun. Shared cross-mode query leakage was separately found by review.
- A corner drag did not resize the window; no narrow-size claim from that
  attempt. Native view was the default 1240x820 logical window.

## Final integration checks

- Independent review fixed query ownership across modes, TextInput UIA SetValue
  and Ctrl+A/Ctrl+V, and partial-save retry rebase. See research/review.md.
- Final `just gpui-check`: passed format and all-target Clippy with warnings denied.
- Final `just gpui-test`: 178 passed, 0 failed.
- Final `just gpui-build`: release build passed.
- Final binary SHA-256:
  B2E796DE0B7FD8474F837037BF664EEBA7C65E651BD7CA72B7DB57D2EF5C84FB.

## Final native pass

The final binary ran with isolated LOCALAPPDATA/APPDATA in dist/gpui-usability-qa.
Screenshots were 1242x821 including the border (1240x820 logical client default).
No cleanup, uninstall or optimization was executed.

- Saved Cascadia Code restored after restart. Font catalogue searched actual
  installed names; Microsoft YaHei UI selection updated the full UI and input.
  Save persisted that family, light theme and 125% text scale in V2 preferences.
- UIA SetValue accepted Chinese text. Ctrl+A followed by pasted text replaced
  the query. Alt+Right reached Optimize; returning to Software preserved its
  query without filtering Optimize. Clean and Software queries were independent.
- Clean opened with the explicit saved test root and did not scan automatically.
  Clicking Scan projects found 12 generated node_modules fixtures. Search reduced
  this to fixture-project-03. List and preview controls preceded statistics/map.
  The folder picker opened and Escape cancelled it; picker selection itself was
  not completed because the automation could not target its child field reliably.
- Software loaded 60 clearly named manual-only fixture rows. First test file
  contained an unsupported launch field; corrected fixture loaded after restart.
  Filtered-empty state showed 0/60 with recovery guidance. Settings Escape return
  retained the query. No fake execution capability was granted to fixtures.
- English/dark/100% and Chinese/light/125% were rendered. The compact icon mode
  header and Software selector/toolbar remained visible at 125%. About remained
  reachable by scrolling, with correct 0.1.0 GPUI version, MIT and project URL.
- Final screenshots: evidence/final-clean-results.jpg,
  final-software-list.jpg, final-software-empty.jpg, final-font-catalogue.jpg,
  final-settings-zh-125-light.jpg, final-about-zh-125-light.jpg,
  final-software-zh-125-light.jpg.

## Evidence limits (not completion blockers)

On 2026-11-06 the user explicitly stopped peripheral testing: only test changed
core functionality; do not spend time on window maximize/resize and other
unrelated matrices. The following items document coverage limits, not required
follow-up work. Do not resume them without a concrete related failure or request.

- Exact 960x720, 1280x800 and 1536x1024 matrix remains unverified. Two edge drags
  did not resize the Windows custom-caption window; do not substitute default
  viewport screenshots for these assertions. No product-only QA override added.
- Real IME candidate interaction and OS high contrast remain unverified. Unicode
  composition buffer tests and palette unit tests are not native proof.
- Folder selection no-implicit-scan and scope invalidation have regression tests;
  native picker success still needs completion. Native unavailable/updates/startup
  edge-state matrix is not exhaustive; source and state tests cover those paths.
- Changed core functions and directly affected regressions have passed. Product
  changes were committed as 4fc8a64; the user authorized task archive on 2026-11-06.
  Do not delay completion for the peripheral checks above.
