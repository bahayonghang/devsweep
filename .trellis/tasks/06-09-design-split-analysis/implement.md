# Design.md split analysis implementation plan

## Planning Checklist

- [x] Create parent Trellis planning task.
- [x] Read root `design.md`.
- [x] Confirm repository evidence and Trellis workflow state.
- [x] Extract task tree, safety invariants, and MVP tension.
- [x] Write parent `prd.md`.
- [x] Write parent `design.md`.
- [x] Create child tasks now that Docker is explicitly deferred.
- [x] Write child PRDs with dependency and verification boundaries.
- [ ] Ask the user to review planning artifacts before any `task.py start`.

## Recommended Child Creation Commands

Run these only after the user accepts the split or asks to materialize child tasks:

```powershell
python .\.trellis\scripts\task.py create "Foundation CLI and domain model" --slug foundation-cli-domain-model --parent .trellis/tasks/06-09-design-split-analysis
python .\.trellis\scripts\task.py create "Project scanner and JSON plan" --slug project-scanner-json-plan --parent .trellis/tasks/06-09-design-split-analysis
python .\.trellis\scripts\task.py create "Execution engine and audit log" --slug execution-engine-audit --parent .trellis/tasks/06-09-design-split-analysis
python .\.trellis\scripts\task.py create "Global cache providers" --slug global-cache-providers --parent .trellis/tasks/06-09-design-split-analysis
python .\.trellis\scripts\task.py create "Ratatui app experience" --slug ratatui-app-experience --parent .trellis/tasks/06-09-design-split-analysis
python .\.trellis\scripts\task.py create "Safety hardening and release" --slug safety-hardening-release --parent .trellis/tasks/06-09-design-split-analysis
```

## Future Execution Order

1. Start `foundation-cli-domain-model`.
   - Verify: root `justfile` exposes `ci`, `build`, `dev`, and `test`; `just ci`, CLI help output, plan schema serialization test.
2. Start `project-scanner-json-plan`.
   - Verify: scanner fixture tests for Rust/Node/Python, no markerless `target`/`build` match, parent-child dedupe.
3. Start `execution-engine-audit`.
   - Verify: dry-run tests cannot mutate fixtures, `cargo clean` command plan is argv-based, trash action is isolated, audit JSONL covers success/failure.
4. Start `global-cache-providers`.
   - Verify: command discovery tests/mocks, official-command actions, Cargo home inspect-only behavior.
5. Start `ratatui-app-experience`.
   - Verify: ratatui TestBackend or smoke rendering, app update tests, worker event handling, no side effects in render path.
6. Start `safety-hardening-release`.
   - Verify: Windows path/lock/reparse tests where available, CI matrix, release packaging, README safety docs.

## Review Gates

- Do not start the parent task for implementation unless planning artifacts themselves need edits.
- Start only one child at a time.
- Before each child starts, read relevant `.trellis/spec` index files and root `design.md`.
- For implementation in Codex inline mode, load `trellis-before-dev` before editing.
- After code edits, load `trellis-check` and run the child's validation commands.

## Risky Files And Rollback Points

- `Cargo.toml` / dependency changes: verify with `cargo check` and keep versions scoped to the child.
- `justfile`: keep recipes as stable project entrypoints; update it whenever validation commands change.
- Domain model files: changes here affect all later children; update parent/child artifacts if contracts change.
- Executor files: rollback immediately if tests show cleanup can run without explicit execute/confirmation.
- TUI worker/render boundaries: rollback if render code performs IO, scanning, or cleanup.
- Release/CI files: keep separate from product implementation commits.

## Current Blocker

No current blocker. Docker builder cache is deferred out of MVP and may be revisited in `safety-hardening-release` or a later child task after the language-cache MVP is working.
