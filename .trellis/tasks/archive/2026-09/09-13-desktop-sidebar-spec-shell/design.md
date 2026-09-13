# Design - Sidebar spec and shell

Follow parent `design.md` sections 2, 3, and 6.

## Owned files

- `.trellis/spec/desktop-frontend/index.md`, `component-guidelines.md`,
  `state-management.md` (one added line).
- `desktop/src/app-shell/AppShell.tsx`, `AppShell.test.tsx`, new
  `glyphs.tsx`, `glyphs.test.tsx`, `index.ts` exports.
- `desktop/src/App.test.tsx` shell selectors only.
- `desktop/src/styles.css`, `styles.test.ts`.
- `desktop/src/i18n/index.ts`, `index.test.ts`.
- `resources/i18n/en.json`, `zh-CN.json` (additive keys only).
- `crates/devsweep-cli/src/i18n/mod.rs` test expectation list only.
- `desktop/src-tauri/tauri.conf.json` window block only.
- `docs/provenance.md`.

## Shell DOM

```text
div.app-shell[data-mode][data-locale]
  aside.shell-sidebar[aria-label=app.title]
    div.shell-brand  (SweepBody compact, img.shell-brand-icon, span DevSweep)
    p.shell-section-label  (Modes; additive shell.v1.section.modes)
    nav.mode-navigation[role=tablist][aria-orientation=vertical][aria-label]
      button.mode-tab[role=tab][aria-selected][aria-controls=mode-panel]
        span.glyph-tile > svg (aria-hidden)  span.mode-tab-label
    p.shell-section-label  (shell.v1.supporting)
    nav.support-navigation[aria-label=shell.v1.supporting]
      button.support-link (Protection / Rules / History)
    div.shell-footer
      button (shell.v1.settings.action)   a[href=readme] (shell.v1.help)
  main#mode-workbench.mode-workbench
    header.page-header
      div.page-header-text  h1#mode-heading  p.page-subtitle
      div.page-header-slot  (mode-provided chip, may be empty)
    section#mode-panel.mode-panel[role=tabpanel][aria-labelledby=mode-heading]
```

- The `Modes` section label needs one additive key `shell.v1.section.modes`
  (EN `Modes`, zh-CN `模式`). Total additive shell keys: 1 section label
  plus 9 subtitles = 10. Frozen list grows from 23 to 33.
- `ModeRegistration` gains an optional `subtitleKey` only if needed;
  simpler: a `SUBTITLE_KEYS` map next to `MODE_MESSAGE_KEYS` in `AppShell`.
- Header slot: `AppShell` exposes a `PageHeaderSlot` context or a prop on
  the registration render call. Prefer a small context in `app-shell/` so
  mode children can render a chip without lifting state.

## Keyboard

`onKeyDown` on the tablist: ArrowDown / ArrowRight next, ArrowUp / ArrowLeft
previous, Home first, End last. Alt accelerators unchanged.

## CSS

- Tokens: `--sidebar`, `--card`, `--card-border`, `--tile-alpha`,
  `--radius-card`, `--radius-control`, `--radius-tile`, `--sidebar-width:
232px`.
- `.app-shell { display: grid; grid-template-columns: var(--sidebar-width)
minmax(0, 1fr); }` at 800px and above. Below 800px:
  `grid-template-columns: minmax(0, 1fr)` and the sidebar becomes a
  horizontal strip with `overflow-x: auto`, visible names.
- `.card { background: var(--card); border: 1px solid var(--card-border);
border-radius: var(--radius-card); padding: 16px; }`.
- `.glyph-tile { width: 24px; height: 24px; border-radius: var(--radius-tile);
background: color-mix(in srgb, var(--accent) 16%, transparent); }`.
  If `color-mix` is not acceptable for the WebView2 baseline, use a
  per-mode `--tile` token instead.
- Remove `.sweep-body-hero`, `.mode-capsule`, `.shell-more*`,
  `.shell-header*`, `.capsule-track` rules. Keep `.sweep-body` compact and
  its reduced-motion rule.
- Keep `@media (forced-colors: active)`, `outline: 3px solid var(--focus)`,
  and the four width breakpoints. Update `styles.test.ts` in the same
  change.

## Glyphs

`glyphs.tsx` exports `Glyph` components keyed by a union type:
`clean | software | optimize | analyze | status | protection | rules |
history | language | help | build_artifacts | dependency_directory |
package_cache | test_cache | tool_cache | virtual_env | docker | generic |
node | python | rust | risk`. Each is a 16x16 `viewBox` with
`stroke="currentColor"`, `fill="none"`, `stroke-width 1.5`, `aria-hidden`.
Shapes are simple originals: broom, box, gauge, tree, pulse, shield,
list, clock, globe-less speech mark, question mark, brick, folder stack,
package, flask, wrench, leaf, container, dot, hexagon, snake curve, gear.
No path is traced from SF Symbols or PureMac.

## Tauri

```json
"width": 1080, "height": 720, "minWidth": 900, "minHeight": 600, "title": "DevSweep"
```

CSP string unchanged.

## Compatibility

- `AppShell` exports (`MODE_IDS`, `SUPPORTING_DESTINATION_IDS`,
  `parseShellRoute`, `SweepBody`, `AccessibleUserData`) stay. `SweepBody`
  drops the `size` prop; callers in modes and pages that pass `size="hero"`
  are updated by children 2 and 3, so in this child keep accepting and
  ignoring `size` until those land, then remove it in child 3.
- No coordinator change.
