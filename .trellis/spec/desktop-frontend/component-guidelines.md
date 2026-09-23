# Component Guidelines

## Product Mode

This is a five-mode Operate surface for repeated Clean, Software, Optimize,
Analyze, and Status work. Use one quiet immersive shell with native Windows
chrome, one top-centered capsule navigation bar, a stage host below it, and a
mode-local stage/detail slot. There is no sidebar and no page header block.
Unavailable modes are absent from navigation and deep links; never render a
clickable placeholder. There is no More disclosure.

The capsule is one pill-shaped bar, horizontally centered near the top of the
window. Its left end is the brand-mark button (original DevSweep icon plus the
visible product name). After the brand button comes a `role=tablist` with
`aria-orientation="horizontal"` and one tab per available primary mode in the
order Clean, Software, Optimize, Analyze, Status. The active tab has a solid
light fill and dark text; inactive tabs use muted light text. The brand button
is outside the tablist and opens the brand menu: a real `role=menu` popover
with `role=menuitem` entries for the available supporting destinations
(Protection, Rules, History), Language (the settings route), and Help
(external link). Arrow/Home/End move inside the menu, Escape and Tab close it,
and closing returns focus to the brand button. Below 800 CSS pixels the
capsule scrolls horizontally (`overflow-x: auto`); tab names stay fully
visible, nothing overlaps the capsule, and the planet shrinks. Names,
authority, warnings, and critical actions are never icon-only or hidden by
responsive layout.

The shell is presentation and lifecycle infrastructure only. It never invents
targets, plans, digests, command arguments, authorization, mode results,
counts, sizes, or status. Mode children own their typed pages and reducers.
Protection, Rules, History, Settings, and language are supporting destinations,
not sixth primary modes.

The default window is 1080x720 CSS pixels with a 900x600 minimum. The window
title is `DevSweep`.

## Composition

- Keep application registration composition in `App`; route, shell, and
  presentation-setting code live under `app-shell/`, while mode pages receive
  typed state and command callbacks rather than importing Tauri directly.
- Register routes from one typed feature registry. Preserve deterministic deep
  links/back behavior, restore focus to the activating navigation control, and
  omit unavailable registrations atomically.
- Compose the shell as the capsule bar plus a stage host. Mode routes carry a
  visually hidden `h1` that names the mode; the active capsule tab is the
  visible identity. Supporting and Language routes render in the stage host
  with a visible back control to the last mode, a visible `h1`, and a
  subtitle.
- Every mode's first screen uses the shared `Stage` stack from
  `desktop/src/stage/`: planet hero, one primary number or state title, one
  secondary line, one primary action, and an optional secondary action.
  Mode controls that the first action needs (scope, path) sit between the
  secondary line and the primary action. `StageResult` shows one number with
  unit, one secondary line of facts, and one action. The first screen has no
  cards.
- Detail content (review lists, tables, treemap, process table, previews,
  results, audits) renders in `DetailView`: a full-width card region below the
  capsule that replaces the stage and has a "Back to overview" control. A
  mode may show a detail region below the stage while its first action runs
  (for example, the Clean scan preview); that region has no back control.
- Status chips render in the stage secondary line or the detail header. There
  is no page-header portal.
- Card surfaces use the raised card token, a 1px hairline border, radius 12,
  and padding 16. Controls, inputs, badges, and rows stay at radius 8 or below.
  Primary actions keep the pill radius.
- Destination and kind marks use a tinted 24px icon tile, radius 6, 16% accent
  tint, and an original inline SVG glyph with `aria-hidden`. Glyphs come from
  one DevSweep glyph module. No icon font, no SF Symbols, no image sprite.
  Names and authority are never icon-only.
- Put repeated target presentation in focused components such as `TargetTable`,
  `RiskBadge`, `CapacityLabel`, and `EvidenceList`.
- Use a real table for comparable target data. Completed-review mode keeps
  selection in the first column; active/stopped preview mode has no selection
  column or action callback. Keep primary path/name next and risk/capacity/status
  facts aligned.
- Keep the persistent summary/action bar outside the table frame. It must remain
  stable when selection, errors, or result counts change.
- Use a dialog only for the destructive second confirmation. Trap focus through
  the native `<dialog>` element and provide clear cancel/confirm actions.
- Keep Clean observation, completed-report authority, selection, dry-run digest,
  and confirmation surfaces intact when composing the mode slot. Shell
  navigation must not promote an observation or reconstruct a cleanup request.

## Controls And Accessibility

- Use buttons for commands, checkboxes for selection, and `<details>` for
  evidence disclosure. Do not make rows or styled `<div>` elements act as
  controls.
- Every checkbox has an accessible target label. Disabled inspect-only controls
  explain their state in adjacent text and a title.
- Provide visible `:focus-visible` states and never rely on color alone for risk,
  status, or selection.
- Capsule and brand-menu navigation implement arrow/Home/End keyboard
  movement and a stable active-page announcement. ArrowRight and ArrowDown
  move to the next mode tab; ArrowLeft and ArrowUp move to the previous tab.
  Route changes restore focus (mode routes to the mode tab, supporting and
  Language routes to the brand button); language changes do not reset
  mode-local state.
