# Technical design

## Boundaries and approach

Remove the final status-bar child from ShellView and the Note entity. Establish
shell/root fallback focus and a valid active-navigation target on transitions.
Remove the unconditional Note redirection in on_focus_lost and initialization.
Guard fallback against recursion and do not steal focus from editable controls.
Preserve action bubbling; migrate Note-dependent key contexts/tests to shell
and actual search fields. Keep shared TextInput constructors or simplify only
those made orphaned by this removal. AccessibleSlot::Note and its labels have no
remaining UI owner and must be removed with dependent tests.

Existing page job/error projections remain authoritative. Where an active job
can be hidden by navigation, put a compact indication in current header chrome.
No new toast queue or status service.

## Shared design contract

Follow ../11-06-gpui-usability-workbenches/design.md and the archived GPUI
airy-glass adaptation. ui.rs remains the token owner. Preserve high contrast,
localization, text scaling, stable accessibility roles and current domain safety.

## Compatibility and rollback

First implementation child. Other children build on this focus contract.
Keep this child's UI and Session changes together when reverting. Preserve
persisted user data. Do not delete unrelated pre-existing code or shared controls.
