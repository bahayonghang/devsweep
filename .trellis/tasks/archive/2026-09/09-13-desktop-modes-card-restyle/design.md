# Design - Other modes and support pages on card chrome

Follow parent `design.md` section 5.

## Owned files

- `desktop/src/modes/software/SoftwareWorkbench.tsx`, `styles.css`, test.
- `desktop/src/modes/optimize/OptimizeWorkbench.tsx`, `styles.css`, test.
- `desktop/src/modes/analyze/AnalyzePage.tsx`, `styles.css`, test,
  `render-benchmark.tsx`.
- `desktop/src/modes/status/StatusWorkbench.tsx`, `styles.css`, test.
- `desktop/src/support/*.tsx`, `styles.css`, `SupportViews.test.tsx`.
- `desktop/src/app-shell/AppShell.tsx` only to delete the `SweepBody`
  `size` prop after every caller is gone.
- `docs/validation/desktop-sidebar-chrome.md`; `evidence/` in this task.

Reducers, selectors, state modules, parity tests, fixtures, and IPC stay.

## Shared patterns

- Header chip: each workbench renders its status chip through the
  `PageHeaderSlot` from child 1. Chip text reuses existing `*.v1.state.*`
  or status keys. No new key unless no existing key fits.
- Tile row: `li.tile-row > span.glyph-tile + div.tile-row-text (strong
name, span.secondary facts) + div.tile-row-actions`. Selection stays a
  real checkbox with an accessible label built as today.
- Card: `.card` from child 1. Mode stylesheets keep their locked strings
  (`--surface-raised: var(--raised);`, `position: sticky`,
  `.X-mode .secondary-button { color: var(--text); background:
var(--canvas);`, `@media (max-width: 800px)`, `430px`,
  `animation: none !important`, `::backdrop`) or update the matching test
  in the same change.
- Empty state: `section.card.mode-empty > span.glyph-tile + h2 + p +
button`.

## Per mode

- Software: list becomes tile rows; eligibility label stays the
  checkbox `aria-label` suffix. Preview and results sections become cards.
  Summary bar keeps `.software-summary-bar` sticky.
- Optimize: `.optimize-row` becomes a tile row with a glyph by
  `action_class`. Catalogue header becomes the card header.
- Analyze: `.analyze-summary` card, controls card, list and treemap in a
  two-column card grid at 1024px and above, stacked below.
- Status: `.status-cards` becomes a grid of `.card` with a glyph tile per
  metric family. Charts card. Process table card.
- Support: `.support-page` drops its own `h1` if the shell header already
  names the page; keep `aria-labelledby` pointing at the shell heading or a
  visible section heading.

## Evidence protocol

- Web widths: Vite fixture build served locally, driven by the archived
  desktop driver pattern (`.trellis/tasks/archive/2026-09/08-29-five-mode-native-integration/evidence/resources/desktop-driver.mjs`).
  Capture EN and zh-CN for `#/clean`, `#/software`, `#/optimize`,
  `#/analyze`, `#/status`, `#/protection`, `#/rules`, `#/history` at
  390, 800, 1024, 1440. Emulate `prefers-reduced-motion: reduce` and
  `forced-colors: active` for one pass each.
- Native: release build `target/release/devsweep-desktop.exe`, isolated
  `LOCALAPPDATA`, `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` with remote
  debugging on loopback, window resized to 1080x720 and 900x600 through the
  CDP session. Log binary SHA-256 and every step to `capture-log.jsonl`.
- Report: `docs/validation/desktop-sidebar-chrome.md` follows the
  `five-mode-native.md` format: environment, evidence root, per-route table,
  UNVERIFIED rows where a capture was not possible.
