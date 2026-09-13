# Rewrite desktop spec and build sidebar shell

## Goal

Rewrite the desktop-frontend visual spec for a sidebar workbench, then
implement the sidebar shell, page header, card tokens, original glyph
module, additive catalogue keys, and window sizing. This child is first;
the other two children depend on its chrome. Parent:
`09-13-desktop-puremac-visual-refactor`.

## Requirements

- R1: Spec first. Rewrite `.trellis/spec/desktop-frontend/index.md` and
  `component-guidelines.md` per parent `design.md` section 2 before any
  React, CSS, JSON, or Tauri edit. Add the shell no-derived-data line to
  `state-management.md`. Do not touch `.trellis/spec/frontend/`.
- R2: Sidebar shell. `AppShell` renders `aside.shell-sidebar` with a brand
  row, `Modes` tabs (`role=tablist`, `aria-orientation="vertical"`),
  `Supporting destinations` buttons, and a footer with Language and Help.
  Below 800 CSS pixels the same list renders as a horizontal top strip.
  Remove the `More` disclosure. Keep deep links, back, focus restore, Alt
  accelerators, Home, End, and mode-local state retention. ArrowDown and
  ArrowUp move tabs; ArrowRight and ArrowLeft stay accepted.
- R3: Page header. Every route renders a visible `h1` title, a subtitle,
  and a header slot. The `sr-only` heading is removed. Titles reuse
  `command.*` and `shell.v1.supporting.*`. Subtitles use additive
  `shell.v1.subtitle.*` keys for clean, software, optimize, analyze,
  status, protection, rules, history, and settings.
- R4: Catalogue. Add subtitle keys to `resources/i18n/en.json` and
  `zh-CN.json` with `count none`, no placeholders, no accelerator, no
  group, `truncation never`. Update `SHELL_V1_KEYS` in
  `desktop/src/i18n/index.ts`, the freeze in `index.test.ts`, and the CLI
  test at `crates/devsweep-cli/src/i18n/mod.rs:760` in the same change.
  Existing forms stay byte-for-byte. `shell.v1.more` stays in the catalogue
  but is no longer rendered.
- R5: Tokens and glyphs. Add sidebar, card, border, tile, and radius tokens
  to `desktop/src/styles.css`. Add `desktop/src/app-shell/glyphs.tsx` with
  original inline SVG glyphs for the eight destinations, language, help,
  six target kinds, five ecosystems, and a risk dot. No gradient, glow,
  blur, icon font, or image asset. Keep the compact `SweepBody` as the
  brand mark only; remove the `hero` size.
- R6: Window. `desktop/src-tauri/tauri.conf.json` window becomes 1080x720
  default, 900x600 minimum, title `DevSweep`. CSP and capabilities stay.
- R7: Tests. Move shell, App, and stylesheet locks from capsule, `More`,
  `.shell-more`, and `.sweep-body-hero` to sidebar, page header, and card
  selectors. Add tests for vertical arrow keys, top strip class at 800px
  (stylesheet string), and the subtitle on every route in EN and zh-CN.
- R8: Provenance. Add PureMac to `docs/provenance.md` as a reviewed MIT
  reference with nothing copied.

## Acceptance Criteria

- [ ] AC1 (R1): Spec commits or diff hunks precede UI hunks in the child
      history. `.trellis/spec/frontend/` is unchanged.
- [ ] AC2 (R2, R3): In EN and zh-CN, tabs Clean / Software / Optimize /
      Analyze / Status and buttons Protection / Rules / History / Language
      and link Help are visible without opening anything. No `More` text.
      Each route shows a visible `h1` and subtitle. Focus returns to the
      activating control after back.
- [ ] AC3 (R4): `desktop/src/i18n/index.test.ts` and CLI
      `canonical_shell_v1_namespace_has_exact_copy_and_closed_metadata`
      pass with 33 keys. EN and zh-CN key sets are equal. TUI snapshot
      tests are unchanged.
- [ ] AC4 (R5): `styles.test.ts` still rejects `linear-gradient`,
      `radial-gradient`, `backdrop-filter`, and `filter: blur`. Glyph module
      exports render with `aria-hidden`. No `.sweep-body-hero` selector
      remains.
- [ ] AC5 (R6): `tauri.conf.json` has the new size and the same CSP string.
      `cargo check -p devsweep-desktop` passes.
- [ ] AC6 (R7, R8): Desktop lint, typecheck, test, and build pass.
      `just ci` passes. `docs/provenance.md` lists PureMac.

## Out of Scope

- Clean stage hero, category rows, file rows (child 2).
- Other mode and support page restyle, native screenshots (child 3).
- Sidebar size badges. Removing `shell.v1.more` from the catalogue.

## Dependency

None. This child starts first. Children 2 and 3 wait for this child's
chrome, tokens, and glyphs on the branch.
