# Software implementation evidence

## Delivered

- First Session transition into Software opens Uninstall once. Production queues
  read-only inventory; unbacked/fixture sessions retain isolated behavior.
- Removed duplicate landing cards and Back-to-introduction. A single semantic
  TabList owns stable accessible segment IDs and explicit UIA click actions.
- Same-tab activation is a no-op. Mode/Settings round trips retain the segment
  and query. Explicit Refresh reloads, including previously unavailable host
  reads; an in-flight refresh coalesces. Updates probing remains segment-owned.
- Segment changes clear pending authority, queued old Software loads and the
  old generation. Late in-flight completions are refused. Escape only addresses
  the active mode and dismisses a pending review without destroying Software.
- The shared input restores Software's model query without emitting an edit;
  its accessible label/placeholder reflects installed apps, updates or startup.
- Uninstall filters appear only for Uninstall; other segments cannot accidentally
  apply its Removable filter. Filtered-empty differs from a true empty inventory.
  Successfully checked empty updates no longer claim an unavailable probe.
- Rows scroll within a viewport-derived height. Row selection, sort, filter,
  refresh and review use explicit UIA adapters. The view does not retain obsolete
  controls for old automation handles; clients must reacquire them.
- Host capabilities are stated truthfully next to the list. Host uninstall and
  startup previews do not offer an executable Confirm button. Keyboard activation
  observes the same host-preview restriction. No executor authority changed.
- Removed newly orphaned software landing-row discovery and its env constant.
  The bounded-row selection test remains, using an in-memory 1000-row fixture.

## Files

Product changes: shell.rs, software_review.rs, view/page_workbench.rs, view.rs,
input.rs and workbench_review.rs. Previous footer/Settings/Clean edits preserved.

## Validation

- Final product source passed `just gpui-check` (format + clippy all targets).
- New Session tests cover one initial inventory load, same-tab/route no-op,
  stale result refusal, segment/query retention through Settings and other-mode
  Escape, no navigation IDs in empty results, explicit refresh, and Leftovers
  refusal without replacing the selected segment.
- Initial full tests exposed stale synthetic landing selection; corrected the
  transition to clear it and refuse activation of non-projected IDs.
- Updated old Escape expectations to persistent workspaces. A bounded-row test
  now permits its one legitimate offscreen-selection summary because the empty
  baseline no longer contains a synthetic uninstall selection.
- Final `just gpui-test` passed: 174 tests, 0 failures (2026-11-06).
  Captured output: `dist/software-gpui-test-final.log` (generated, untracked).
- Release build and native keyboard/UIA/layout evidence remain owned by main.

## Native checks still required

First entry should load only inventory, not winget. Verify same-tab no reload,
query retention across mode/Settings return, all three tabs, filter visibility,
filtered-empty recovery and keyboard/UIA activation. Verify list bounds and long
names at the parent size/font matrix. No live uninstall or startup mutation.
