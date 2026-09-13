# Design - Sidebar workbench visual language

## 1. Boundary

`ref/repo/PureMac` is reference only. Implementation starts from current
DevSweep contracts in `desktop/src/` and `.trellis/spec/desktop-frontend/`.
No PureMac source, string, screenshot, SF Symbol, or hex value enters
`desktop/`, `resources/`, or tests.

TUI stays on the current chrome. Spec edits stay under
`.trellis/spec/desktop-frontend/`. Do not change `.trellis/spec/frontend/`.
Reducers, coordinator, IPC types, and fixtures stay.

## 2. Spec rewrite (child 1, first)

`component-guidelines.md` Product Mode and Composition currently require a
centered capsule and a More disclosure (`:3-31`). Replace with:

- Persistent sidebar shell. Sections `Modes` (five tabs, `role=tablist`,
  vertical orientation) and `Supporting destinations` (Protection, Rules,
  History as links). Footer holds Language and Help. Below 800 CSS pixels
  the sidebar becomes a horizontal top strip with the same names.
- Page header on every route: visible `h1` title, subtitle, optional status
  chip supplied by the mode. The `sr-only` heading rule is removed.
- Card surface: raised token, 1px hairline border, radius 12, padding 16.
  Controls, inputs, badges, and rows stay at radius 8 or below. Primary
  actions keep the pill.
- Tinted icon tile: 24px, radius 6, tint at 16% opacity behind an original
  inline SVG glyph, `aria-hidden`. Glyphs come from one DevSweep glyph module.
  No icon font, no SF Symbols, no image sprite.
- Status chip: pill, 11px semibold, dot plus text, semantic tint only.
- Stacked meter: solid segments with 2px separators, no animation beyond
  width transition, still under reduced motion. Segments are verified and
  partial lower bound only. Unknown is labelled, never drawn.
- Window: default 1080x720, minimum 900x600, title `DevSweep`.

Visual System (`:84-113`) keeps: Segoe UI Variable, tabular numerals,
dark-only canvas, mineral/forest tokens, visible focus, forced colors,
reduced motion, and the prohibition on glass, glow, planet heroes,
`linear-gradient`, `radial-gradient`, `backdrop-filter`, and "space freed".
The sweep body clause shrinks to: compact brand mark in the sidebar only; no
hero instance.

`index.md` completion checklist replaces "compact CSS sweep body" and
"capsule" wording with sidebar, page header, and card wording.

`state-management.md` invariants stay. Add one line: the shell never
derives counts, sizes, or status for sidebar rows.

## 3. Shell composition (child 1)

Keep `AppShell` as the route, focus, and coordinator owner
(`desktop/src/app-shell/AppShell.tsx`). Change presentation only.

```text
+------------------+--------------------------------------------------+
| (mark) DevSweep  | Clean                              [Ready]       |
|                  | Estimated recoverable stays an estimate until... |
| MODES            |--------------------------------------------------|
| [tile] Clean     |                                                  |
| [tile] Software  |   mode canvas                                    |
| [tile] Optimize  |                                                  |
| [tile] Analyze   |                                                  |
| [tile] Status    |                                                  |
|                  |                                                  |
| SUPPORTING       |                                                  |
| [tile] Protection|                                                  |
| [tile] Rules     |                                                  |
| [tile] History   |                                                  |
|                  |                                                  |
| Language  Help   |                                                  |
+------------------+--------------------------------------------------+
```

- DOM: `div.app-shell[data-mode] > aside.shell-sidebar + main#mode-workbench`.
  Sidebar: `div.shell-brand (SweepBody compact + DevSweep)`,
  `nav.mode-navigation[role=tablist][aria-orientation=vertical]` with
  `button[role=tab]`, `nav.support-navigation[aria-label=shell.v1.supporting]`
  with buttons, `div.shell-footer` with the Language button and Help link.
- Keyboard: ArrowDown/ArrowUp replace ArrowRight/ArrowLeft as the primary
  pair; both pairs stay accepted. Home, End, and Alt accelerators stay.
- Header: `header.page-header > div > h1#... + p.page-subtitle` plus a
  `div.page-header-slot` for a mode status chip. Title uses existing
  `command.*` and `shell.v1.supporting.*` keys. Subtitles are additive
  `shell.v1.subtitle.clean|software|optimize|analyze|status|protection|rules|history|settings`.
- `shell.v1.more` stays in the catalogue and the frozen list. It is no longer
  rendered. Removing a key is a catalogue change and is out of scope.
- `data-mode` on `.app-shell` still selects canvas and accent tokens.
- Deep links `#/clean` to `#/history` and `#/settings` stay.
- Tests: replace `More`, `.shell-more`, `.mode-capsule`, and
  `.sweep-body-hero` locks with `.shell-sidebar`, `.page-header`,
  `aria-orientation="vertical"`, and `.card` locks in the same change.

