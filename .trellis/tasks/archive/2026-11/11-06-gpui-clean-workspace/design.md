# Technical design

## Boundaries and approach

Reuse ui.rs tokens and existing controls inside one sheet. Remove only Clean's
decorative Earth placement, not shared assets used elsewhere. Introduce/pass
through pending scope as presentation state; selecting Project cannot call the
existing clean_choose_scope directly because it can start a scan. Only explicit
Scan dispatches the appropriate validated Session path. Preserve Global's armed
confirmation check, explicit root validation and background-job generation.

Compact heading plus secondary AI action precedes scope tabs and configuration.
Use existing project-folder picker. On results keep current scope context and
put search/filter, rows and evidence within the workspace. Wide layout may keep
details adjacent; narrow layout stacks them without hiding confirmation.
Retain a bounded list and intentional scroll owner.

Do not reset existing results merely by rendering. Any committed replacement
scope invalidates preview before reuse; busy work must be cancelled/joined per
current lifecycle before replacement. Do not weaken inspect-only row restrictions.

## Shared design contract

Follow ../11-06-gpui-usability-workbenches/design.md and the archived GPUI
airy-glass adaptation. ui.rs remains the token owner. Preserve high contrast,
localization, text scaling, stable accessibility roles and current domain safety.

## Compatibility and rollback

After footer/focus child; font child recommended before final visual evidence.
Keep this child's UI and Session changes together when reverting. Preserve
persisted user data. Do not delete unrelated pre-existing code or shared controls.
