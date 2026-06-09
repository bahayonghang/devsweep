# Foundation CLI and domain model

## Goal

Create the Rust application foundation and the stable cleanup domain contract that all later scanner, executor, provider, CLI, and TUI work will use.

## Parent Context

- Parent task: `.trellis/tasks/06-09-design-split-analysis`
- Source design: root `design.md`
- This is the first implementation child. It has no code dependency on other children.

## Requirements

- Create the Rust project structure for the `devsweep` cleanup tool.
- Create a root `justfile` as the standard local command entrypoint.
- Add `justfile` recipes for at least:
  - `ci`
  - `build`
  - `dev`
  - `test`
- Add CLI entrypoints for the future modes described in root `design.md`: `tui`, `scan`, `clean`, and `rules`.
- Add an empty/non-destructive ratatui entrypoint so the binary can launch a placeholder TUI without cleanup behavior.
- Add config and tracing/logging bootstrap only as far as later children need.
- Define the core cleanup plan/domain model:
  - `CleanTarget`
  - `Scope`
  - `Ecosystem`
  - `TargetKind`
  - `RiskLevel`
  - `CleanAction`
  - `Evidence`
- Support JSON serialization for cleanup plans.
- Keep all behavior non-mutating in this child.

## Acceptance Criteria

- [x] `cargo check` passes.
- [x] Root `justfile` exists and exposes `ci`, `build`, `dev`, and `test` recipes.
- [x] `just ci` runs the current foundation validation sequence.
- [x] CLI help shows the planned subcommands.
- [x] `scan --json` can emit an empty but valid JSON plan.
- [x] The domain model can serialize and deserialize a representative `CleanTarget`.
- [x] No code path performs scanning, deletion, trash movement, or external cleanup command execution.

## Out Of Scope

- Real project scanning.
- Cleanup execution.
- Global provider discovery.
- Full TUI screens.
- Docker support.