Tauri: `tauri.conf.json` window `width 1080`, `height 720`, `minWidth 900`,
`minHeight 600`, `title DevSweep`. No plugin, no capability change.

## 4. Clean flow (child 2)

Reducer and IPC stay in `desktop/src/state/app-state.ts` and
`desktop/src/modes/clean/`. Pages change composition:

| Phase                            | First screen                                                                                                                        |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| idle, no Scan Report             | Stage card: eyebrow `Local scan`, headline, lede, scope checkboxes, Scan; plaque card with `Ready`                                  |
| scanning                         | Same stage; headline changes; backend phase and message; indeterminate progress; Cancel scan; `Found so far` card from Scan Preview |
| reviewed                         | Summary card (Estimated Recoverable, count, selection strip, Review dry run); category rows by existing `kind`; file rows per row   |
| dry_run / confirming / executing | Existing ExecutePage and ConfirmDialog on card chrome                                                                               |
| reported                         | Result card: large truthful number, existing trash copy, return to review                                                           |
| empty completed report           | Stage card with existing empty copy and Scan                                                                                        |
| canceled                         | Existing canceled copy; preview stays read-only                                                                                     |

Category row: tile glyph by `kind`, bilingual kind label, count, subtotal
with confidence, disclosure. File row: checkbox (review only), ecosystem
glyph, path name, muted `ecosystem · scope`, risk chip, capacity, evidence
disclosure. `<table>` semantics stay for the file rows so
`TargetTable.test.tsx` and `App.test.tsx` selectors survive.

Capacity plaque and summary keep verified / partial lower bound / unknown
separate. Never sum them into one precise total. Scan Progress stays
indeterminate.

Copy: reuse `clean.v1.action.scan`, `clean.v1.scope.*`,
`clean.v1.status.ready`, `clean.v1.preview.estimated`,
`clean.v1.summary.selected`, `clean.v1.review.empty_*`,
`clean.v1.trash.moved`. Additive keys: `clean.v1.stage.eyebrow`,
`clean.v1.stage.headline`, `clean.v1.stage.lede`,
`clean.v1.stage.scanning_headline`, `clean.v1.preview.found_so_far`,
`clean.v1.kind.build_artifacts|dependency_directory|package_cache|test_cache|tool_cache|virtual_env`,
`clean.v1.risk.low|medium|high|dangerous`. Headline copy must not repeat
PureMac copy. Working English: `Scan first. Decide what leaves.` is too
close; use `Review every target before anything moves.` or a shorter
original line chosen in the child.

## 5. Other modes and support pages (child 3)

No new IPC. Restyle on the shared chrome:

- Software: header chip from `state.status`; inventory rows as tile rows
  with eligibility copy unchanged; sticky summary bar stays outside the
  list; confirm dialog unchanged.
- Optimize: catalogue rows as tile rows with action class glyph; running
  checklist keeps existing outcome tags; guidance rows keep `no_run`.
- Analyze: summary card, breadcrumbs, list plus treemap on cards. Read-only.
  No trash action.
- Status: cards become a bento of card surfaces; charts keep polyline
  rendering; process table on a card. Unavailable stays unavailable. No
  score. No GPU zero.
- Protection, Rules, History: `support-page` gets the page header and card
  surfaces; tables stay tables; dialogs unchanged.
- Empty states: glyph tile, title, detail, one action. No hero ring.

## 6. Tokens and glyphs (child 1 freezes)

| Token                                                                | Role                                         |
| -------------------------------------------------------------------- | -------------------------------------------- |
| `--canvas-*`, `--accent-*`                                           | Keep current mineral/forest values           |
| `--sidebar`                                                          | Sidebar surface, slightly raised from canvas |
| `--card`                                                             | Card surface                                 |
| `--card-border`                                                      | Hairline                                     |
| `--tile-alpha`                                                       | 16% tint behind glyphs                       |
| `--radius-card: 12px`, `--radius-control: 8px`, `--radius-tile: 6px` | Radii                                        |
| `--text`, `--muted`, `--focus`, `--danger`, `--warning`, `--ok`      | Unchanged                                    |

Glyph module `desktop/src/app-shell/glyphs.tsx`: original 16x16 stroke
paths for clean, software, optimize, analyze, status, protection, rules,
history, language, help, kind glyphs, ecosystem glyphs, risk dot. Each
export is a React component with `aria-hidden`. No outline traced from SF
Symbols or PureMac assets.

## 7. Compatibility

- Coordinator, fixtures, generated IPC types stay.
- TUI snapshots must not change. Existing catalogue forms are frozen.
- Three `shell.v1.*` freezes change together.
- `just ci` after catalogue and Tauri edits.

## 8. Rollback

Each child rolls back its files. If the spec rewrite is reverted, UI
children must not ship. Do not leave a sidebar shell on the old hero pages
or the old capsule on new card pages.
