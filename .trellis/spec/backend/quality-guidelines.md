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
