# Update desktop spec and immersive shell

## Goal

Rewrite the desktop-frontend visual contract, then implement a dark canvas,
centered capsule navigation, original sweep-body tokens, and a labelled
supporting disclosure. This is the first child of
`09-01-desktop-mole-visual-redesign`. Mode workbenches stay functionally
as they are until later children restyle them onto this chrome.

## Requirements

- R1: Update `.trellis/spec/desktop-frontend/index.md` and
  `component-guidelines.md` to the parent immersive contract before any
  React visual edit. Keep `state-management.md` authority invariants.
  Do not edit `.trellis/spec/frontend/`.
- R2: Implement tokens and `AppShell` capsule navigation from parent
  `design.md`. Native Windows chrome. Generated DevSweep icon in the
  capsule. No light workbench background on the shell.
- R3: Supporting destinations remain reachable with visible names via a
  labelled disclosure. Existing `shell.v1.*` forms stay byte-for-byte.
  At most one additive `shell.v1.more` with EN/zh-CN parity.
- R4: Routing, focus restore, Alt accelerators, reduced motion (still
  sweep body), forced colors, and 390/800/1024/1440 layouts pass. TUI
  snapshots do not change.

## Acceptance Criteria

- [x] AC1 (R1): Spec Visual System matches parent design tokens, allows
      display-size numbers and mode canvases, and still forbids Mole
      photographs, five-planet metaphor, traffic lights, and "space freed".
- [x] AC2 (R2, R3): Capsule shows five available modes; More reveals
      Protection, Rules, History, Language, Help by name.
- [x] AC3 (R3, R4): `AppShell` tests cover deep link, keyboard, focus
      restore, missing-mode omission, and both locales. Existing 22 shell
      strings still match `desktop/src/i18n/index.test.ts` plus additive
      `shell.v1.more`.
- [x] AC4 (R4): Reduced motion disables sweep animation. High contrast
      keeps focus and selection. Desktop lint/typecheck/test/build pass.

## Out of Scope

- Clean grouped review, Software/Optimize/Analyze/Status restyle.
- New capabilities, TUI restyle, new production dependencies.

## Dependency

None. This child starts first.
