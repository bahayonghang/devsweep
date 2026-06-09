# Execution engine and audit log

## Goal

Implement the non-mutating dry-run path, safe execution boundaries, and audit logging for project cleanup actions.

## Parent Context

- Parent task: `.trellis/tasks/06-09-design-split-analysis`
- Depends on: `foundation-cli-domain-model`, `project-scanner-json-plan`
- Source design sections: execution engine, command execution safety, audit log, Rust project cleanup strategy.

## Requirements

- Execute cleanup only from an explicit plan/selected targets.
- Keep dry-run as the default behavior for CLI and internal engine calls.
- Implement command-backed execution without shell string interpolation.
- Implement `cargo clean` action for Rust targets where metadata supports it.
- Implement trash-backed project cleanup for selected directory actions.
- Emit audit JSONL for attempted actions, successes, failures, partial failures, durations, estimated bytes, and command argv where applicable.
- Continue a job after individual target failures and return a failure report.
- Ensure permanent delete remains disabled unless a future explicit scope enables it.

## Acceptance Criteria

- [ ] Dry-run tests prove fixture files/directories are not mutated.
- [ ] Command actions are built with program plus argv, not shell-composed strings.
- [ ] `cargo clean` plan includes `--manifest-path` or equivalent safe project targeting.
- [ ] Trash-backed actions are isolated to target paths from the cleanup plan.
- [ ] Audit JSONL records both success and failure cases.
- [ ] Permanent delete cannot run by default.

## Out Of Scope

- Global npm/pip/pnpm/Yarn provider implementation.
- Full TUI confirmation UI.
- Docker support.
- Advanced Cargo home cleanup.

