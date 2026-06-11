# Optimize TUI visual hierarchy and layout - Design

## Design Lens

`design-taste-frontend` is out of scope for terminal dashboards as a direct web
implementation guide, but it is useful as an audit discipline:

- infer the product surface before styling
- preserve existing IA during a redesign unless there is a reason to change it
- avoid decorative UI filler
- use one consistent visual system
- make state transitions and empty/error states visible

Applied to `devsweep`, the design target is a dense, keyboard-first devtool
TUI. This is not a marketing page, not a colorful dashboard, and not a place
for animation. Use compact hierarchy, semantic color, aligned columns, and
clear labels.

Adapted dials:

- `DESIGN_VARIANCE 4`: structured, with modest layout adaptation by width.
- `MOTION_INTENSITY 1`: static terminal UI only.
- `VISUAL_DENSITY 7`: efficient scanning without hiding safety information.

## Current UI Audit

The existing TUI has a sound foundation:

- `src/tui.rs` owns terminal setup, app state, event/update logic, render
  helpers, overlays, worker events, logs, and tests.
- `render_app` divides the screen into header, tabs, body, and footer.
- The main body for Dashboard/Global/Projects uses categories, targets, and
  details columns.
- Confirmation, dry-run, details, help, and cleanup progress overlays are
  already present.
- Recent archived tasks already improved confirmation copy, context-aware
  footer actions, display-only Windows path normalization, and cleanup progress.

The current style and layout weaknesses are mostly structural:

- Colors are repeated as raw RGB values across many render helpers. Some helper
  functions exist, but there is no compact semantic palette contract for new
  work to follow.
- The body layout assumes fixed horizontal percentages. Categories, targets,
  and details are always shown together, which is fragile below wide terminal
  sizes.
- The target list is rendered as paragraph lines with manual padding. It lacks
  a header row and can become harder to scan when paths are long.
- Header and footer are useful but dense. The header mixes brand/tagline,
  selected bytes, scope bytes, filter, risk, and active jobs in a fixed height.
  The footer can become too long when every normal-mode action is present.
- Jobs/Logs currently render some debug-shaped status text. It is functional
  but less polished than the confirmation and progress surfaces.
- Centered modals use fixed percentage sizing, which can be awkward on short or
  narrow terminals.

## Proposed Direction

Preserve the five-view IA and make the visual system more systematic instead of
redesigning the product. This is a targeted evolution, not a full rewrite.

### Semantic Style Layer

Add a small internal style vocabulary near the render helpers. Keep it local to
`src/tui.rs` unless the TUI is split for other reasons.

Suggested roles:

- surface
- surface raised
- border
- border focused
- text
- text muted
- accent
- warning
- danger
- selected row
- footer background

Risk styling should continue to show text labels. Color is a supplement, not
the only signal.

### Width-Aware Layout

Introduce a small layout classification from `Rect`, for example:

- Wide: roughly `>= 120` columns. Keep the three-column body.
- Medium: roughly `100-119` columns. Prefer targets plus details, and fold
  category counts into the header or a compact summary.
- Narrow: roughly `< 100` columns. Prefer targets as the primary full-width
  surface and make details available through the existing details overlay.

The approved size target is full layout at `100x28` and graceful degraded
layout at `80x24`. The exact breakpoints can be adjusted during implementation,
but tests should lock the intended behavior.

### Target List

Make the target list the primary visual surface. Preferred implementation is a
Ratatui `Table` or an equivalent dedicated row formatter with explicit column
widths.

Required row information:

- current cursor
- selected mark
- risk label
- estimated size
- compact target identity

Optional but useful if width allows:

- scope marker
- action kind

The selected row should use both a cursor/marker and a selected row style.
Do not rely only on background color.

### Header And Footer

Header should answer:

- what view am I in?
- how much is selected?
- are filters active?
- is a job running?

Footer should answer:

- what mode am I in?
- what are the currently valid primary actions?

Keep the footer context-aware. At narrower widths, prefer omitting secondary
actions over wrapping or clipping critical actions.

### Overlays And Secondary Views

Keep the specialized confirmation and cleanup progress behavior from recent
tasks. Polish only where the layout work requires it:

- size modals with clamped width/height instead of fixed percentages only
- keep confirmation and progress readable on the supported narrow size
- make Jobs/Logs more scan-friendly without changing worker state or audit
  semantics

## Boundaries

Allowed:

- `src/tui.rs` render/style/layout helpers and tests
- small layout/style helper extraction if it removes real complexity
- updated `TestBackend` tests for size-specific rendering

Not allowed:

- scanner/provider/executor behavior changes
- cleanup-plan serialization changes
- audit JSONL changes
- permanent-delete enablement
- default-selection safety changes
- render-side filesystem or process work

## Compatibility And Risk

The main risk is accidentally changing behavior while polishing presentation.
Mitigation:

- keep updates in pure render helpers where possible
- add tests before or alongside each visible behavior change
- assert that selected target state and rendered path normalization still work
- finish with `just ci`

The second risk is overfitting to one terminal size. Mitigation:

- lock at least three representative render sizes with `TestBackend`
- avoid string assertions that require exact full-frame snapshots unless the
  layout is intentionally fixed

## Rollback

Rollback should be local to TUI rendering and tests. Reverting the layout/style
helpers and related tests should restore the previous UI without data migration
or cleanup model changes.
