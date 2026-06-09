# Quality Guidelines

> Code quality standards for backend development.

---

## Overview

This project is a safety-first cleanup tool. Backend quality checks must prove
that planning and dry-run behavior stay non-mutating until an execution task
explicitly adds cleanup support.

---

## Forbidden Patterns

- Do not add file deletion, trash movement, or external cleanup command
  execution from scanner, plan, or foundation CLI code.
- Do not shell-compose commands. Future command-backed cleanup must keep
  program and argv separate.
- Do not make `clean --execute` perform side effects until the execution-engine
  task owns that behavior and its audit tests.

---

## Required Patterns

### Scenario: Foundation CLI and command entrypoints

#### 1. Scope / Trigger
- Trigger: the foundation task defines the project command surface and the
  local validation entrypoint used by later Trellis tasks.

#### 2. Signatures
- Local command entrypoints:
  - `just ci`
  - `just build`
  - `just dev`
  - `just test`
- CLI entrypoints:
  - `devsweep tui`
  - `devsweep scan [ROOT]... [--json] [--global] [--projects]`
  - `devsweep clean [--plan PATH] [--execute] [--allow-permanent-delete]`
  - `devsweep rules`

#### 3. Contracts
- `just ci` is the canonical local quality gate and must include formatting,
  type-checking, tests, and clippy.
- `devsweep scan --json` must emit the current JSON cleanup plan contract.
  During foundation it emits an empty plan:
  ```json
  {
    "version": 1,
    "targets": []
  }
  ```
- `devsweep clean` defaults to dry-run placeholder behavior.
- `devsweep clean --execute` must fail in the foundation build and must not run
  cleanup commands.

#### 4. Validation & Error Matrix
- `scan --json` succeeds -> valid JSON plan with `version` and `targets`.
- `clean --plan PATH` without `--execute` succeeds -> reports dry-run
  placeholder and performs no action.
- `clean --execute` in foundation -> returns an error explaining execution is
  not implemented.
- Missing future scanner/provider implementations -> return placeholder output,
  not side effects.

#### 5. Good/Base/Bad Cases
- Good: `just ci` passes before a task is reported complete.
- Base: `cargo check --all-targets` passes when run directly.
- Bad: `scan` discovers or deletes directories before the scanner task exists.
- Bad: `clean --execute` invokes `std::process::Command` before the execution
  engine and audit log are implemented.

#### 6. Tests Required
- CLI definition test for subcommand shape.
- Cleanup plan JSON serialization/round-trip tests for representative targets.
- TUI placeholder render test through ratatui `TestBackend`.
- Source scan or review confirming no deletion/trash/process execution code is
  present in foundation.

#### 7. Wrong vs Correct

Wrong:
```rust
// Scanner/foundation code must not execute cleanup commands.
std::process::Command::new("cargo").arg("clean").status()?;
```

Correct:
```rust
// Foundation only defines the serializable action contract.
CleanAction::Command {
    program: "cargo".to_string(),
    args: vec!["clean".to_string()],
    cwd: None,
    irreversible: true,
}
```

### Scenario: Project scanner and JSON cleanup plan

#### 1. Scope / Trigger
- Trigger: scanner code discovers project cleanup candidates and changes the
  `devsweep scan --json` output contract from an empty placeholder plan to real
  `CleanTarget` values.

#### 2. Signatures
- Library entrypoint:
  - `ProjectScanner::new().scan_roots(&[PathBuf]) -> anyhow::Result<CleanupPlan>`
- CLI entrypoint:
  - `devsweep scan [ROOT]... [--json] [--projects]`

#### 3. Contracts
- Scanner code may only create `CleanTarget` values; it must not delete, move to
  trash, or execute cleanup commands.
- Project matching is marker-first:
  - Rust `target` requires `Cargo.toml` evidence.
  - Node cleanup targets require `package.json` or lockfile evidence.
  - Python cleanup targets require project markers or local virtualenv evidence.
- `.gitignore` filtering must not hide cleanup candidates. The current scanner
  uses `std::fs` traversal and therefore does not apply ignore files.
- Symlinks, Windows junctions, and reparse points are not followed by default.
- JSON plan targets must include `risk`, `evidence`, `selected_by_default`,
  `action`, and `estimated_bytes`.
- Parent/child cleanup paths must be deduplicated so one plan never double-counts
  or emits duplicate cleanup actions for nested targets.

#### 4. Validation & Error Matrix
- Missing scan root -> return an error to the CLI.
- Markerless `target`, `build`, `dist`, or `node_modules` -> no target emitted.
- Inaccessible nested entry -> skip that entry, continue scanning the rest of
  the root.
- Nested cleanup target under an already selected cleanup path -> keep the
  parent target and drop the nested target.

#### 5. Good/Base/Bad Cases
- Good: fixture with `Cargo.toml` plus `target/` emits a Rust target with
  command-shaped `cargo clean` action data but does not run Cargo.
- Good: fixture with `package.json` plus `node_modules/` emits a medium-risk,
  not-selected-by-default dependency-directory target.
- Good: fixture with `pyproject.toml` plus `__pycache__/` emits a low-risk test
  cache target.
- Base: `devsweep scan . --json` emits valid plan JSON.
- Bad: matching a markerless directory because its name is `target`, `build`,
  or `dist`.
- Bad: scanner code importing `std::process::Command`, `trash`, or file removal
  APIs.

#### 6. Tests Required
- Fixture tests for Rust, Node, and Python discovery.
- Fixture tests proving markerless cleanup names are ignored.
- Tests that serialized plan targets contain risk, evidence, selection, action,
  and size fields.
- Dedupe tests for nested cleanup paths.
- Non-mutating test that scanned fixture files still exist after scan.

#### 7. Wrong vs Correct

Wrong:
```rust
// Name-only matching is unsafe.
if path.file_name() == Some("target".as_ref()) {
    targets.push(clean_target_for(path));
}
```

Correct:
```rust
// Marker-first: only a Rust project root can own target/.
let manifest = project_root.join("Cargo.toml");
if manifest.is_file() && project_root.join("target").is_dir() {
    targets.push(clean_target_with_marker(manifest));
}
```

---

## Testing Requirements

- Run `just ci` before reporting implementation complete.
- For command-surface changes, run the affected CLI help or command manually
  and record the result in the task summary.
- For JSON contract changes, include serialization or round-trip tests.

---

## Code Review Checklist

- Does the change keep scanner/plan/foundation code non-mutating?
- Does `just ci` pass?
- Are command signatures and JSON fields covered by tests?
- Is any new execution behavior owned by the correct child task?
