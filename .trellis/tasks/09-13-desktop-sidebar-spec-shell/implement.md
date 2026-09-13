# Implement - Sidebar spec and shell

1. Spec: rewrite `component-guidelines.md` Product Mode, Composition, and
   Visual System; update `index.md` completion checklist; add the shell
   no-derived-data line to `state-management.md`. Validation gate: review
   diff; no code yet.
2. Catalogue: add 10 `shell.v1.*` keys to `en.json` and `zh-CN.json`.
   Update `SHELL_V1_KEYS`, `index.test.ts` expected arrays, and the CLI
   `expected` table. Run `cargo test -p devsweep-cli i18n` and desktop
   `npm run test -- i18n`.
3. Tokens and glyphs: extend `styles.css`, add `glyphs.tsx` plus test.
4. Shell: rewrite `AppShell` presentation to the sidebar DOM, page header,
   header slot context, vertical keyboard. Keep route and focus logic.
5. Tests: update `AppShell.test.tsx`, `App.test.tsx` shell selectors,
   `styles.test.ts`. Add vertical arrow key, top strip string, and subtitle
   tests in EN and zh-CN.
6. Tauri: window size and title. `cargo check -p devsweep-desktop`.
7. Provenance: add PureMac paragraph to `docs/provenance.md`.
8. Gates in `desktop/`: `types:generate`, `lint`, `typecheck`, `test`,
   `build`. Root: `just ci`.

Rollback: restore spec, shell, styles, i18n, catalogue, Tauri, and
provenance files. Do not leave additive keys in one locale only.
