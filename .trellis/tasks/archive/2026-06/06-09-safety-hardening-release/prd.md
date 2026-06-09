# Safety hardening and release

## Goal

Harden the MVP for Windows-first safety, cross-platform correctness, documentation, CI, and release packaging after core functionality works.

## Parent Context

- Parent task: `.trellis/tasks/06-09-design-split-analysis`
- Depends on all MVP implementation children.
- Source design sections: Windows-specific design, tests, platform matrix, release shape.
- User decision: Docker is deferred from MVP and may be reconsidered here or in a later child.

## Requirements

- Add focused safety tests for symlink, junction, and reparse point behavior where the platform allows it.
- Improve Windows locked-file and partial-failure reporting behavior.
- Add snapshot/smoke tests for key TUI states.
- Add CI checks for the supported Rust targets/platforms practical for this repo.
- Add release packaging for a single binary/archive.
- Write README/user documentation that explains:
  - dry-run default
  - evidence and risk levels
  - command-backed global cleanup
  - trash-backed project cleanup
  - Cargo home inspect-only behavior
  - permanent delete disabled
  - Docker deferred/not included in MVP
- Optionally create a follow-up design note or task for Docker builder cache after MVP safety is verified.

## Acceptance Criteria

- [x] Safety tests cover non-following of symlinks/reparse points or document platform limitations.
- [x] Locked-file/partial-failure behavior is tested or manually verified on Windows.
- [x] TUI snapshot/smoke checks cover dashboard, details, confirm modal, and jobs/logs.
- [x] CI runs the agreed validation commands.
- [x] Release archive can be produced locally.
- [x] README documents MVP scope and explicitly states Docker is deferred.

## Out Of Scope

- Implementing core scanner/executor/provider/TUI features not completed by earlier children.
- Docker cleanup implementation unless the user explicitly approves a new follow-up task.
- Package-manager distribution such as Scoop, Winget, or Homebrew.
