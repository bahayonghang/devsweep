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
  execution from scanner or plan code.
- Do not shell-compose commands. Command-backed cleanup must keep program and
  argv separate.
- Do not run permanent delete in the current implementation, even when
  `--allow-permanent-delete` is present.

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
- `devsweep clean` defaults to dry-run behavior.
- `devsweep clean --execute` is owned by `src/executor.rs` after the execution
  engine task.

#### 4. Validation & Error Matrix
- `scan --json` succeeds -> valid JSON plan with `version` and `targets`.
- `clean --plan PATH` without `--execute` succeeds -> reports dry-run and
  performs no action.
- `clean --execute` without `--plan PATH` -> returns an error before execution
  starts.
- Missing future scanner/provider implementations -> return placeholder output,
  not side effects.

#### 5. Good/Base/Bad Cases
- Good: `just ci` passes before a task is reported complete.
- Base: `cargo check --all-targets` passes when run directly.
- Bad: `scan` discovers or deletes directories before the scanner task exists.
- Bad: `clean --execute` invokes `std::process::Command` outside
  `src/executor.rs`.

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

### Scenario: Execution engine and audit log

#### 1. Scope / Trigger
- Trigger: `devsweep clean` consumes an existing cleanup plan and either
  dry-runs selected actions or executes command/trash-backed actions with an
  audit JSONL record for each attempted target.

#### 2. Signatures
- CLI entrypoint:
  - `devsweep clean [--plan PATH] [--execute] [--audit-log PATH] [--allow-permanent-delete]`
- Library entrypoint:
  - `Executor::default().run_plan(&CleanupPlan, ExecutionRequest) -> anyhow::Result<ExecutionReport>`

#### 3. Contracts
- `clean` without `--execute` is always dry-run and must not call command or
  trash runners.
- `clean --execute` requires `--plan PATH`; execution must never discover new
  targets.
- The current CLI executes only targets where `selected_by_default` is true.
- Command actions use `CommandRequest { program, args, cwd }`; do not combine
  user-controlled values into a shell string.
- `MoveToTrash` actions pass only the path stored in the plan to the trash
  runner.
- `DeletePermanently` is disabled in this build, including when
  `--allow-permanent-delete` is set.
- Execution appends JSONL audit records to `--audit-log PATH` or
  `devsweep-audit.jsonl`.

#### 4. Validation & Error Matrix
- Dry-run with or without plan -> returns selected target count, no side
  effects, no audit file.
- `--execute` without `--plan` -> error before executor runs.
- Audit file cannot be opened -> error before any target action runs.
- Individual target failure -> record failed audit entry, continue remaining
  targets, return a report with failures.
- Inspect-only target -> skipped audit entry, no side effect.
- Permanent delete target -> failed audit entry, no side effect.

#### 5. Good/Base/Bad Cases
- Good: command-backed Rust target records `["cargo", "clean",
  "--manifest-path", "<Cargo.toml>"]`.
- Good: trash-backed target moves exactly the path from
  `CleanAction::MoveToTrash`.
- Base: `devsweep clean` reports a dry-run with zero selected targets when no
  plan is provided.
- Bad: executor reruns scanner logic to infer paths.
- Bad: executor uses `cmd /C`, `sh -c`, or string-form shell commands.
- Bad: a failed target aborts the job before later selected targets are audited.

#### 6. Tests Required
- Dry-run test proving command and trash runners are not called.
- Command-runner test asserting program and argv are separate.
- Trash-runner test asserting the exact plan path is used.
- Audit JSONL test covering both success and failure records in one job.
- Permanent-delete test proving the file remains present even with the flag.
- Scanner regression proving Rust target plans keep `--manifest-path`.

#### 7. Wrong vs Correct

Wrong:
```rust
std::process::Command::new("cmd")
    .args(["/C", &format!("cargo clean --manifest-path {}", manifest.display())])
    .status()?;
```

Correct:
```rust
CommandRequest {
    program: "cargo".to_string(),
    args: vec![
        "clean".to_string(),
        "--manifest-path".to_string(),
        manifest.display().to_string(),
    ],
    cwd: None,
}
```

### Scenario: Global cache providers

#### 1. Scope / Trigger
- Trigger: `devsweep scan --global` discovers global package-manager cache
  providers and emits command-backed or inspect-only cleanup plan targets.

#### 2. Signatures
- CLI entrypoint:
  - `devsweep scan [ROOT]... [--json] [--global] [--projects]`
- Library entrypoint:
  - `GlobalProviderScanner::new().scan() -> CleanupPlan`
- Provider discovery boundary:
  - executable lookup by program name
  - command output probes for official inspect commands only

#### 3. Contracts
- `scan --global` may run read-only or provider-owned inspect commands such as
  `npm config get cache`, `pip cache dir`, `pnpm store path`, `yarn --version`,
  and Yarn cache-folder commands.
- `scan --global` must never execute cleanup commands. It only serializes
  future `CleanAction::Command` plans.
