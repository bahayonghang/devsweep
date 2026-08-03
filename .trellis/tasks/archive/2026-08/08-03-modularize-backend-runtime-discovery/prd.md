# Modularize backend runtime and discovery

## Goal

Move the large backend implementation clusters into cohesive private directory
modules while keeping their safety-critical interfaces and runtime behavior
stable for the application and TUI.

## Parent And Ordering

- Parent: `08-02-reorganize-rust-source-architecture`.
- Depends on completed child `08-03-untangle-core-contracts-rules` and its
  model, rule, plan, and Cargo metadata interfaces.
- Must pass `just ci` and be archived before the TUI child starts.

## Confirmed Problems

- `fs_size` owns both bounded sizing and platform reparse probing
  (`src/fs_size.rs:14`, `src/fs_size.rs:140`).
- `process_runner` owns cancellation, process policy, output capture,
  sanitization, platform tree termination, and test support in one file
  (`src/process_runner.rs:22` through `src/process_runner.rs:714`).
- `scanner` combines traversal, ecosystem discovery, Cargo workspace caching,
  normalization, dedupe, and reviewed rescan.
- `providers` combines production probing, command-backed providers, known
  cache paths, Cargo home inspection, and sizing.
- `executor` combines orchestration, adapters, authorization dispatch, durable
  audit, replay, and presentation helpers.
- `safety` combines authorization policy, protection persistence, path checks,
  and (before child 1) Cargo metadata probing.

## Requirements

- Create a filesystem module that privately owns path identity, no-follow
  reparse/symlink probing, containment/current-executable checks, and bounded
  cancelable size estimation.
- Create a process module that privately owns cancellation, request/result
  types, bounded output capture/sanitization, cwd policy, timeout, and
  Windows/Unix process-tree lifecycle.
- Create a scan module that owns the full project/global scan pipeline, its
  single ranking pass, project traversal/dedupe/rescan, and global provider
  probing/target construction.
- Keep inventory a separate read-only module and split bounded pnpm reference
  inspection from top-level observation orchestration.
- Create an execution module that keeps `Executor` as its interface while
  hiding command/trash adapters, authorization implementation, persistent user
  protections, durable audit journal, and replay.
- Preserve real test adapter seams. Do not create traits or wrapper modules that
  have only one meaningful adapter or only forward calls.
- Localize platform-specific unsafe code and document the safety invariant at
  each unsafe block retained or moved.
- Preserve all current plan, scan, inventory, execution, audit, cancellation,
  path-safety, and platform behavior.
- Use private or `pub(crate)` visibility; temporary internal re-exports must be
  removed by the parent final child.

## Acceptance Criteria

- [ ] Filesystem callers use one interface for identity, reparse safety,
      containment, and sizing; no duplicate path normalization is introduced.
- [ ] Process callers use `ProcessRunner` with separate program/argv, bounded
      output, neutral/explicit cwd policy, timeout, cancellation, and process
      tree cleanup unchanged.
- [ ] `Sweeper` remains the scan seam and the sole owner of merge/order and the
      single ranking pass; project/global implementations do not rank results.
- [ ] Inventory remains read-only, never creates `CleanupPlan`, and never calls
      execution.
- [ ] Every execution side effect still crosses `SafetyPolicy::authorize` and
      durable started-audit ordering remains fail-closed.
- [ ] Permanent delete remains rejected; Cargo home remains inspect-only;
      scanner/model code remains non-mutating.
- [ ] Windows and Unix platform branches compile under the existing all-target
      gate, and platform unsafe code is confined to focused implementation.
- [ ] Existing tests are moved to their owning modules without losing behavior
      coverage; focused interface tests replace layout-coupled tests when
      needed.
- [ ] Affected backend directory/quality/error/logging specs describe the final
      backend owners and paths.
- [ ] `just ci` passes.

## Out Of Scope

- TUI reducer/render decomposition, final CLI/application ownership, final
  crate-interface narrowing, new providers/rules, performance changes, and
  schema changes.
- Changing cleanup authorization, audit durability, provider commands, scan
  ordering, selection defaults, or cancellation semantics.