- Bind catalogue-owned locale accelerators only when the accelerator is unique
  in the currently visible scope. A collision removes the conflicting shortcut;
  it never makes two controls fire or silently chooses one.
- User paths, application names, and similar data may be visually ellipsized
  only when the complete value remains in accessible text and is copyable.
  Authority, refusal, warning, and action text never truncates.
- Loading buttons preserve width and state their active operation.
- Honor `prefers-reduced-motion`; the scan indicator remains meaningful without
  animation.

## Copy And Formatting

- Commands use direct labels: `Scan`, `Cancel scan`, `Review dry run`, `Execute`.
- Progress uses a labelled native indeterminate `<progress>`, requested phase
  states, the exact backend message, and discovered-so-far count. Never show a
  percentage because the contract has no total. Announce only a coalesced
  message/count status, no more than once per second under same-phase discovery;
  phase and cancellation changes may announce immediately. Never put the changing
  result table inside a live region.
- Use `Estimated recoverable` for plans and dry runs. A successful trash outcome
  says `Moved to trash; capacity becomes available after trash is emptied`.
- Consume the CLI catalogue's binary-unit, plural, accelerator, and truncation
  metadata. Do not redefine units or catalogue keys in React. Mark partial
  values through the canonical localized copy and retain `Unknown` semantics.
- Human copy may be English or Simplified Chinese. Machine fields, error codes,
  plan identities, digests, selection, and cleanup authority are locale-neutral.

## Visual System

- Use Segoe UI Variable with system UI fallbacks and tabular numerals for byte
  values. Estimated Recoverable and live metrics may use a display-size tabular
  number. Do not scale the whole UI from viewport fonts.
- Card surfaces may use radius 12. Controls, inputs, badges, and rows stay at
  radius 8 or below. Primary actions may use a full pill radius. State a radius
  through `--radius-card`, `--radius-control`, or `--radius-tile`, never as a
  literal pixel value. The pill (`999px`) and circle (`50%`) shapes are the two
  exceptions. `desktop/src/styles.test.ts` enforces this.
- Use a dark-only canvas. There is no light workbench pane. State the shared
  grammar once in the base rule; do not write a light default and darken it
  again under `.clean-mode` or another mode selector, because the four modes
  without that override then render the light surface.
  `desktop/src/styles.test.ts` holds the list of removed light values.
  Shared tokens are
  `--canvas`, `--text`, `--muted`, `--border`, `--accent`, `--focus`,
  `--danger`, `--warning`, `--ok`, `--card`, `--card-border`, `--tile-alpha`,
  and the capsule tokens `--capsule-bg`, `--capsule-border`,
  `--capsule-active-bg`, `--capsule-active-text`, and `--stage-canvas`.
  All modes share one near-black blue `--stage-canvas`. Mode identity comes
  only from the planet palette and the per-mode accent:
  `--accent-clean`, `--accent-software`, `--accent-optimize`,
  `--accent-analyze`, and `--accent-status`. Supporting destinations use the
  Clean accent. Semantic amber/red/green remain reserved for risk, error, and
  safe actions.
- The original product icon may appear in native chrome and the brand button.
  When adjacent DevSweep text supplies the accessible product name, the image
  is decorative so the product name is announced once.
- The procedural planet (`desktop/src/stage/Planet.tsx`) is the only stage
  hero. It is drawn by DevSweep code on a `<canvas>` from a per-mode seed and
  an original palette in `planet-palettes.ts` (Clean ocean/land, Software
  rust-red, Optimize grey-silver, Analyze banded amber, Status warm
  yellow-white). It is `aria-hidden`, non-interactive, and carries no data.
  It renders at a capped source resolution (256 px diameter), rotates at no
  more than 30 frames per second only while the mode is active and the
  document is visible, stops on route change, `visibilitychange` to hidden,
  and unmount, and draws exactly one still frame under
  `prefers-reduced-motion`. Under `forced-colors: active` it renders as a
  plain outlined circle. No image asset, texture file, photograph, WebGL, or
  new dependency backs it.
- A status chip is a pill, 11px semibold, dot plus text, and semantic tint only.
- A stacked meter uses solid segments with 2px separators and no animation
  beyond width transition, still under reduced motion. Segments are verified
  and partial lower bound only. Unknown is labelled, never drawn.
- Prohibit glass, glow, photographic planets, NASA imagery, Mole assets,
  textures, geometry, or colour values, fake macOS traffic lights,
  `linear-gradient`, `radial-gradient`, `backdrop-filter`, and copy that says
  space was freed or released. Shading exists only inside the planet canvas.
- Test stable shell behavior at 390, 800, 1024, and 1440 CSS pixels, forced
  colors/high contrast, reduced motion, keyboard-only navigation, and both
  locales. Native Windows scaling evidence is direct and separately recorded;
  automation must not change the user's display scaling.
