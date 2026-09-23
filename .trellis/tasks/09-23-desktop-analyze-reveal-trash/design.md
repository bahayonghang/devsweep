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

## Rollback

Reveal and trash are separate commands; revert trash alone if safety tests
fail and keep reveal.
