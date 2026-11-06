# Technical design

## Boundaries and approach

Trigger default entry from Session route transitions, not view rendering.
Preserve the existing generation-guarded BackgroundJob/JobResult flow.
A same-segment activation is a no-op; Refresh is the explicit reload path.
Keep last active segment across mode/Settings round trips. Segment transitions
clear pending review/confirmation before accepting new data; do not run winget
just because the top-level Software page is opened.

Reuse software_review and its bounded row window. Render header, one TabList,
segment-appropriate toolbar, row region and selected-item context. Apply existing
typed capability state when choosing actions, not optimistic introduction copy.
Do not broaden host/fixture execution support to satisfy a redesigned button.
Remove software_landing and its accessibility contracts; update consumers to
locate stable tab/row/action IDs after each transition.

Use remaining viewport for populated rows; compact meaningful empty states are
valid. Keep one scroll owner for rows and accessible selected-item details.
Search/sort/filter follow one convention across segments; preserve state across
ordinary route reentry, clear only values incompatible with a changed segment.

## Shared design contract

Follow ../11-06-gpui-usability-workbenches/design.md and the archived GPUI
airy-glass adaptation. ui.rs remains the token owner. Preserve high contrast,
localization, text scaling, stable accessibility roles and current domain safety.

## Compatibility and rollback

After footer/focus child; implement after Clean to reduce shared view.rs/shell.rs conflicts.
Keep this child's UI and Session changes together when reverting. Preserve
persisted user data. Do not delete unrelated pre-existing code or shared controls.
