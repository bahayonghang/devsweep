# Implement - Sidebar workbench visual language

## Order

Do not start this parent. After the user approves the planning summary:

1. Start `09-13-desktop-sidebar-spec-shell`. Spec files first, then tokens,
   glyphs, catalogue keys with all three freezes, `AppShell`, page header,
   Tauri window. Gate: desktop shell tests, catalogue parity, CLI i18n
   test, no TUI snapshot drift.
2. Start `09-13-desktop-clean-stage-workbench` after child 1 chrome and
   glyphs are on the branch.
3. Start `09-13-desktop-modes-card-restyle` after child 1. It may overlap
   child 2. It owns the other modes, support pages, and the bilingual
   width, reduced-motion, forced-colors, and native evidence.

## Validation

Child-local, in `desktop/`:

```powershell
mise exec node@22 -- npm run types:generate
mise exec node@22 -- npm run lint
mise exec node@22 -- npm run typecheck
mise exec node@22 -- npm run test
mise exec node@22 -- npm run build
```

After catalogue or Tauri edits, at repository root:

```powershell
cargo test -p devsweep-cli i18n
just ci
```

Evidence: EN and zh-CN at 390, 800, 1024, 1440 CSS pixels; reduced motion;
forced colors; keyboard; native window at 1080x720 and 900x600. Native
100/125/150/200% scaling stays user-operated.

## Risky files

| File                                                                                    | Risk                                    |
| --------------------------------------------------------------------------------------- | --------------------------------------- |
| `.trellis/spec/desktop-frontend/component-guidelines.md`                                | Visual contract; must land before UI    |
| `desktop/src/app-shell/AppShell.tsx`                                                    | Routing, focus, accelerator regressions |
| `desktop/src/app-shell/AppShell.test.tsx`, `desktop/src/App.test.tsx`                   | `More`, `.shell-more`, tab queries      |
| `desktop/src/styles.css`, `desktop/src/styles.test.ts`                                  | Locked selector strings                 |
| `resources/i18n/en.json`, `zh-CN.json`                                                  | TUI coupling, parity                    |
| `desktop/src/i18n/index.ts`, `index.test.ts`, `crates/devsweep-cli/src/i18n/mod.rs:760` | Frozen 23-key shell namespace           |
| `desktop/src-tauri/tauri.conf.json`                                                     | Window size; CSP must stay              |
| `desktop/src/state/app-state.ts`                                                        | Must not change authority               |
| `desktop/src/pages/ScanPage.tsx`, `ReviewPage.tsx`, `ScanPreviewPage.tsx`               | Clean first screens                     |
| `desktop/src/components/TargetTable.tsx`                                                | Selection rules and Inspect Only        |

## Rollback

`git checkout --` on the child's files. Do not mutate existing catalogue
forms to roll back UI; drop additive keys and restore the freezes instead.

## Follow-up

Sidebar size badges, Smart Care style aggregate categories, and onboarding
may be a later parent. Do not add them to these children.
