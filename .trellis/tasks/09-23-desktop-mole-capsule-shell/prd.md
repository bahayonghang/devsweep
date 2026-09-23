# Mole-style capsule shell and procedural planet stage

## Goal

Replace the left-sidebar workbench with a Mole-informed stage layout: one
centered capsule navigation bar at the top, a centered original planet visual
per mode, one large primary number, one line of secondary facts, and one
primary action. Detail lists open below or in place of the stage. The layout
topology follows Mole; every pixel, string, colour value, and texture is
DevSweep-original.

## User decision (2026-09-23)

The user compared the current sidebar shell with the Mole desktop app and
selected:

- top-centered capsule navigation for the five modes;
- supporting destinations (Protection, Rules, History, Language, Help) in a
  popover menu opened from the brand mark at the left end of the capsule;
- original procedural planets, one per mode, rendered by DevSweep code. No
  NASA or other photograph, no Mole asset, no Mole texture.

This decision supersedes the 09-13 PureMac sidebar contract and requirements
R1/R2 of `09-20-desktop-mole-workbench-ux` where they require a sidebar.

## Dependency and boundary

- Parent: `09-20-desktop-mole-tauri-performance`.
- First child of the 09-23 round. The Clean, Software, Analyze, and Status
  parity children build their stages on the components of this child.
- Frontend only: `desktop/src/`, desktop styles, i18n catalogue, desktop
  frontend spec, and `docs/provenance.md`. No Tauri command, core type, or
  generated wire type changes.

## Requirements

- R1 Capsule navigation: one pill-shaped bar, horizontally centered at the top
  of the window. Left end: brand mark button. Then five mode tabs in the
  order Clean, Software, Optimize, Analyze, Status. The active tab has a solid
  light fill and dark text; inactive tabs use muted text. The bar keeps
  `role=tablist`, arrow/Home/End movement, Alt accelerators, deep links, back
  navigation, and focus restoration from the current shell.
- R2 Brand menu: activating the brand mark opens a menu with Protection,
  Rules, History, Language, and Help. It is a real menu (`role=menu`,
  Escape closes, focus returns to the brand mark). A supporting destination
  renders in the stage area with a back control to the last mode.
- R3 Stage grammar: every mode first screen is a vertical centered stack:
  planet (hero), primary number or state title, one secondary line, one
  primary action button, optional secondary action link. Detail views (lists,
  tables, treemap, process table) render in a full-width card area below the
  capsule and replace the stage; a "back to planet" control returns to the
  stage. No page header block and no sidebar remain.
- R4 Procedural planet: one `Planet` component draws a lit sphere on a
  `<canvas>` from a per-mode seed and palette (Clean: ocean/land, Software:
  rust-red, Optimize: grey-silver, Analyze: banded amber, Status: warm
  yellow-white). It rotates slowly while the mode is active and visible, stops
  when the page is hidden or the route changes, and draws one still frame under
  `prefers-reduced-motion`. It is `aria-hidden` and carries no data. In forced
  colours it renders as a plain outlined circle.
- R5 Big-number result: a shared `StageResult` component shows one number with
  unit, one secondary line of facts, and one action. Number formatting reuses
  the existing byte/count formatters. It never shows "freed"/"released" copy;
  lower-bound and unknown values keep their current labels.
- R6 Background: a dark, near-black blue canvas shared by all modes. Mode
  identity comes from the planet palette and the accent colour only.
- R7 Window: keep 1080x720 default and 900x600 minimum. At widths below
  800 CSS px the capsule scrolls horizontally and the planet shrinks; nothing
  overlaps the capsule.
- R8 Copy: all new strings are original DevSweep copy in English and
  Simplified Chinese. Do not reuse Mole taglines, poems, or labels such as
  "返回地球".

## Acceptance Criteria

- [ ] AC1: No sidebar element remains; the capsule renders five tabs plus the
      brand mark, and route/back/focus/keyboard/accelerator tests pass.
- [ ] AC2: The brand menu opens and closes by mouse and keyboard, lists the
      five supporting entries, and restores focus; Protection/Rules/History
      pages render inside the stage with a back control.
- [ ] AC3: Each mode's first screen renders the stage stack in both locales at
      390/800/1024/1440 CSS px without overlap or clipped action copy.
- [ ] AC4: `Planet` animation stops on route change, on `visibilitychange`
      to hidden, and on unmount (test with fake timers / mocked
      `requestAnimationFrame`); reduced motion draws exactly one frame.
- [ ] AC5: Forced-colours and reduced-motion selectors exist and are tested.
- [ ] AC6: The desktop frontend spec and `docs/provenance.md` describe the
      capsule shell and the original procedural planets; the spec no longer
      requires a sidebar or forbids an original procedural planet hero.
- [ ] AC7: `npm run lint`, `typecheck`, `test`, and `build` pass in `desktop/`.

## Out of scope

- Mode feature changes (owned by the four parity children).
- Photographic textures, image assets for planets, WebGL, or new npm
  dependencies.
- Native scaling evidence (owned by `09-20-desktop-native-acceptance`).
