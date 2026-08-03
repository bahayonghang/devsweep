# Backend Runtime And Discovery Design

## Target Modules

```text
filesystem/
  mod.rs          narrow internal interface
  identity.rs     lexical normalization and live file identity
  reparse.rs      no-follow symlink/reparse probe and platform implementation
  sizing.rs       bounded parallel walk and SizeEstimate
  containment.rs  shared containment/current-executable helpers

process/
  mod.rs          ProcessRunner and policy/request/result interface
  cancel.rs       cancel observers and shared flag
  capture.rs      bounded stream capture and output sanitization
  tree.rs         Windows Job Object / Unix process-group implementation

scan/
  mod.rs          Sweeper, options, progress, staged merge/rank
  ranking.rs      pure ranking/default-selection policy
  project/        traversal, Rust cache, dedupe, reviewed rescan
  global/         provider probe adapter and target construction

inventory/
  mod.rs          read-only report and top-level inventory orchestration
  pnpm.rs         bounded reference scan and orphan finding

execution/
  mod.rs          Executor request/report/progress and orchestration
  command.rs      ProcessRunner command adapter and trash adapter
  audit.rs        durable JSONL journal and replay
  safety/
    mod.rs        authorization and live revalidation
    protections.rs  versioned atomic protection persistence
```

Subfiles may be coalesced when they would contain only a re-export or one small
forwarding function. Public interfaces and ownership below are mandatory.

## Filesystem Interface

The filesystem module owns implementation currently spread across
`path_identity`, `path_safety`, and `fs_size`. Callers receive typed results and
cannot bypass no-follow checks by directly using platform helpers.

The reparse probe trait remains an internal seam because the system adapter and
injected Cloud Files/symlink test adapters both exercise safety behavior. Size
walking retains lower-bound/unknown semantics, bounded traversal, cancellation,
parallel execution, latest mtime, and fail-closed root probing.

No normalization algorithm is duplicated during the move. Plan validation,
rules, scan dedupe, inventory, and execution import the same normalized identity
and containment operations.

## Process Interface

`ProcessRunner` remains the interface. Capture rings, reader threads, neutral
temporary cwd cleanup, native handles, job/process-group setup, termination,
and liveness probes remain private.

`CancelObserver` is a real internal seam with no-op, shared flag, and test
adapters. `ProcessRequest` continues to keep program and argv separate. The
module never invokes a shell or external kill utility.

Windows unsafe operations move with their owned handle types and safety
comments. Unix process-group unsafe calls remain in the Unix implementation.

## Scan Interface

Application and TUI scan through `Sweeper`. It owns:

1. project phase;
2. global phase;
3. cumulative progress snapshots;
4. merge and health aggregation;
5. one ranking/default-selection pass;
6. report projection and reviewed target rescan.

Project implementation owns marker-first traversal, ignore behavior, root
normalization, Cargo workspace reuse, target dedupe, and rescan. Global
implementation owns tool/environment probing and target construction. Both
return unranked observed targets and cannot execute cleanup.

Existing test traits for project/global scan adapters remain internal to the
scan implementation because production and fake adapters both use them.

## Inventory Interface

The external seam remains `inventory_root` / cancelable inventory returning an
`InventoryReport`. pnpm reference parsing is private implementation. Inventory
may call filesystem and process but cannot import plan validation or execution,
and its types contain no intent/action/selection fields.

## Execution Interface

Application and TUI use `Executor` plus typed request/report/progress values.
Everything else is implementation:

- live `SafetyPolicy` authorization;
- user protection persistence;
- ProcessRunner command adapter;
- trash adapter;
- once-ledger and explicit selection intersection;
- durable started/terminal audit events;
- unconfirmed-start replay.

Command and trash adapter traits remain internal seams because system and fake
adapters are both used. The authorization funnel remains immediately before
the durable-start/side-effect sequence. Audit failure before the side effect
continues to block dispatch; terminal audit failure continues to produce an
unknown result where currently specified.

## Migration Order

1. Move filesystem implementation and update callers without changing caller
   ownership.
2. Move process implementation and update providers, Cargo metadata, inventory,
   execution, and TUI runtime imports.
3. Form scan project/global directories and then move sweep/ranking under the
   scan interface.
4. Split inventory pnpm implementation.
5. Form execution directory, then move audit and safety/protections inside it.
6. Remove obsolete files and reduce crate-private visibility.

Run focused tests after each cluster; run `just ci` before committing the child.

## Risks And Rollback

- Platform code can compile on only one local OS. Preserve all `cfg` branches
  mechanically and rely on existing all-target checks plus CI parity.
- Moving path helpers can accidentally introduce two normalizers. Search all
  old helper names and keep one authoritative implementation.
- Moving execution can reorder authorization/audit/dispatch. Review the exact
  call sequence and existing failure-injection tests.
- Moving scan can rank partial results twice. Keep ranking in the Sweeper seam
  and assert staged progress tests.

Rollback is the child work commit; do not add fallback wrappers after a failed
move.

