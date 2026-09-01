# Redesign desktop UI from Mole visual language

## Goal

Replace the current gray-green Windows workbench with an immersive, review-first
desktop: one centered five-mode entrance, a dark canvas per mode, and a
first screen whose hero is a truthful number plus an original sweep body.

The user compared `desktop/` Clean empty state ("No scan results") with Mole
for Mac's Clean home. The archived parent
`08-29-mole-inspired-cli-windows-desktop-redesign` delivered five-mode
capability and explicitly chose structural depth instead of Mole's visual
world. This task supersedes that **desktop visual** direction only.

User value: a developer on Windows can open DevSweep, see estimated
recoverable capacity as the primary fact, scan with visible scope, review
grouped Cleanup Targets, and confirm cleanup without a light admin table as
the first screen.

## Confirmed Background

- User request 2026-09-01: take ideas from https://mole.fit/zh/, restructure
  desktop UI and interaction, create a Trellis task. Attached Image #1 is
  current Clean empty state. Image #2 is Mole Clean home (navy canvas,
  capsule nav, Earth, 86.4 GB, 返回地球).
- Archived R5 and
  `.trellis/spec/desktop-frontend/component-guidelines.md:82-99` forbid
  heroes, page-wide palettes, oversized display type, planets, and glass.
  `desktop/src/styles.css:1-50` implements dark header `#101914` plus light
  workbench `#eef1ef`. `desktop/src/pages/ReviewPage.tsx:15` renders the
  empty heading.
- Mole sources fetched 2026-09-01: https://mole.fit/zh/,
  https://mole.fit/index.md, https://mole.fit/blog/the-design-of-mole.md,
  https://mole.fit/mac-cleaner.md, and `/img/ch/{clean,uninstall,optimize,analyze,status,menubar}-1400w.webp`.
  Audit: `research/mole-fit-visual-audit.md`.
- Mole is GPL-3.0 with reserved Mac-app assets. DevSweep is MIT
  (`docs/provenance.md`). Earth in Mole uses NASA Blue Marble. No Mole
  asset, copy, hamster, traffic light, or planet photograph may ship.
- Five modes and supporting destinations already exist
  (`desktop/src/app-shell/AppShell.tsx:11-15`). Clean authority lives in
  `desktop/src/state/app-state.ts` via `desktop/src/modes/clean/reducer.ts`.
  Catalogue is CLI-owned (`resources/i18n/en.json`, `zh-CN.json`). Shell V1
  is a closed 22-key set (`desktop/src/i18n/index.ts:9-32`). TUI consumes
  the same `shell.v1.*` keys (`crates/devsweep-cli/src/tui/shell/mod.rs:102`).
- No new production frontend dependency is authorized.

## Resolved Decisions

- Visual world: immersive recreation (centered capsule, per-mode dark
  worlds, lists as drill-down). Mole photographs, hamster, traffic lights,
  and copy stay out.
- Functionality: restructure existing five-mode interaction. Software
  updates, startup items, leftover-file matching, tray HUD, health score,
  and fan control are a later parent.
- Hero: the truthful number is primary. An original CSS sweep body
  supports it (one shape family, five tints, no celestial metaphor).
  Reduced motion keeps a still shape. The generated DevSweep icon stays in
  the capsule and native chrome.
- Clean scope: Projects and Global caches stay visible on the first screen,
  next to Scan, default both on.

## Requirements

- R1: Clean-room reference. Record Mole ideas as accept, adapt, or reject.
  Do not ship Mole assets, copy, or geometry.
- R2: Spec-first immersive contract. Before product UI edits, update
  `.trellis/spec/desktop-frontend/index.md`, `component-guidelines.md`, and
  `state-management.md`. Allow a dark-only canvas, centered capsule
  navigation, per-mode tinted worlds, an original CSS sweep body, display-size
  Estimated Recoverable, and pill radii on chrome only. Forbid NASA/planet
  photographs, a five-planet metaphor, fake traffic lights, copied Mole
  geometry, and "space freed" claims. Keep Clean safety, bilingual
  catalogue ownership, visible focus, and reduced motion. Do not change
  `.trellis/spec/frontend/` TUI chrome.
- R3: Desktop-only. TUI, CLI command grammar, and core services stay
  unchanged. Existing `shell.v1.*` and command message forms stay
  byte-for-byte. Additive catalogue keys are allowed when EN and zh-CN stay
  parallel and TUI does not require them.
