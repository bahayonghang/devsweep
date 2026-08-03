# Entrypoint And Documentation Implementation Plan

## Prerequisite

- Core, backend, and TUI children are completed, committed, and archived.
- Start this child only after reading their final task artifacts and work SHAs.

## Ordered Checklist

- [ ] Inventory final `src/` paths, crate-level exports, temporary re-exports,
      command tests, and stale architecture references.
- [ ] Create `application/`; move CLI definitions, command dispatch/handlers,
      input decoding, tracing startup, and CLI formatting with tests.
- [ ] Replace `src/main.rs` with delegation to `devsweep::run()` and make
      `src/lib.rs` expose only that deliberate entrypoint.
- [ ] Reduce visibility across implementation modules based on actual final
      callers; delete all temporary compatibility exports.
- [ ] Run command/help/input tests and all-target check before documentation
      edits.
- [ ] Update `code_map.md` from the actual final tree.
- [ ] Reconcile affected backend/frontend Trellis specs, including any path or
      owner references not already updated by earlier children.
- [ ] Audit README for directly stale module/interface text; leave historical
      `design.md` unchanged.
- [ ] Generate public docs and inspect the exposed crate interface.
- [ ] Run structural searches, `git diff --check`, and final `just ci`.
- [ ] Review all four child commits against the parent acceptance criteria.
- [ ] Commit only this child, archive it, and record the session; then return to
      the parent for final integration/archive.

## Targeted Validation

```powershell
cargo test --locked application
cargo check --locked --all-targets
cargo doc --locked --no-deps
just ci
```

Structural/documentation checks:

```powershell
rg --files src
rg -n "^pub (mod|use) " src
rg -n "process_runner|path_identity|path_safety|fs_size|plan_validation|providers|scanner|executor" src code_map.md .trellis/spec
git diff --check
git status --short --untracked-files=all
```

Legacy-name matches in historical text or migration notes may remain only when
clearly labeled. Production imports and maintained current-layout guidance must
use the final owners.

## Final Review Checklist

- `main` delegates only; application owns commands.
- Generated docs expose only the intended entrypoint.
- CLI/TUI/JSON/audit/safety/platform behavior is unchanged.
- No compatibility shim or accidental public module remains.
- Actual source tree, code map, and backend/frontend specs agree.
- Every child has a passing final gate and scoped work commit.
- Parent acceptance criteria can be checked without unresolved evidence.

