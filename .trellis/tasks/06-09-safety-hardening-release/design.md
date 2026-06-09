# Safety hardening and release design

## Scope

This child closes the MVP after scanner, executor, providers, and TUI basics exist. It should harden the existing surfaces without adding new cleanup domains.

In scope:

- scanner safety around symlinks, Windows junctions, and reparse points
- executor reporting for locked-file or partial failures
- TUI smoke coverage for dashboard, details, confirm modal, and jobs/logs
- CI and local release archive entrypoints
- README documentation for MVP safety behavior

Out of scope:

- Docker cleanup implementation
- new package-manager providers
- permanent delete support
- broad scanner rule expansion

## Current Implementation Boundaries

- `src/scanner.rs` discovers marker-backed project targets and already uses `symlink_metadata` plus `follow_links = false` behavior by construction.
- `src/executor.rs` executes only selected plan actions, keeps dry-run non-mutating, records per-target audit results, and continues after action failures.
- `src/providers.rs` emits command-backed global targets for npm, pip, pnpm, Yarn, plus inspect-only Cargo home.
- `src/tui.rs` renders and updates app state; worker effects own scan/clean side effects.
- `justfile` is the stable local validation surface.

## Safety Contracts

- Scanner must never follow symlink or reparse-point cleanup directories.
- Scanner tests should cover symlink behavior on all platforms and Windows junction/reparse behavior when available.
- Failed cleanup actions must be reported per target and must not stop later selected targets.
- Audit records for failures must keep `partial = true`.
- Permanent delete remains disabled even if the CLI flag is present.
- Cargo home remains inspect-only.
- Docker remains documented as deferred.

## CI And Release Shape

- CI should run the same validation commands as local development: format, check, tests, and clippy.
- The matrix should include Windows and at least one non-Windows Rust target practical for this repo.
- Release packaging is a local archive recipe for the single built binary. Package-manager distribution stays out of scope.

## Documentation Shape

README should explain:

- dry-run default for `clean`
- evidence and risk fields in cleanup plans
- command-backed global cleanup
- trash-backed project cleanup
- Cargo home inspect-only behavior
- permanent delete disabled in MVP
- Docker deferred from MVP
- local validation and release archive commands

## Rollback Notes

- If scanner hardening breaks normal fixture discovery, rollback the specific platform guard before changing rule matching.
- If CI or release scripts fail because a tool is unavailable, prefer adjusting the repo-local recipe over weakening product tests.
- Documentation changes should not claim unsupported cleanup behavior.
