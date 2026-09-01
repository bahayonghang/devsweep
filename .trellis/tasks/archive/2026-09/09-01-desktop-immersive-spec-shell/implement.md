# Implement - Spec and immersive shell

1. Rewrite desktop-frontend Visual System and index completion checklist.
2. Add CSS variables and capsule layout. Remove `.mode-workbench` light
   `#eef1ef` as the default canvas.
3. Change `AppShell` presentation. Keep route/focus/coordinator logic.
4. If needed, add `shell.v1.more` only; do not mutate existing forms.
5. Update `AppShell.test.tsx` and `styles.test.ts`.
6. Run desktop lint, typecheck, test, build. If catalogue changed, run
   CLI i18n/TUI tests and `just ci`.

Rollback: restore spec + `AppShell` + `styles.css` + catalogue freeze.
