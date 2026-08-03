# Component Guidelines

## Product Mode

This is an Operate surface for repeated cleanup review. Prefer a quiet,
workbench-like application shell: stable toolbar, dense table, restrained
surfaces, familiar controls, and semantic color reserved for action and risk.
Do not use a landing-page hero, decorative cards, nested cards, or oversized
display type.

## Composition

- Keep application composition in `App`; pages receive typed state and command
  callbacks rather than importing Tauri directly.
- Put repeated target presentation in focused components such as `TargetTable`,
  `RiskBadge`, `CapacityLabel`, and `EvidenceList`.
- Use a real table for comparable target data. Keep selection in the first
  column, primary path/name next, and risk/capacity/action facts aligned.
- Keep the persistent summary/action bar outside the table frame. It must remain
  stable when selection, errors, or result counts change.
- Use a dialog only for the destructive second confirmation. Trap focus through
  the native `<dialog>` element and provide clear cancel/confirm actions.

## Controls And Accessibility

- Use buttons for commands, checkboxes for selection, and `<details>` for
  evidence disclosure. Do not make rows or styled `<div>` elements act as
  controls.
- Every checkbox has an accessible target label. Disabled inspect-only controls
  explain their state in adjacent text and a title.
- Provide visible `:focus-visible` states and never rely on color alone for risk,
  status, or selection.
- Loading buttons preserve width and state their active operation.
- Honor `prefers-reduced-motion`; the scan indicator remains meaningful without
  animation.

## Copy And Formatting

- Commands use direct labels: `Scan`, `Cancel scan`, `Review dry run`, `Execute`.
- Progress shows the exact backend message and a human phase label; never show a
  percentage because the contract has no total.
- Use `Estimated recoverable` for plans and dry runs. A successful trash outcome
  says `Moved to trash; capacity becomes available after trash is emptied`.
- Use `B`, `KB`, `MB`, `GB`, and `TB` consistently. Mark partial values as `At
  least`, and show `Unknown` when confidence is unknown.

## Visual System

- Use system UI fonts and tabular numerals for byte values.
- Keep radii at 8px or below for tool surfaces.
- Use neutral white/gray surfaces, green for the primary safe action, amber for
  elevated risk/irreversible warnings, and red only for dangerous/error states.
- Avoid gradients, glow, glass effects, decorative illustrations, and viewport-
  scaled font sizes.

