# Implement - Clean stage workbench

1. Confirm child 1 chrome, `.card` tokens, and `glyphs.tsx` are on the
   branch.
2. Catalogue: add 15 `clean.v1.*` keys in EN and zh-CN. Run desktop i18n
   tests and `cargo test -p devsweep-cli i18n`.
3. `labels.ts` with kind and risk lookups plus an exhaustiveness test over
   the generated unions.
4. ScanPage: stage card, scope, Scan / Cancel scan, indeterminate status,
   plaque. Remove the hero ring and hint-only layout.
5. ScanPreviewPage: `Found so far` card with existing scope headings.
6. ReviewPage and TargetTable: summary card, category rows, file rows,
   risk chip, capacity plaque with stacked meter.
7. ExecutePage and ConfirmDialog on card chrome; copy unchanged.
8. Clean CSS on card tokens; update `styles.test.ts` Clean locks.
9. Tests: ScanPage, ReviewPage, TargetTable, App Clean flows, labels, in
   EN and zh-CN. Fixture bridge walk-through.
10. Gates in `desktop/`: `lint`, `typecheck`, `test`, `build`. Root:
    `just ci`.

Rollback: restore Clean pages, components, CSS, tests, and drop the
additive `clean.v1.*` keys from both locales.
