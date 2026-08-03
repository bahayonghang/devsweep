# Backend Runtime And Discovery Implementation Plan

## Prerequisite

- `08-03-untangle-core-contracts-rules` is completed, committed, and archived.
- Start this child only after confirming its imported model/rules/plan/Cargo
  metadata interfaces are current.

## Ordered Checklist

- [ ] Record baseline focused tests for filesystem, process, scanner, sweep,
      inventory, executor, and safety behavior.
- [ ] Form `filesystem/`; move identity, reparse, sizing, and containment code
      with tests and update all callers.
- [ ] Confirm one path normalization/reparse implementation remains and run
      filesystem/validation/safety tests.
- [ ] Form `process/`; move cancel, capture/sanitization, runner, and native
      process-tree implementation with dynamic fixture tests.
- [ ] Form `scan/project/` and `scan/global/`; move implementation by
      responsibility while keeping targets unranked and non-mutating.
- [ ] Move sweep/ranking under `scan/`; verify cumulative progress and exactly
      one ranking pass.
- [ ] Form `inventory/`; isolate bounded pnpm reference inspection and retain
      read-only report semantics.
- [ ] Form `execution/`; move command/trash adapters, audit journal/replay, and
      safety/protection implementation behind `Executor`.
- [ ] Remove obsolete flat modules/imports and narrow internal visibility.
- [ ] Update affected backend Trellis specs for the implemented filesystem,
      process, scan, inventory, execution, safety, and audit owners.
- [ ] Inspect every moved unsafe block and its owning type/lifetime invariant.
- [ ] Run focused tests and `just ci`; inspect the diff for behavior changes.
- [ ] Commit only this child, archive it, and record the session before child 3.

## Targeted Validation

```powershell
cargo test --locked filesystem
cargo test --locked process
cargo test --locked scan
cargo test --locked inventory
cargo test --locked execution
just ci
```

Use actual post-move module filters if names differ. Additionally search:

```powershell
rg -n "std::process::Command|cmd /C|sh -c|taskkill|\bkill\b" src
rg -n "remove_file|remove_dir|remove_dir_all|trash::" src
rg -n "unsafe" src
```

Every process/filesystem mutation match must belong to the approved process,
execution, atomic protection persistence, test fixture, or test code path.

## Review Checklist

- Program/argv are separate at every command seam.
- Scan, inventory, model, plan, and rules contain no cleanup side effects.
- `SafetyPolicy::authorize` remains mandatory and fail-closed.
- Durable audit ordering and replay formats are unchanged.
- Cancellation reaches in-flight process termination and prevents later work.
- Cargo home is inspect-only and permanent delete is rejected.
- No helper-only module or one-adapter trait was added to satisfy the tree.
