# Design — Analyze reveal and trash

## Snapshot retention

`AnalyzeCoordinator` (`desktop/src-tauri/src/analyze.rs:45`) retains the last
completed `AnalyzeSnapshotV1` keyed by `operation_id` (one slot; a new run
replaces it). Reveal and trash requests must name that `operation_id`;
otherwise `stale_operation`.

## Core

- `analysis/actions.rs`
  - `node_path(snapshot, node_id) -> Result<PathBuf>`: join
    `root.normalized` with names along `parent_id` links; reject cycles and
    unknown ids.
  - `analyze_trash_preview(snapshot, node_ids, protection) -> AnalyzeTrashPreviewV1 { plan, digest, refused: Vec<{ node_id, reason_code }> }`.
    Refusal codes: `protected`, `system_location`, `volume_root`,
    `profile_root`, `analysis_root`, `reparse_point`, `changed_since_snapshot`,
    `not_found`. Re-stat each path with the existing single-open reparse check
    (`filesystem`).
  - The plan reuses the Clean plan types (`CleanAction::MoveToTrash`) and
    `plan/digest.rs`, so `plan_execute` validation and audit apply unchanged.
    Target kind: add no new `TargetKind`; the plan origin is recorded as
    `analyze` in the plan metadata field used for audit domain labelling (add
    one enum value if no such field exists; state this in the spec update).
  - `reveal_in_explorer(path, runner)`: program `explorer.exe`, argv
    `["/select,", path]`, short timeout; `explorer.exe` returns exit code 1 on
    success, so the result checks only for spawn failure.
- Nested selection: if a selected node is an ancestor of another selected
  node, keep only the ancestor.

## Tauri

Commands `analyze_reveal(operation_id, node_id)`,
`analyze_trash_preview(operation_id, node_ids)`,
`analyze_trash_execute(operation_id, plan, digest, confirmed)`. Execute runs
through the frontend `OperationCoordinator` as kind `analyze.trash`.

## Desktop

- `modes/analyze/ContextMenu.tsx`: `role=menu`, positioned at pointer or at
  the focused element for keyboard. Items disabled with reason text for
  refused nodes (reason from preview only after the user picks Move; before
  that the menu uses a client-side hint from the node kind and depth, which is
  not authority).
- Trash flow: preview `DetailView` (list with sizes and refusals) →
  `ConfirmDialog` second confirmation → execute → mark moved nodes.
- `state.ts`: add `movedIds` and a projection that subtracts moved bytes from
  ancestors for display only.

## Spec updates

- Desktop spec and backend spec: replace "Analyze is read-only" with the R4
  boundary. `docs/safety-capability-matrix.md`: add "Analyze → Recycle Bin".

## Implementation note

The implementation differs from the design above in these points:

- The preview (`AnalyzeTrashPreviewV1`) carries `items` (node id, target id,
  path, kind, bytes, evidence), `refused`, and `digest`. It carries no plan.
  `analyze_trash_execute` takes `(operation_id, node_ids, digest, confirmed)`.
  Core rebuilds the plan from the retained snapshot and the live file system,
  and the Clean executor rejects the call when the live digest differs. The
  webview therefore never sends a plan or a path.
- No audit origin field is added. The execution writes Clean V1 audit records
  (`domain: "clean"`). The target id `analyze.trash:<operation_id>:<node_id>`
  and the rule id `analyze.trash` identify the origin. Plan files cannot carry
  the `analyze.trash` rule id.
- Targets use `Scope::Global`, `TargetKind::ToolCache` as a placeholder kind,
  risk `High`, and `MoveToTrash`. No new `TargetKind` is added.
- The Tauri adapter retains a canceled snapshot as well as a completed one.
  A new run clears the retained snapshot. A mismatch returns the new
  `analyze_stale_operation` command error.
- A stale digest keeps the Clean `stale_confirmation` error shape because the
  Clean executor detects it.
- One extra command, `analyze_default_root`, gives the system drive for R1.
  The shipped command count is 40.
- `profile_root` also covers the Desktop, Documents, and Downloads folders and
  any folder that contains the user profile. `protected` also covers the Clean
  protected profile subtrees and any folder that contains the running
  executable.
- The shell subtitle for Analyze still says the mode is read-only. The key is
  in the frozen `shell.v1` set that the CLI and desktop tests check, and the
  TUI Analyze mode stays read-only. The subtitle was not changed.

## Rollback

Reveal and trash are separate commands; revert trash alone if safety tests
fail and keep reveal.