- npm, pip, pnpm, and Yarn global cache cleanup targets use
  `CleanAction::Command` with program and argv stored separately.
- A provider must not emit multiple user-facing cleanup targets for the same
  cache path just because the tool has alternative commands. Keep one cleanup
  target per physical cache footprint and record related official commands in
  evidence unless the model grows an explicit action-choice contract.
- Missing npm, pip, pnpm, or Yarn executables are non-fatal unavailable
  providers.
- Cargo home is inspect-only by default and uses
  `CleanAction::NoopInspectOnly`. Do not mark cargo `bin`,
  `credentials.toml`, `.crates.toml`, or the whole cargo home as a cleanable
  delete/trash target.
- Docker builder cache is not part of the MVP global provider set.

#### 4. Validation & Error Matrix
- Tool missing -> no target for that provider, scan still succeeds.
- Cache path command fails -> command-backed target may still be emitted with
  no `path`, as long as official command evidence is present.
- Yarn version unknown -> skip Yarn provider rather than guessing classic vs
  modern cleanup behavior.
- Cargo home missing or not a directory -> no Cargo home target.

#### 5. Good/Base/Bad Cases
- Good: npm target plans `["cache", "verify"]` or
  `["cache", "clean", "--force"]` without running either command.
- Good: npm cache discovery emits one target for the cache directory and may
  list `npm cache verify` as evidence for the same target.
- Good: pip target plans `["-m", "pip", "cache", "purge"]` and uses
  `pip cache dir` only for discovery evidence.
- Good: pnpm target plans `["store", "prune"]`.
- Good: Yarn provider branches on `yarn --version` before choosing classic or
  modern cache clean arguments.
- Good: Cargo home target is high risk, not selected by default, and
  inspect-only.
- Bad: deleting `_cacache`, pip `http-v2`, pnpm store internals, Yarn cache
  folders, or Cargo home directories directly.
- Bad: emitting both `npm.cache.verify:<path>` and `npm.cache.clean:<path>` as
  separate targets with the same estimated size.
- Bad: adding Docker builder cache to this MVP provider set.

#### 6. Tests Required
- Missing-tool provider test proving scan succeeds with no targets.
- Available-tool provider test asserting official program/argv pairs.
- Duplicate-path provider test proving alternative commands do not create
  duplicate user-facing targets for the same cache directory.
- Yarn version-branch test for classic and modern command arguments.
- Cargo home test proving the action is inspect-only and no cargo `bin` or
  credential path is emitted as cleanable.

#### 7. Wrong vs Correct

Wrong:
```rust
// Provider discovery must not directly delete opaque cache internals.
CleanAction::MoveToTrash {
    path: npm_cache.join("_cacache"),
}
```

Correct:
```rust
CleanAction::Command {
    program: npm_program,
    args: vec!["cache".to_string(), "clean".to_string(), "--force".to_string()],
    cwd: None,
    irreversible: true,
}
```

### Scenario: CI and release archive

#### 1. Scope / Trigger
- Trigger: repository-level validation or release packaging changes add or
  change CI workflow commands, `justfile` recipes, or generated release
  artifacts.

#### 2. Signatures
- Local validation:
  - `just ci`
- Local release archive:
  - `just release-archive`
- CI validation:
  - `cargo fmt --all -- --check`
  - `cargo check --all-targets`
  - `cargo test --all-targets`
  - `cargo clippy --all-targets -- -D warnings`

#### 3. Contracts
- `just ci` remains the canonical local quality gate.
- CI must run the same four validation classes as `just ci`: format, check,
  tests, and clippy.
- `just release-archive` builds the release binary and writes a Windows archive
  to `dist/devsweep-x86_64-pc-windows-msvc.zip`.
- `dist/` is a generated artifact directory and must stay ignored by git.
- Release packaging must not imply package-manager distribution such as Scoop,
  Winget, Homebrew, or cargo publish.

#### 4. Validation & Error Matrix
- Format/check/test/clippy failure -> CI fails and local `just ci` fails.
- Release build failure -> `just release-archive` fails before creating or
  replacing the archive.
- Missing `target/release/devsweep.exe` after build -> archive command fails.
- Generated `dist/` output -> ignored by git status.

#### 5. Good/Base/Bad Cases
- Good: CI includes Windows and a non-Windows runner for the Rust validation
  matrix.
- Good: release recipe uses the already built single binary.
- Base: local `cargo build --release` succeeds without packaging.
- Bad: committing generated `dist/*.zip` archives.
- Bad: documenting Docker cleanup, permanent delete, or package-manager
  distribution as released MVP behavior.

#### 6. Tests Required
- Run `just ci` after workflow or validation command changes.
- Run `just release-archive` after release recipe changes on Windows.
- Check `git status --short --ignored` to confirm `dist/` is ignored.

#### 7. Wrong vs Correct

Wrong:
```text
Commit dist/devsweep-x86_64-pc-windows-msvc.zip as a release artifact.
```

Correct:
```text
Generate dist/devsweep-x86_64-pc-windows-msvc.zip locally and keep dist/
ignored by git.
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
