# Restyle other modes and support pages on card chrome

## Goal

Move Software, Optimize, Analyze, Status, Protection, Rules, and History
onto the sidebar chrome with page headers, card surfaces, and glyph tile
rows, then record bilingual width, reduced-motion, forced-colors, and
native Windows evidence for the whole desktop. Depends on
`09-13-desktop-sidebar-spec-shell`. Parent:
`09-13-desktop-puremac-visual-refactor`.

## Requirements

- R1: Software. Header status chip from `state.status`. Inventory rows as
  glyph tile rows with name, publisher or source, last-used, eligibility
  copy unchanged, selection only for eligible current-user MSIX. Sticky
  summary bar outside the list. Confirm dialog and five terminal outcomes
  unchanged.
- R2: Optimize. Catalogue rows as tile rows with action-class glyph and
  existing facts. Guidance rows keep `no_run`. Running checklist keeps
  existing outcome tags. Empty state is a card with existing copy and one
  action.
- R3: Analyze. Summary card, breadcrumbs, list plus treemap on cards.
  Read-only. No trash action. Render benchmark still builds.
- R4: Status. Cards become a bento of card surfaces with existing metrics.
  Charts keep polyline rendering and the text alternative. Process table on
  a card. Unavailable stays unavailable. No score. No GPU zero.
- R5: Protection, Rules, History. Page header from the shell, card
  surfaces, tables stay tables, dialogs unchanged.
- R6: Remove every `SweepBody size="hero"` use; delete the `size` prop from
  `SweepBody`. Empty states use a glyph tile, title, detail, and one
  action.
- R7: Evidence. Screenshots in EN and zh-CN at 390, 800, 1024, 1440 CSS
  pixels for every route; reduced motion; forced colors; keyboard walk;
  native release window at 1080x720 and 900x600 with the archived capture
  approach (`capture-native-five-mode.mjs` pattern, isolated
  `LOCALAPPDATA`, WebView2 remote debugging). Record under
  `evidence/` in this task and point to it from a new
  `docs/validation/desktop-sidebar-chrome.md`.

## Acceptance Criteria

- [ ] AC1 (R1, R2): Software and Optimize tests pass with existing role and
      name queries; eligibility and guidance copy unchanged; sticky bars
      remain `position: sticky` in their stylesheets.
- [ ] AC2 (R3, R4): Analyze and Status tests pass; no new IPC; no trash
      control in Analyze; Status has no score text.
- [ ] AC3 (R5, R6): Support view tests pass. `grep -r 'size="hero"'
    desktop/src` returns nothing. `SweepBody` has no `size` prop.
- [ ] AC4 (R7): Evidence directory has bilingual screenshots for all eight
      routes at four widths, reduced-motion and forced-colors captures,
      native captures at both window sizes, and a capture log with the
      binary hash. `docs/validation/desktop-sidebar-chrome.md` links them.
- [ ] AC5: Desktop lint, typecheck, test, and build pass. `just ci` passes.

## Out of Scope

- Software updates, startup items, leftover matching, health score, tray
  HUD, Analyze-from-treemap delete.
- Clean pages (child 2). Shell and glyph module (child 1).
- Native 100 / 125 / 150 / 200% scaling beyond the WebView2
  `--force-device-scale-factor` approach; OS display settings stay
  untouched.

## Dependency

Wait for `09-13-desktop-sidebar-spec-shell` chrome, tokens, and glyphs. May
overlap `09-13-desktop-clean-stage-workbench`; the final evidence run must
include child 2's Clean pages, so run evidence last.
