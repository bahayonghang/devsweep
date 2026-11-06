# Single segmented Software workspace

## Goal and parent mapping

Deliver R5 and R6 from ../11-06-gpui-usability-workbenches/prd.md.
This child is planning only; implementation awaits approval of the parent summary.

## Confirmed evidence

view/page_workbench.rs:10, :106, :118 and :243 onward; shell.rs:1556-1589, :1803-1830 and :2131; software_review.rs host loaders; workbench.rs Leftovers refusal.
Full causal analysis: ../11-06-gpui-usability-workbenches/research/audit.md.

## Requirements

Remove duplicate landing cards and Back-to-introduction. Default to an actual
Uninstall inventory workspace with single segment navigation and consistent
read-only loading, filtering, selection, contextual review and capability copy.

## Acceptance criteria

- First entry selects Uninstall and loads read-only inventory once; no repeated
  load on render. Reentry retains segment/state; Refresh explicitly reloads.
- Tabs are the sole segment destination controls and have correct selected,
  focus and UIA semantics. No duplicate compact cards after selection.
- Updates/Startup load on explicit segment entry/refresh; switching invalidates
  pending authority and stale responses cannot overwrite the current segment.
- Search and sort are usable; filters and placeholder fit the active segment.
  Uninstall-only Removable filters are not shown indiscriminately.
- Loading, no data, filtered-empty, unavailable and failed states differ;
  long names and qualified/unknown sizes remain truthful.
- Row click selects; explicit review prepares action and explicit confirmation
  dispatches only existing supported operations. No new executor capability.
- Capability explanations sit by relevant actions; no ordinary Leftovers
  disclaimer, while Leftovers requests remain refused in the model.
- Native UIA tests reacquire current controls after transitions instead of
  requiring duplicate cards to stay mounted.

## Dependencies

After footer/focus child; implement after Clean to reduce shared view.rs/shell.rs conflicts.

## Out of scope

No new cleanup authority, new providers, permanent delete, live cleanup tests,
Tauri redesign or unrelated refactoring. Parent exclusions and safety acceptance
apply. No code change is part of this planning delivery.
