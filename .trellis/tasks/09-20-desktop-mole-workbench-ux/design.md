# Design — workbench UX

## Composition

Keep `AppShell` as the presentation/lifecycle owner and keep mode reducers as
the only owners of domain state. The shell supplies a visible heading,
subtitle, status slot, mode canvas, and persistent action boundary. It never
derives targets, counts, sizes, plans, digests, or status.

Use shared CSS tokens and focused components for card, grouped row, status
chip, evidence disclosure, target table, and action bar. Each mode may set its
accent/canvas variables but may not invent a second visual system.

## State rendering

- Clean maps reducer states to home, scanning, reviewed, dry-run/confirming,
  executing, reported, canceled, failed, and empty views. It passes selection
  and authority through existing callbacks unchanged.
- Software maps inventory, preview, uninstall, audit, cancellation, and
  manual-only states without turning display strings into commands.
- Optimize maps catalogue/list/preview/run/audit outcomes and refusal reasons.
- Analyze keeps the list/treemap/breadcrumb model and read-only semantics.
- Status keeps metric cards, charts, processes, and unavailable states.

## Responsive and accessibility contract

At widths below 800 CSS px, the same mode list becomes a horizontal overflow
strip. Data tables remain real tables; controls remain buttons, checkboxes,
selects, details, and dialogs. Focus-visible states, keyboard movement, live
progress announcements, reduced motion, and forced colors follow the existing
desktop spec. Native scaling is recorded separately and not simulated by CSS.

## Files and rollback

Primary files are `desktop/src/app-shell/`, `desktop/src/styles.css`, each
mode's `*.tsx`/`styles.css`, shared components, and their tests. Roll back UI
files as one child if a state/authority regression appears; do not revert core
or Tauri contracts to fix a visual issue.
