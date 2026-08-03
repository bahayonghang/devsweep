# Parent Implementation Plan

## Execution Model

This parent task owns requirements, ordering, and final integration review. It
does not directly own product-code edits. Implement and archive one child at a
time so every architectural change has an independently passing rollback point.

## Ordered Checklist

- [ ] Start `08-03-untangle-core-contracts-rules` after this parent plan is
      approved; complete its core dependency and rule-ownership work.
- [ ] Run the child 1 targeted checks and `just ci`; inspect its diff for JSON,
      action, and safety drift; commit and archive it.
- [ ] Start `08-03-modularize-backend-runtime-discovery`; split backend
      implementation clusters behind the interfaces established by child 1.
- [ ] Run the child 2 targeted checks and `just ci`; inspect unsafe/platform,
      path-safety, audit, and cancellation moves; commit and archive it.
- [ ] Start `08-03-decompose-tui-internals`; split reducer/runtime/render
      implementation without changing the event/effect protocol or user flows.
- [ ] Run state/runtime/render tests plus `just ci`; inspect the child 3 diff;
      commit and archive it.
- [ ] Start `08-03-narrow-entrypoint-architecture-docs`; reduce the crate
      interface to `devsweep::run()`, reconcile architecture docs/specs, and
      remove temporary compatibility exports.
- [ ] Run the child 4 targeted checks and final `just ci`; verify the documented
      tree and actual `src/` tree agree; commit and archive it.
- [ ] Re-open the parent acceptance criteria and perform a cross-child diff/log
      review. Confirm all child work commits are present and no source work is
      left under the parent.
- [ ] Archive the parent and record the Trellis session only after all children
      and final integration acceptance are complete.

## Validation Gates

Run after every child:

```powershell
cargo fmt --all -- --check
cargo check --locked --all-targets
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
```

`just ci` is the canonical combined gate and must be the final gate for each
child and for the integrated tree.

Final structural checks:

```powershell
rg --files src
rg -n "^pub mod |^pub use " src/lib.rs src
rg -n "use crate::(scanner|providers|safety|process_runner|path_identity|fs_size)" src
git status --short --untracked-files=all
```

The legacy-name search is diagnostic: any remaining match must be an intentional
identifier or documentation reference, not an obsolete module dependency.

## Review Gates

- Serialized DTO derives, version constants, field names, and digest inputs are
  reviewed before accepting child 1.
- Unsafe blocks and platform `cfg` branches are reviewed before accepting child
  2.
- TUI behavior tests and immutable render paths are reviewed before accepting
  child 3.
- Public exports, command help/output, `code_map.md`, and Trellis specs are
  reviewed before accepting child 4.

## Rollback Points

- Each child work commit is a rollback point.
- Do not mix unrelated cleanup or dependency upgrades into these commits.
- If an intermediate compatibility re-export is needed to keep a child
  buildable, mark it with an implementation checklist item and delete it in
  child 4. Do not preserve it as a permanent fallback.

