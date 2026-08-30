# Build Analyze TUI and desktop treemap

## Goal

Own accessible hierarchical browsing, zero-dependency treemap layout, bilingual presentation, responsive interaction, and native visual evidence for Analyze.

## Requirements

- R1: Render the frozen Analyze snapshot as a synchronized directory list and
  accessible hierarchy. Desktop also provides a zero-dependency treemap of the
  current directory; TUI provides proportional bars and the same breadcrumbs.
- R2: Treemap rectangles are deterministic, derived only from available child
  lower bounds, render at most 512 child tiles plus one aggregated `Other` tile,
  and never imply omitted/unknown capacity. The accessible list pages at 200
  rows and remains the complete route to every represented node.
- R3: Support keyboard drill-down/up, breadcrumb navigation, search/sort, focus
  restoration, resize, reduced motion, long paths, and bilingual available/
  partial/unknown/permission/unsupported states. No select/delete affordance.
- R4: Cancel/join Analyze when leaving the mode and reject stale IPC events.
- R5: On `analysis-250k-v1` with a 10,000-child current directory, treemap layout
  returns <=513 rectangles, rendered DOM stays <=900 elements, and after five
  warm-ups 30 measured navigations have p95 layout <=50 ms and p95 React commit
  <=100 ms on the recorded release-build host.

## Acceptance Criteria

- [ ] AC1 (R1, R2): Deterministic layout tests cover zero/equal/extreme sizes, rounding,
      tiny rectangles, `Other`, unknown nodes, resize, and snapshot parity.
- [ ] AC2 (R1, R2, R3): Accessibility tests prove list alternative, logical focus order,
      keyboard drill-down/up, labelled rectangles, contrast, reduced motion, and
      no cleanup action.
- [ ] AC3 (R3, R5): Both languages pass empty/loading/canceling/partial/error/complete
      states at target widths and native scaling; large snapshots stay within the
      documented render budget.
- [ ] AC4 (R4, R5): Frontend/TUI tests, desktop build, native visual evidence, and relevant
      `just ci` gates pass.

## Out of Scope

- Filesystem reads, cleanup selection/execution, WebGL/canvas-only interaction,
  photo planets, persistent snapshots, or hidden synthetic capacity.
