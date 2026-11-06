# Remove the footer and repair focus ownership

## Goal and parent mapping

Deliver R1 and R6 from ../11-06-gpui-usability-workbenches/prd.md.
This child is planning only; implementation awaits approval of the parent summary.

## Confirmed evidence

view.rs:188, :705-851, :2489-2535; input.rs:77; shared key bindings and source-string tests in view.rs.
Full causal analysis: ../11-06-gpui-usability-workbenches/research/audit.md.

## Requirements

Remove the entire bottom Ready/Working, keyboard-hint and Note strip.
Keep meaningful job status/errors in existing page/header context. Delete only
Note-specific entity state, copy, focus routing and tests; preserve TextInput,
buffer and search for their existing users. No substitute fixed bottom banner.

## Acceptance criteria

- No visible or UIA footer/Note remains on any mode or Settings/support page.
- No blank footer allocation, removed-entity focus or invisible tab stop.
- Startup, empty clicks, menu dismissal and route changes leave valid focus.
- Tab/Shift+Tab traverse real controls; Alt navigation and F10 work from shell
  and inputs; text editing/IME, Enter/Space/Escape retain their proper owners.
- Running jobs and failures remain discoverable after leaving their page.
- Search in Clean/Software/Optimize/Status remains functional.

## Dependencies

First implementation child. Other children build on this focus contract.

## Out of scope

No new cleanup authority, new providers, permanent delete, live cleanup tests,
Tauri redesign or unrelated refactoring. Parent exclusions and safety acceptance
apply. No code change is part of this planning delivery.
