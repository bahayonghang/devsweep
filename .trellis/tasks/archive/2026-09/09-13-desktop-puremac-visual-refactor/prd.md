# Refactor desktop UI from PureMac visual language

## Goal

Replace the centered-capsule immersive desktop with a sidebar workbench that
reads like a finished product: a persistent left sidebar with named
destinations, a page header on every screen, card surfaces, a Clean stage
hero that states the review-first promise, live scan feedback, category rows,
and file rows. Visual and interaction ideas come from `ref/repo/PureMac`.
Every string, token, glyph, and component is DevSweep-original.

User value: a developer on Windows opens DevSweep and sees where they are,
what the tool promises, what a scan found, and what is selected, without a
bare ring on an empty canvas.

## Confirmed Background

- User request 2026-09-13 with Image #1: the current Tauri desktop is ugly and
  bare. Reference `ref/repo/PureMac`, redesign and refactor the interface,
  then create a Trellis task.
- Current desktop (`desktop/src/app-shell/AppShell.tsx:300-380`,
  `desktop/src/styles.css`, `desktop/src/pages/ScanPage.tsx:83-107`): dark
  canvas, centered capsule `role=tablist`, `More` disclosure, `h1.sr-only`,
  168px CSS ring hero, one hint sentence, bottom scope toolbar, 800x600
  window (`desktop/src-tauri/tauri.conf.json:16-17`).
- Archived parent `09-01-desktop-mole-visual-redesign` delivered the current
  immersive contract. This task supersedes that **shell composition and Clean
  first-screen** direction. Its safety, catalogue, and evidence rules stay.
- PureMac is MIT (`ref/repo/PureMac/LICENSE`). DevSweep is MIT with a
  stricter no-copy policy (`docs/provenance.md`). Audit:
  `research/puremac-visual-audit.md`.
- Spec `.trellis/spec/desktop-frontend/component-guidelines.md` requires
  centered capsule navigation, forbids `linear-gradient`,
  `radial-gradient`, `backdrop-filter`, glow, glass, planet heroes, and
  "space freed" copy. `desktop/src/styles.test.ts:18-44` enforces the same.
- Catalogue is CLI-owned (`resources/i18n/en.json`, `zh-CN.json`, 353 keys).
  `shell.v1.*` is a closed 23-key set frozen in
  `desktop/src/i18n/index.ts:9-33`, `desktop/src/i18n/index.test.ts`, and
  `crates/devsweep-cli/src/i18n/mod.rs:760` (`canonical_shell_v1_namespace_has_exact_copy_and_closed_metadata`).
  Additive keys must update all three freezes. EN and zh-CN must stay
  parallel. TUI uses no `clean.v1.*` key.
- CSP `style-src 'self'` forbids inline styles. No new production frontend
  dependency is authorized. Windows keeps native chrome.
- Native evidence from the Mole task at 390/800/1024/1440 was recorded as
  outstanding in `.trellis/workspace/lyh/journal-1.md` Session 51.

## Resolved Decisions

- Navigation: persistent left sidebar with sections `Modes` and
  `Supporting destinations`, Language and Help in the sidebar footer. Below
  800 CSS pixels the same list becomes a horizontal top strip. The `More`
  disclosure is removed. The five-mode `role=tablist` stays.
- Header: every mode and supporting page shows a visible title and subtitle.
  Mode workbenches may add a status chip from their own state.
- Clean first screen: stage hero with eyebrow, headline, lede, scope
  checkboxes, Scan, and a capacity plaque. The ring hero is removed from all
  modes. `SweepBody` survives only as the compact sidebar brand mark.
- Review: category rows (existing `kind`) with count and subtotal, then file
  rows inside the existing `<table>` semantics. Selection strip shows
  `Selected {count}` and the single select-all.
- Window: default 1080x720, minimum 900x600, title `DevSweep`.
- Kind and risk get bilingual labels through additive `clean.v1.kind.*` and
  `clean.v1.risk.*` keys. Raw enum text leaves the visible UI.

## Requirements

- R1: Clean-room reference. `research/puremac-visual-audit.md` records
  sources, the MIT and provenance boundary, and accept / adapt / reject. No
  PureMac source, string, screenshot, SF Symbol, or hex value ships.
  `docs/provenance.md` names PureMac as a reviewed reference.
- R2: Spec-first. Before product UI edits, rewrite
  `.trellis/spec/desktop-frontend/index.md` and
  `component-guidelines.md`: sidebar shell, page header, card surfaces,
  original glyph set, tinted tiles, card radius 12 with 8 for controls,
  window minimum, top strip below 800px. Keep dark-only canvas, native
  Windows chrome, Segoe UI Variable, visible focus, forced colors, reduced
  motion, Inspect Only, indeterminate Scan Progress, no "space freed", and
  the gradient / glow / glass prohibition. Do not change
  `.trellis/spec/frontend/`.
- R3: Desktop-only and catalogue-safe. TUI, CLI grammar, core services, and
  IPC types stay unchanged. Existing catalogue forms stay byte-for-byte.
  Additive keys are allowed only with EN and zh-CN parity and with every
  frozen key list updated in the same change.
- R4: Preserve every authority invariant in
  `.trellis/spec/desktop-frontend/state-management.md`. Inspect Only stays
  unselectable. Selection change and rescan invalidate dry-run. Execute needs
  a current digest and a second confirmation. Analyze and Status stay
  read-only. The shell never invents counts, sizes, or status.