- R4: Preserve every cleanup, software, optimize, analyze, and status
  authority invariant in `state-management.md`. Inspect Only stays
  unselectable. Selection and rescan still invalidate dry-run. Execute
  still needs a current digest and a second confirmation. Analyze and
  Status stay read-only.
- R5: Clean first-screen flow using existing DTOs. Home: sweep body +
  capacity or ready copy + visible scope + Scan. Scanning: one calm working
  state with backend phase/message and indeterminate progress, no
  percentage. Review: groups from existing `kind` and `scope`, then
  target rows. Dry-run and confirm stay. Result: large truthful outcome
  number with existing trash copy, then return to review. Empty, error,
  canceled, and reduced-motion variants are first-class.
- R6: Other modes, same chrome. Software: denser rows and a sticky remove
  summary from existing inventory. Optimize: closed-catalogue running
  checklist under the mode hero. Analyze: keep list + treemap, restyle
  only, no trash from the treemap. Status: denser bento from existing
  snapshot/live metrics, no health score, no GPU-zero invention.
- R7: Navigation. Primary modes remain Clean, Software, Optimize, Analyze,
  Status. Protection, Rules, History, Language, and Help remain reachable
  supporting destinations with visible names (disclosure is allowed; icon-only
  is not). Deep links, back, focus restore, and Alt accelerators stay.
- R8: Windows evidence. EN and zh-CN at 390, 800, 1024, and 1440 CSS
  pixels; forced colors; reduced motion; keyboard; native Windows chrome.
  Native scaling evidence stays user-operated at 100/125/150/200%.

## Acceptance Criteria

- [ ] AC1 (R1): `research/mole-fit-visual-audit.md` lists sources, GPL and
      trademark boundary, and accept/adapt/reject. No Mole asset is in
      `desktop/` production or tests.
- [ ] AC2 (R2): Desktop-frontend spec describes the immersive contract
      before React visual implementation. TUI spec is unchanged.
- [ ] AC3 (R3, R7): Capsule navigation, supporting disclosure, routing,
      focus restore, and accelerators pass in EN and zh-CN. Existing
      catalogue forms are unchanged. TUI still renders current `shell.v1.*`
      strings.
- [ ] AC4 (R4, R5): Clean home is not the current light empty heading.
      Scope checkboxes remain visible. Inspect Only cannot be selected.
      Dry-run still invalidates on selection change. Execute still needs
      digest + confirm. No copy says space was freed.
- [ ] AC5 (R5, R6): Software, Optimize, Analyze, and Status first screens
      sit on the dark canvas with one primary action. Analyze stays
      read-only. Status shows no synthetic health score.
- [ ] AC6 (R8): 390/800/1024/1440 layouts, reduced motion (still sweep
      body), high contrast, and bilingual native evidence are recorded.
- [ ] AC7: `desktop` lint, typecheck, test, and build pass. `just ci`
      passes after catalogue or Tauri contract edits.

## Out of Scope

- Copying Mole planets, hamster, traffic lights, marketing copy, or
  pixel-identical layouts.
- TUI restyle.
- Tray HUD, fan control, camera/mic HUD, screen-clean mode, Software
  updates, startup items, leftover-file matching, synthetic health score,
  Analyze-from-treemap delete.
- Invented secondary metaphors (4K-minute equivalents, cumulative "space
  freed").
- New production frontend dependencies, WebGL, or photographic assets.
- Changing CLI grammar, core safety, or permanent-delete policy.
- Publishing or pushing.

## Risks

- Catalogue: `desktop/src/i18n/index.test.ts:63` freezes the 22-key shell
  namespace. Additive keys must update that freeze without mutating existing
  forms, or TUI snapshots fail.
- Spec conflict: implementing UI before the spec rewrite repeats the
  archived R5 failure mode.
- Provenance: an Earth-like sphere even in CSS can read as Mole. The sweep
  body must stay an abstract ring/body, not a globe map.

## Child Map

| Child | Deliverable |
| --- | --- |
| `desktop-immersive-spec-shell` | Spec rewrite, tokens, AppShell capsule, supporting disclosure |
| `desktop-clean-hero-workbench` | Clean home, scan working state, grouped review, result hero |
| `desktop-modes-immersive-restyle` | Software, Optimize, Analyze, Status restyle + bilingual native evidence |

The parent is a coordination gate. It is not an implementation target.
Start the spec-shell child first after this plan is approved.

## Notes

- Domain terms follow `CONTEXT.md`: Cleanup Target, Scan Progress, Scan
  Preview, Scan Report, Cleanup Plan, Estimated Recoverable, Inspect Only.
- This parent remains `planning` until the user approves this summary and
  a child `task.py start` runs.
