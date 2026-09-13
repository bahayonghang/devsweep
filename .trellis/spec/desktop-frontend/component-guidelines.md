# Component Guidelines

## Product Mode

This is a five-mode Operate surface for repeated Clean, Software, Optimize,
Analyze, and Status work. Use one quiet immersive shell with native Windows
chrome, a compact original DevSweep brand, a persistent left sidebar workbench
(not a centered capsule), a visible page header, a mode-local canvas slot, and
a persistent action/status boundary. Unavailable modes are absent from
navigation and deep links; never render a clickable placeholder. There is no
More disclosure.

The sidebar has two sections. Modes is a `role=tablist` with
`aria-orientation="vertical"` and one tab per available primary mode.
Supporting destinations are named buttons for Protection, Rules, and History.
The footer holds Language and Help. Below 800 CSS pixels the same list is a
horizontal top strip: `overflow-x: auto`, destination names stay fully
visible and do not overlap, and section labels may be visually hidden while
remaining accessible. Names, authority, warnings, and critical actions are
never icon-only or hidden by responsive layout.

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
- Compose the shell as a persistent sidebar plus a workbench column. The
  workbench starts with a page header: a visible `h1` title, a subtitle, and an
  optional status chip in the header slot. Do not use an `sr-only` page
  heading.
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
- Primary/support navigation implements arrow/Home/End keyboard movement and a
  stable active-page announcement. ArrowDown and ArrowRight move to the next
  mode tab; ArrowUp and ArrowLeft move to the previous tab. Route changes
  restore focus; language changes do not reset mode-local state.
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
  radius 8 or below. Primary actions may use a full pill radius.
- Use a dark-only canvas. There is no light workbench pane. Shared tokens are
  `--canvas`, `--text`, `--muted`, `--border`, `--accent`, `--focus`,
  `--danger`, `--warning`, `--ok`, `--sidebar`, `--card`, `--card-border`, and
  `--tile-alpha`. Each primary mode sets `--canvas` and `--accent` from an
  original mineral/forest family:
  `--canvas-clean` / `--accent-clean` (pine),
  `--canvas-software` / `--accent-software` (oxide),
  `--canvas-optimize` / `--accent-optimize` (olive),
  `--canvas-analyze` / `--accent-analyze` (umber),
  `--canvas-status` / `--accent-status` (gold-green).
  Supporting destinations use the shell canvas. Semantic amber/red/green remain
  reserved for risk, error, and safe actions.
- The original product icon may appear in native chrome and the sidebar brand
  row. When adjacent DevSweep text supplies the accessible product name, the
  image is decorative so the product name is announced once.
- The sweep body is CSS-native, one shape family, five accent tints,
  non-informational, non-interactive, and still under
  `prefers-reduced-motion`. It is an abstract ring/body, not a globe map, not
  a five-planet metaphor, and not status evidence. Keep only a compact instance
  as the sidebar brand mark. Do not place a hero instance.
- A status chip is a pill, 11px semibold, dot plus text, and semantic tint only.
- A stacked meter uses solid segments with 2px separators and no animation
  beyond width transition, still under reduced motion. Segments are verified
  and partial lower bound only. Unknown is labelled, never drawn.
- Prohibit glass, glow, photographic or planet heroes, NASA imagery, Mole
  geometry, fake macOS traffic lights, `linear-gradient`, `radial-gradient`,
  `backdrop-filter`, and copy that says space was freed or released.
- Test stable shell behavior at 390, 800, 1024, and 1440 CSS pixels, forced
  colors/high contrast, reduced motion, keyboard-only navigation, and both
  locales. Native Windows scaling evidence is direct and separately recorded;
  automation must not change the user's display scaling.