- R5: Shell. Sidebar lists Clean, Software, Optimize, Analyze, Status as
  tabs with vertical arrow keys, Home, End, and existing Alt accelerators.
  Protection, Rules, History, Language, and Help are visible by name at all
  widths. Deep links, back, focus restore, and mode-local state retention
  stay. Every page has a visible header.
- R6: Clean flow on existing DTOs and reducer phases. Home: stage hero,
  visible scope, Scan, plaque with `Ready`. Scanning: same stage, backend
  phase and message, indeterminate progress, Cancel scan, `Found so far`
  from Scan Preview. Reviewed: category rows then file rows, selection
  strip, Estimated Recoverable, Review dry run. Dry run, confirm, execute,
  and reported keep current copy. Empty, canceled, error, and reduced-motion
  variants are first-class.
- R7: Other modes and support pages on the same chrome. Software, Optimize,
  Analyze, Status, Protection, Rules, and History get the page header, card
  surfaces, tile rows, and sticky summary bars without new IPC.
- R8: Evidence. EN and zh-CN at 390, 800, 1024, and 1440 CSS pixels; forced
  colors; reduced motion; keyboard; native Windows window at default and
  minimum size. Native scaling stays user-operated at 100 / 125 / 150 / 200%.

## Acceptance Criteria

- [ ] AC1 (R1): Audit file exists with sources and dispositions. `grep` of
      `desktop/` and `resources/` finds no PureMac string, symbol name, or
      hex value from `ref/repo/PureMac`. `docs/provenance.md` lists PureMac.
- [ ] AC2 (R2): Spec rewrite lands before any React or CSS edit in the
      children. TUI spec is unchanged.
- [ ] AC3 (R3): `desktop/src/i18n/index.test.ts` and CLI
      `canonical_shell_v1_namespace_has_exact_copy_and_closed_metadata` pass
      with the new key list. EN and zh-CN key sets are equal. `just ci`
      passes after catalogue edits.
- [ ] AC4 (R4, R5): Sidebar tabs, supporting destinations, routing, focus
      restore, and accelerators pass in EN and zh-CN. No `More` disclosure.
      No icon-only destination. Page header is visible on every route.
- [ ] AC5 (R4, R6): Clean home shows stage hero, scope, Scan, and plaque.
      Scanning shows `Found so far` without a percentage. Review shows
      category rows and file rows with bilingual kind and risk labels.
      Inspect Only cannot be selected. Dry run invalidates on selection
      change. Execute needs digest plus confirm. No copy says space was
      freed.
- [ ] AC6 (R7): Software, Optimize, Analyze, Status, Protection, Rules, and
      History render on the sidebar chrome with a header and cards. Analyze
      stays read-only. Status shows no synthetic score.
- [ ] AC7 (R8): 390/800/1024/1440 layouts, reduced motion, forced colors,
      and bilingual native screenshots at 1080x720 and 900x600 are recorded
      under the child evidence directory and pointed to from
      `docs/validation/`.
- [ ] AC8: `desktop` lint, typecheck, test, and build pass. `just ci` passes.

## Out of Scope

- Copying PureMac assets, copy, Swift code, SF Symbols, or pixel geometry.
- Gradients, glow, glass, orb heroes, shimmer, count-up animation.
- Sidebar size badges that need lifted mode state.
- New scan providers, Smart Care style aggregate categories, onboarding,
  Docker cleanup, permanent delete, health score, tray HUD.
- TUI restyle, CLI grammar, core safety, IPC contract changes.
- New production frontend dependencies, icon fonts, or image assets.
- Publishing or pushing.

## Risks

- Catalogue: three frozen `shell.v1.*` lists must change together or desktop
  and CLI tests fail.
- Test churn: `desktop/src/styles.test.ts` and shell tests lock
  `.mode-capsule`, `More`, `.shell-more`, and `.sweep-body-hero`. The
  spec-shell child must move those locks to sidebar equivalents in the same
  change.
- Spec conflict: implementing UI before the spec rewrite repeats the archived
  R5 failure mode.
- Provenance: original glyphs must not trace SF Symbol outlines.
- Width: a 232px sidebar at 800 CSS pixels leaves 568px for content. The top
  strip breakpoint at 800px must be verified in both locales.

## Child Map

| Child                                 | Deliverable                                                                                                                        |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `09-13-desktop-sidebar-spec-shell`    | Spec rewrite, tokens, glyphs, sidebar shell, page header, window size, catalogue freezes                                           |
| `09-13-desktop-clean-stage-workbench` | Clean stage hero, plaque, `Found so far`, category rows, file rows, bilingual labels                                               |
| `09-13-desktop-modes-card-restyle`    | Software, Optimize, Analyze, Status, support pages on card chrome; bilingual width, reduced-motion, forced-colors, native evidence |

The parent is a coordination gate. It is not an implementation target.
Start the spec-shell child first after this plan is approved.

## Notes

- Domain terms follow `CONTEXT.md`: Cleanup Target, Scan Progress, Scan
  Preview, Scan Report, Cleanup Plan, Estimated Recoverable, Inspect Only.
- This parent remains `planning` until the user approves this summary and a
  child `task.py start` runs.
