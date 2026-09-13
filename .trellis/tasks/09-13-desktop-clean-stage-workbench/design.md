# Design - Clean stage workbench

Follow parent `design.md` section 4.

## Owned files

- `desktop/src/pages/ScanPage.tsx`, `ScanPage.test.tsx`,
  `ScanPreviewPage.tsx`, `ReviewPage.tsx`, `ReviewPage.test.tsx`,
  `ExecutePage.tsx`.
- `desktop/src/components/TargetTable.tsx`, `TargetTable.test.tsx`,
  `ConfirmDialog.tsx`, `format.ts`.
- `desktop/src/modes/clean/CleanWorkbench.tsx`; new
  `desktop/src/modes/clean/labels.ts` for kind and risk label lookup.
- Clean rules in `desktop/src/styles.css` (or a new
  `desktop/src/modes/clean/styles.css` imported by `CleanWorkbench`, which
  matches the other modes).
- `desktop/src/App.test.tsx` Clean assertions.
- `resources/i18n/en.json`, `zh-CN.json` additive `clean.v1.*` keys.

Do not change `desktop/src/state/app-state.ts` or
`desktop/src/modes/clean/reducer.ts` authority transitions. Map existing
phases to views.

## Composition

```text
CleanWorkbench (.clean-mode)
  ErrorBanner
  phase idle | scanning | empty report
    section.card.clean-stage
      p.stage-eyebrow      h2.stage-headline      p.stage-lede
      div.scope-controls (checkboxes)   button.primary-button Scan | Cancel scan
      div.scan-status (progressbar indeterminate, phase, message)
    section.card.capacity-plaque
      Ready | Scan Progress phase | Estimated Recoverable
      (reviewed only) stacked meter: verified, partial lower bound
    scanning only: section.card.found-so-far (ScanPreviewPage groups)
  phase reviewed | dry_run | confirming | executing | reported
    section.card.review-summary
      Estimated Recoverable, count, selection strip, Review dry run
    section.preview-group.card  (one per kind)
      header: glyph tile, kind label (h3), count, subtotal
      TargetTable rows (file rows)
    ExecutePage card, ConfirmDialog
```

- `ReviewPage` grouping stays a presentation fold over `target.kind` then
  `target.scope.type`. Keep `.preview-group h3` and `.preview-group header
p` so `ReviewPage.test.tsx` selectors survive, or update them in the
  same change.
- `TargetTable` keeps `<table>`, the review select-all header cell, and
  the `Select all executable targets` label. Cells: selection, primary
  (ecosystem glyph, path strong, muted `ecosystem · scope`), risk chip,
  capacity, status, evidence disclosure. The `Category` column moves to
  the group header.
- `labels.ts`: `kindLabel(locale, kind)` and `riskLabel(locale, risk)` map
  the generated unions to `clean.v1.kind.*` and `clean.v1.risk.*`. A test
  asserts every union member has a key in both locales.
- Capacity plaque: verified total, partial lower bound, unknown count.
  Stacked meter draws verified and partial only. Unknown is text.

## Copy (additive keys)

| Key                                  | EN                                                                                           | zh-CN                                                           |
| ------------------------------------ | -------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| `clean.v1.stage.eyebrow`             | Local scan                                                                                   | 本机扫描                                                        |
| `clean.v1.stage.headline`            | Review every target before anything moves.                                                   | 先逐项审核，再移动任何文件。                                    |
| `clean.v1.stage.lede`                | DevSweep builds a Scan Report you can inspect. Nothing leaves until you preview and confirm. | DevSweep 生成可检查的扫描报告。预览并确认之前不会移动任何内容。 |
| `clean.v1.stage.scanning_headline`   | Scanning. Targets appear as they are found.                                                  | 扫描中。发现的目标会即时列出。                                  |
| `clean.v1.preview.found_so_far`      | Found so far                                                                                 | 目前已发现                                                      |
| `clean.v1.kind.build_artifacts`      | Build artifacts                                                                              | 构建产物                                                        |
| `clean.v1.kind.dependency_directory` | Dependency directories                                                                       | 依赖目录                                                        |
| `clean.v1.kind.package_cache`        | Package caches                                                                               | 包缓存                                                          |
| `clean.v1.kind.test_cache`           | Test caches                                                                                  | 测试缓存                                                        |
| `clean.v1.kind.tool_cache`           | Tool caches                                                                                  | 工具缓存                                                        |
| `clean.v1.kind.virtual_env`          | Virtual environments                                                                         | 虚拟环境                                                        |
| `clean.v1.risk.low`                  | Low                                                                                          | 低                                                              |
| `clean.v1.risk.medium`               | Medium                                                                                       | 中                                                              |
| `clean.v1.risk.high`                 | High                                                                                         | 高                                                              |
| `clean.v1.risk.dangerous`            | Dangerous                                                                                    | 危险                                                            |

Final wording is the implementer's choice inside these constraints: no
PureMac copy, no "space freed", no percentage, trash-only language.

## CSS

- `.clean-stage`, `.capacity-plaque`, `.found-so-far`, `.review-summary`,
  `.preview-group` use `.card` tokens from child 1.
- `.stage-headline { font-size: clamp(1.6rem, 3vw, 2.25rem); }`; no
  viewport scaling of body text.
- `.capacity-total .display-capacity { font-size: clamp(2rem, 6vw, 3.5rem);`
  is locked by `styles.test.ts:42`; keep or update the lock in the same
  change.
- Stacked meter: render as an inline SVG with `<rect>` widths computed in
  JSX as attributes, the same technique as the Status polyline chart.
  No inline `style` attribute (the desktop tree has none today and CSP
  is `style-src 'self'`). A `<p>` text alternative lists verified,
  partial lower bound, and unknown counts.

## Compatibility

- Fixture bridge (`npm run dev:fixture`) must still walk idle, scanning,
  reviewed, dry run, confirm, executed with the new views.
- `App.test.tsx` queries for `Projects`, `Scan`, `Search`, `Ready`,
  `Projects 1`, `Review dry run`, and `Select node.node_modules` stay valid.
