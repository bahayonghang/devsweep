# Analyze parity: reveal in Explorer and confirmed move to trash

## Goal

Let the user act on what Analyze finds, like the Mole desktop Analyze view:
open a context menu on a treemap tile or list row, show the item in Windows
Explorer, or move a user-owned item to the Recycle Bin after a preview and a
second confirmation. System locations stay view-only.

## User decision (2026-09-23)

The user selected "reveal in Explorer + move to Recycle Bin after
confirmation". This replaces the previous "Analyze is read-only" contract
with "Analyze is read-only by default; the only mutation is a Recycle Bin
move through a core-built plan, digest, and second confirmation".

## Dependency and boundary

- Parent: `09-20-desktop-mole-tauri-performance`.
- Starts after `09-23-desktop-mole-capsule-shell`.
- Changes `devsweep-core` (analysis → plan bridge), Tauri analyze adapter,
  generated wire types, `desktop/src/modes/analyze/`, and the backend and
  desktop specs that state Analyze is read-only.

## Requirements

- R1 Stage: the Analyze first screen shows the planet, the selected root
  (default: system drive), and the Analyze action. After completion the stage
  shows the accounted total and opens the treemap/list detail view.
- R2 Context menu: right-click, Shift+F10, or the context-menu key on a tile
  or row opens a menu with "Show in Explorer" and "Move to Recycle Bin".
  Double-click or Enter on a directory keeps drilling down; the breadcrumb
  path bar keeps returning to any level.
- R3 Show in Explorer: core runs `explorer.exe` with argv
  `["/select,", <path>]` through `ProcessRunner` (program and argv separate,
  no shell). The UI sends only `(operation_id, node_id)`; core rebuilds the
  path from the retained snapshot. The call is allowed for every node.
- R4 Move to Recycle Bin: the UI sends `(operation_id, node_ids)`. Core
  rebuilds each path from the retained snapshot, then refuses nodes that are:
  on the protection list; under `%WINDIR%`, `%PROGRAMFILES%`,
  `%PROGRAMFILES(X86)%`, `%PROGRAMDATA%`, or a volume root; the user profile
  root itself; a reparse point; changed since the snapshot (type or mtime
  mismatch); or the analysis root. Accepted nodes become a plan of
  `MoveToTrash` actions with a preview digest. The UI shows the preview
  (paths, sizes, refusals with reasons) and requires the existing second
  confirmation. Execution uses the Clean execution/audit path. Refused nodes
  show a "view only" label in the menu (menu item disabled with the reason).
- R5 After a move, the affected nodes are marked "moved" in the current view
  and their bytes are subtracted from ancestors in the UI projection only;
  the stage suggests a re-analyze for exact totals.

## Acceptance Criteria

- [ ] AC1: The UI never sends a path string for reveal or trash; a bridge test
      proves only ids cross the boundary.
- [ ] AC2: Core tests cover each refusal class in R4 and prove an accepted
      node yields a `MoveToTrash` plan with a digest; a stale digest or a
      snapshot/operation mismatch is refused.
- [ ] AC3: Reveal uses `ProcessRunner` with program `explorer.exe` and argv
      `["/select,", path]`; no shell string exists in the adapter.
- [ ] AC4: Keyboard access: context menu opens from Shift+F10 and the menu
      key, Escape closes, focus returns to the tile/row.
- [ ] AC5: Execution writes Clean audit records and never permanently deletes.
- [ ] AC6: Specs state the new Analyze mutation boundary; the safety
      capability matrix has the new row.
- [ ] AC7: `just ci`, desktop gates, and `types:generate -- --check` pass.

## Out of scope

- Permanent delete, emptying the Recycle Bin, moving items to other
  locations, or batch actions across different analysis runs.
- Opening files with their default application.
