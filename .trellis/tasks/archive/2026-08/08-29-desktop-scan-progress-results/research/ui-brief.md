# Active Scan UI Brief

## Job And Audience

The surface serves developers waiting on a local project/global cache scan. They
need to see that work is progressing, understand what has already been discovered,
and retain confidence that nothing can be cleaned before final review.

## Outcome And Proof

Success means the user can identify the active phase, read the current backend
message, inspect continuously arriving targets by scope/ecosystem, cancel safely,
and recognize that the rows are incomplete and read-only. The final Scan Report,
not the preview, is the proof that review and selection may begin.

## Selected Direction

- Keep the incumbent quiet Windows workbench and strengthen its hierarchy rather
  than replacing its visual world.
- Turn the scan toolbar into a stable control row plus a compact progress rail:
  requested phases show pending/current/complete state and an indeterminate bar
  belongs to the current phase.
- Replace the active-scan empty state with `Discovered so far`, grouped into
  Projects and Global caches with ecosystem counts and the existing dense target
  facts. Preview mode has no selection column or action footer.
- The focal transition is earned evidence appearing below the progress rail while
  the cancel control remains fixed; completion atomically changes the same area
  into the normal selectable review surface.

## Scope And Boundaries

This is a production-ready active-scan flow spanning `ScanPage`, the main workspace,
the shared target table, reducer/bridge contracts, responsive CSS, and controlled
fixtures. Preserve the app header, safe green/neutral palette, system typography,
dry-run/confirmation flow, and direct operational copy. Do not add hero layouts,
decorative cards, gradients, illustration, invented capacity claims, or a second
design system.

## States And Ranges

- Starting with zero discovered targets.
- Projects active; project targets arriving continuously.
- Projects complete and Global caches active; both pending and populated groups.
- Cancel requested while preview rows remain visible.
- Scan fails or cancels after partial evidence; evidence stays read-only.
- Scan completes empty, partial, or complete with results.
- Typical 800×600 window, narrow 390×844 fixture, large target lists, long paths,
  incomplete sizes, inspect-only targets, and reduced-motion mode.

## Interaction And Layout

Scope controls remain first, status occupies the flexible center, and Cancel keeps
a stable hit area. The progress rail uses semantic text and a labelled native
progress element; a polite live region announces the phase/message and a compact
count, not every row mutation. Scope sections and ecosystem counts supply
classification without forcing users through new controls. The table remains the
comparison surface and may scroll horizontally inside its own frame at narrow
widths; the page itself must not overflow.

## Constraints

React stays an untrusted presentation boundary, progress works without animation,
and preview rows cannot expose cleanup controls. No new production dependency is
approved. Final visual verification is bounded to one combined desktop/narrow pass,
one batch fix if needed, and one confirmation pass.
