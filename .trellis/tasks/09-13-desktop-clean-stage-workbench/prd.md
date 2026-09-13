# Rebuild Clean stage workbench

## Goal

Replace the Clean ring hero and six-column table with a stage hero, capacity
plaque, `Found so far` list, category rows, and file rows on existing DTOs
and reducer phases. Depends on `09-13-desktop-sidebar-spec-shell` chrome,
tokens, and glyphs. Parent: `09-13-desktop-puremac-visual-refactor`.

## Requirements

- R1: Idle home is a stage card: eyebrow, headline, lede, Projects and
  Global caches checkboxes (default both on), Scan. A plaque card shows
  `Ready`. No ring hero. No hint sentence as the only content.
- R2: Scanning keeps the stage, changes the headline, shows backend phase
  and message with an indeterminate progressbar (no `aria-valuenow`, no
  percentage), Cancel scan, and a `Found so far` card built from Scan
  Preview scope groups. Heading semantics `Projects 1` / `Global caches 1`
  stay for existing tests.
- R3: Reviewed report renders a summary card (Estimated Recoverable with
  verified / partial / unknown kept separate, target count, `Selected
{count}`, single select-all, Review dry run) and category rows grouped by
  existing `kind` with a glyph tile, bilingual kind label, count, and
  subtotal. Each category row expands to file rows in a `<table>`: checkbox
  (review only), ecosystem glyph, path, muted `ecosystem · scope`, risk
  chip with bilingual label, capacity, evidence disclosure. Inspect Only
  stays unselectable by row, select-all, and reducer. Selection change and
  rescan invalidate dry-run. Execute needs digest plus confirm.
- R4: Reported outcome uses a large number and existing trash copy. Empty
  completed report shows the stage card with existing empty copy. Canceled
  preview stays read-only with existing copy.
- R5: Additive keys only: `clean.v1.stage.eyebrow`,
  `clean.v1.stage.headline`, `clean.v1.stage.lede`,
  `clean.v1.stage.scanning_headline`, `clean.v1.preview.found_so_far`,
  six `clean.v1.kind.*`, four `clean.v1.risk.*`. EN and zh-CN parallel.
  Existing forms byte-for-byte. Headline and lede must be original copy,
  not PureMac copy.
- R6: Remove `SweepBody size="hero"` from Clean pages. Reduced motion has
  no residual animation in Clean.

## Acceptance Criteria

- [ ] AC1 (R1, R2): ScanPage tests in EN and zh-CN find Scan / 扫描,
      Projects / 项目, Global caches / 全局缓存, Ready / 就绪, the stage
      headline, and a progressbar without `aria-valuenow` while scanning.
      `Cancel scan` is present while scanning.
- [ ] AC2 (R3): ReviewPage and App tests show category rows with bilingual
      kind labels, one `Select all executable targets` control, Inspect
      Only rows disabled, dry-run invalidation on selection change, and
      digest plus confirm before execute. No raw `build_artifacts` or
      `dangerous` text is visible.
- [ ] AC3 (R4): Result copy matches `clean.v1.trash.moved`. No text
      matches `space freed`, `released space`, or `4K`.
- [ ] AC4 (R5): Catalogue key sets are equal in EN and zh-CN. CLI i18n
      tests pass. TUI snapshots unchanged.
- [ ] AC5 (R6): No `sweep-body-hero` in Clean pages. Desktop lint,
      typecheck, test, and build pass. `just ci` passes.

## Out of Scope

- New scan providers, Docker cleanup, permanent delete, sidebar badges.
- Shell chrome and glyph module (child 1). Other modes (child 3).
- Count-up animation, shimmer, stacked meter animation beyond a width
  transition.

## Dependency

Wait for `09-13-desktop-sidebar-spec-shell` chrome, card tokens, and
glyphs on the branch. Reducer-neutral helpers may be prepared earlier.
