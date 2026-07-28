# Code Map

Start here before broad grep or repo-wide search. This repository is a single
Rust crate plus Trellis/Codex workflow scaffolding.

## Root

- `Cargo.toml` - crate metadata and dependencies for the `devsweep` binary.
- `justfile` - local command registry. `just ci` is the canonical gate.
- `.github/workflows/ci.yml` - CI parity for format, check, test, and clippy on
  Windows and Ubuntu.
- `README.md` - current user-facing safety model, usage, validation, and release
  archive notes.
- `design.md` - historical product and architecture design context. Treat
  current code, README, tests, and Trellis specs as stronger evidence when they
  differ.

## Rust Source

- `src/main.rs` - CLI dispatch and top-level handlers for `tui`, `scan`,
  `clean`, `protect`, and `rules`.
- `src/cli.rs` - `clap` command definitions and command-line option shapes.
- `src/model.rs` - serialized cleanup-plan contract: `CleanupPlan`,
  `CleanTarget`, `CleanAction`, risk, evidence, scope, and ecosystem enums.
- `src/scanner.rs` - marker-first project scanner for Rust, Node, and Python
  cleanup targets; includes symlink/reparse guards and nested-target dedupe.
- `src/providers.rs` - global provider scanner for npm, pip, pnpm, Yarn, and
  inspect-only Cargo home targets.
- `src/executor.rs` - dry-run and execution engine, command/trash runner
  abstractions, partial-failure handling, and JSONL audit records.
- `src/safety.rs` - central `SafetyPolicy::authorize` funnel, protection
  categories, user protection list, live revalidation, cargo metadata scope.
- `src/path_identity.rs` - lexical path normalization and live file identity.
- `src/process_runner.rs` - bounded external process runner: timeout, output
  caps, tree termination, neutral cwd, sanitize, typed diagnostics.
- `src/bin/process_fixture.rs` - child/grandchild helper for process-tree tests.
- `src/tui.rs` - ratatui entrypoint, app state, update/effect loop, worker
  events, render helpers, and TUI tests.
- `src/config.rs` - tracing initialization; diagnostics go to stderr so JSON
  output can stay clean.
- `src/lib.rs` - crate module exports shared by the binary and tests.

## Specs And Workflow

- `.trellis/spec/backend/index.md` - backend pre-development checklist and
  links for scanner, executor, CLI, logging, and data-contract work.
- `.trellis/spec/frontend/index.md` - ratatui/TUI pre-development checklist and
  links for render, state, event, and type-safety work.
- `.trellis/workflow.md` and `.trellis/scripts/` - Trellis task lifecycle and
  context helpers.
- `.agents/skills/`, `.codex/agents/`, `.codex/hooks.json`, and `.claude/` -
  local platform scaffolding. These paths are ignored by git in this checkout
  unless policy changes.

## Generated Or Ignored Paths

- `target/` - Cargo build output.
- `dist/` - release archive output from `just release-archive`.
- Do not add nested guidance for generated, vendored, dependency, cache, or
  build-output directories.
