# GPUI usability and operational workbenches

## Goal and value

Make the Windows GPUI desktop useful on first entry. Remove inert shell chrome,
expose real font preferences and project information, and bring Clean and
Software operations into the workspace instead of hiding them behind introductions.

The user explicitly requested task creation and confirmed planning, then approved
implementation on 2026-11-06: "按照规范和规划开始实施". Product changes and
automated checks are complete; native evidence is tracked in
research/verification.md. On 2026-11-06 the user explicitly narrowed acceptance
to changed core functionality and rejected further peripheral window/display
matrix testing. Core delivery was accepted for commit and archive on 2026-11-06. Product commit: 4fc8a64.

## Confirmed background

The worktree was clean on branch dev at 63a4245. Three user screenshots show
Settings, Clean landing and Software landing. They are visual evidence, not
instructions. Source inspection confirms the problems; see research/audit.md for
anchors, capability constraints and evidence limits.

The current GPUI visual language is airy glass, documented in the archived
11-04-gpui-airy-glass task. Root DESIGN.md is historical product architecture,
not the GPUI token specification. Neither source is replaced in this task.

## Requirements and ownership

| ID | Requirement | Owning child |
| --- | --- | --- |
| R1 | Remove the entire bottom status/Note strip, preserving useful status and keyboard operation | ../11-06-gpui-shell-footer |
| R2 | Enumerate actual local system fonts, select and preview them, explicitly save them and restore them on restart | ../11-06-gpui-settings-fonts-about |
| R3 | Add About within Settings with accurate project URL, GPUI build version and license | ../11-06-gpui-settings-fonts-about |
| R4 | Replace Clean's introductory gateway with scope-first scan controls and consistent review/result flow | ../11-06-gpui-clean-workspace |
| R5 | Replace Software's duplicate gateways with one segmented operational workspace | ../11-06-gpui-software-workspace |
| R6 | Integrate keyboard, accessibility, localization, density and safety acceptance across the changed surfaces | Parent integration gate |

## Acceptance criteria

- AC1 / R1: No status-bar, Note input, dead tab stop or reserved footer height
  exists on any page. Startup, menu dismissal and route changes leave a valid
  focus target. Existing search editing, IME, Alt+Left/Right, F10, Tab,
  Shift+Tab, Enter/Space and Escape remain usable.
- AC2 / R2: Settings shows System default plus real installed family names,
  searchable with bounded visible rows. Loading, empty, error and missing saved
  family states are distinguishable. No font download or filesystem font scan.
- AC3 / R2: Selection immediately previews in the GPUI session; only Save writes.
  The chosen family affects normal text, lists and input text/placeholder/IME.
  Restart restores saved choice; missing families fall back without erasing the
  preference. Invalid/future preference bytes and unrelated fields survive.
- AC4 / R3: About shows DevSweep, accurately labelled GPUI package/build version,
  MIT license and the canonical repository link. Link activation works with mouse
  and keyboard; no claim that the independent GPUI version equals CLI version.
- AC5 / R4: First Clean entry exposes Project and Global scope choices and the
  corresponding action without an introductory Start screen or decorative Earth.
  Selecting scope does not start scanning. Project roots are explicit; Global
  retains explicit scope confirmation. Opening the page never starts a scan.
- AC6 / R5: First Software entry selects Uninstall and starts only read-only
  inventory. Three segment tabs are the only segment navigation. No same-purpose
  cards or Back-to-introduction layer. Refresh, search, sort, applicable filters
  and contextual review actions are reachable without traversing an introduction.
- AC7 / R4,R5: Loading, no data, filtered-empty, unavailable, error, review,
  confirmation and result are distinct where applicable. Changing scope,
  selection or data invalidates stale authority; stale jobs cannot replace a
  newer view. Switching a tab never executes a destructive operation.
- AC8 / R6 (revised by user): In the normal working window, verify that changed
  Clean/Software controls and first result rows are accessible and that Settings
  font selection works. Do not enlarge empty cards just to fill height. Do not
  require additional resize/maximize, exact viewport or exhaustive display
  matrices unless a concrete related defect appears.
- AC9 / R6: Clean and Software retain their existing backend authority, preview,
  digest, second confirmation, cancel/join and honest capacity semantics. No
  permanent delete, Docker cleanup, automatic cleanup or new uninstall authority.
- AC10 / R6: Regression tests and native Windows evidence accompany
  implementation; source projection/string tests alone cannot prove rendering
  or UIA behavior. The native GPUI gate is separate from root just ci.

## Proposed UX decisions for implementation review

Remove the whole footer. Keep the current airy-glass palette/material and top
navigation. Use compact page headers and solid list surfaces. Clean starts with
Project selected but no scan; switching scope changes configuration only.
Software defaults to Uninstall, preserves the active segment across mode/Settings
round trips, and refreshes only on first load, segment change or explicit Refresh.
Updates probing is deferred until its segment is opened. Font editing follows the
existing GPUI draft-plus-Save model. About is a Settings section, not a sixth mode.

These choices were approved and implemented; see research/review.md.

## Out of scope

No new cleanup providers,
whole-app redesign, Tauri UI rewrite, update installer, font manager, telemetry,
new preferences schema, cross-process live synchronization, automatic update
checker, version bump or migration of the historical root design document.
Optimize/Analyze/Status change only as necessary to preserve shell focus/status
and global typography. Preserve shared search infrastructure.

## Risks and deferred technical checks

The pinned public GPUI font API has been verified and used. Native rendering,
UIA text replacement, paste, font preview/save/restart and read-only workspaces
have execution evidence. Exact viewport, real IME candidate and OS high-contrast
coverage is not claimed and is not a blocker under the user's revised scope.
