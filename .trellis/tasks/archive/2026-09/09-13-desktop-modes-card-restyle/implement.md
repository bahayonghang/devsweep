# Implement - Other modes and support pages on card chrome

1. Confirm child 1 chrome, `.card`, `PageHeaderSlot`, and `glyphs.tsx`
   are on the branch.
2. Software: header chip, tile rows, cards, sticky bar. Update test and
   stylesheet locks together.
3. Optimize: tile rows, card header, empty card.
4. Analyze: summary and controls cards, list plus treemap grid. Run
   `benchmark:analyze:build`.
5. Status: bento cards, charts card, process card.
6. Support pages: header integration, cards.
7. Delete `SweepBody` `size` prop and any `sweep-body-hero` CSS or test
   lock left after child 2.
8. Gates in `desktop/`: `lint`, `typecheck`, `test`, `build`.
9. Evidence: web widths in EN and zh-CN, reduced motion, forced colors,
   keyboard walk, native 1080x720 and 900x600. Write
   `docs/validation/desktop-sidebar-chrome.md`.
10. Root: `just ci`.

Rollback: restore the owned mode, support, and doc files. Evidence
directories are task-local and need no rollback.
