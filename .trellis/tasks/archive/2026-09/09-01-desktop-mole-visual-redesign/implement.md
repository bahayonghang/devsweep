# Implement - Immersive desktop visual language

## Order

Do not start this parent. After the user approves the planning summary:

1. Start `desktop-immersive-spec-shell`. Spec files first, then tokens and
   `AppShell`. Gate: desktop shell tests, catalogue parity, no TUI snapshot
   drift.
2. Start `desktop-clean-hero-workbench` only after child 1 is in_progress
   with the spec rewrite committed or at least present on the branch.
3. Start `desktop-modes-immersive-restyle` after child 1 shell tokens exist.
   It may overlap child 2. It owns Software/Optimize/Analyze/Status restyle
   plus bilingual width/reduced-motion/native evidence.

## Validation

Child-local:

```powershell
mise exec node@22 -- npm run lint
mise exec node@22 -- npm run typecheck
mise exec node@22 -- npm run test
mise exec node@22 -- npm run build
```

in `desktop/`. After catalogue edits also run CLI i18n/TUI tests and
`just ci`.

Evidence: EN and zh-CN at 390, 800, 1024, 1440 CSS pixels; reduced motion;
forced colors; keyboard. Native 100/125/150/200% remains user-operated.

## Risky files

| File | Risk |
| --- | --- |
| `.trellis/spec/desktop-frontend/component-guidelines.md` | Visual contract; must land before UI |
| `desktop/src/app-shell/AppShell.tsx` | Routing/focus regressions |
| `desktop/src/styles.css` | Global light-pane leftover |
| `resources/i18n/en.json`, `zh-CN.json` | TUI coupling |
| `desktop/src/i18n/index.ts` / `index.test.ts` | Frozen 22-key shell namespace |
| `desktop/src/state/app-state.ts` | Must not change authority |
| `desktop/src/pages/ReviewPage.tsx` | Empty-state replacement |

## Rollback

`git checkout --` on the child's files. Do not leave a capsule nav on the
old light workbench. Do not mutate existing catalogue forms to roll back
UI; drop additive keys instead.

## Follow-up

A later parent may take Software updates, startup items, leftover matching,
tray HUD, and health score. Do not sneak them into these children.
