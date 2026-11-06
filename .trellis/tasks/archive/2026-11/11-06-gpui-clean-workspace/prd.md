# Scope-first Clean workspace

## Goal and parent mapping

Deliver R4 and R6 from ../11-06-gpui-usability-workbenches/prd.md.
This child is planning only; implementation awaits approval of the parent summary.

## Confirmed evidence

view/page_clean.rs:147-390; shell.rs:2468-2491 and :2976 onward; clean_workbench.rs:583 onward.
Full causal analysis: ../11-06-gpui-usability-workbenches/research/audit.md.

## Requirements

Replace the introductory Clean landing and Earth with direct Project/Global
scope controls, selected scope details and explicit scan actions. Preserve the
existing scan/review/preview/confirmation/result workflow and AI tools route.

## Acceptance criteria

- First entry exposes both scope options, current selection, selected project
  roots or global scope explanation and corresponding explicit action.
- Project is initially selected without scanning. Scope changes do not scan.
  Missing roots provides Choose folders; never fall back to cwd/home implicitly.
- Global scan retains explicit scope confirmation; no scan on route reentry.
- Header, scope and action fit initial viewport; populated target list receives
  remaining room; no hero spacer or enlarged empty card.
- All scan/cancel, found, no-target, failed/retry, preview and result states are
  visible and consistent; AI return restores meaningful focus.
- Scope/selection/rescan invalidates old authority. Cleanup still requires
  current preview digest and second confirmation. No real cleanup in tests.

## Dependencies

After footer/focus child; font child recommended before final visual evidence.

## Out of scope

No new cleanup authority, new providers, permanent delete, live cleanup tests,
Tauri redesign or unrelated refactoring. Parent exclusions and safety acceptance
apply. No code change is part of this planning delivery.
