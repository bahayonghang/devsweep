# Code Map

Start here before broad grep or repo-wide search. This repository is a Cargo
workspace with reusable core and CLI/TUI crates plus Trellis/Codex scaffolding.

## Root

- `Cargo.toml` - virtual workspace membership and shared package metadata.
- `justfile` - local command registry. `just ci` is the canonical gate.
- `.github/workflows/ci.yml` - CI parity for format, check, test, and clippy on
  Windows, Ubuntu, and macOS, plus Windows/Ubuntu MSRV jobs.
- `README.md` - current user-facing safety model, usage, validation, and release
  archive notes.
- `design.md` - historical product and architecture design context. Treat
  current code, README, tests, and Trellis specs as stronger evidence when they
  differ.

## Rust Source

### Core

- `crates/devsweep-core/src/lib.rs` - public reusable core module boundary.
- `crates/devsweep-core/src/cargo_metadata.rs` - neutral Cargo scope probes, result classification,
  JSON parsing, and diagnostics shared by scanning and live authorization.
- `crates/devsweep-core/src/services.rs` - frontend-neutral scan, inventory,
  and cleanup service traits plus production adapters.
- `crates/devsweep-core/src/model/mod.rs` - public domain-model interface.
- `crates/devsweep-core/src/model/plan.rs` - v2 persisted plan DTOs and internal cleanup domain
  values.
- `crates/devsweep-core/src/model/scan.rs` - scan report, health, diagnostics, totals, and sizing
  warning contracts.
- `crates/devsweep-core/src/plan/mod.rs` - `UntrustedPlan` to opaque `ValidatedPlan` trust boundary.
- `crates/devsweep-core/src/plan/digest.rs` - private canonical manifest and SHA-256 digest.
- `crates/devsweep-core/src/rules/mod.rs` - rule catalogue, display formatting, and trusted
  intent-resolution interface.
- `crates/devsweep-core/src/rules/definitions.rs` - sole owner of built-in rule facts and IDs.
- `crates/devsweep-core/src/rules/registry.rs` - private typed intent-to-action reconstruction.
- `crates/devsweep-core/src/filesystem/mod.rs` - shared filesystem safety and sizing interface.
- `crates/devsweep-core/src/filesystem/identity.rs` - lexical normalization and live file identity.
- `crates/devsweep-core/src/filesystem/containment.rs` - containment and current-executable checks.
- `crates/devsweep-core/src/filesystem/reparse.rs` - no-follow symlink/reparse-point inspection.
- `crates/devsweep-core/src/filesystem/sizing.rs` - bounded, cancelable, no-follow tree sizing.
- `crates/devsweep-core/src/process/mod.rs` - bounded process runner and typed request/result policy.
- `crates/devsweep-core/src/process/cancel.rs` - cooperative cancellation observers.
- `crates/devsweep-core/src/process/capture.rs` - bounded stream capture and output sanitization.
- `crates/devsweep-core/src/process/tree.rs` - Windows Job Object and Unix process-group lifecycle.
- `crates/devsweep-core/src/scan/mod.rs` - `Sweeper` merge/progress/report interface and sole
  production ranking call site.
- `crates/devsweep-core/src/scan/ranking.rs` - pure ordering and conservative default selection.
- `crates/devsweep-core/src/scan/project/mod.rs` - marker-first traversal, diagnostics, and reviewed
  target rescan.
- `crates/devsweep-core/src/scan/project/cargo.rs` - scan-lifetime Cargo workspace cache.
- `crates/devsweep-core/src/scan/project/dedupe.rs` - scan-root and target-footprint deduplication.
- `crates/devsweep-core/src/scan/global/mod.rs` - global provider target construction.
- `crates/devsweep-core/src/scan/global/probe.rs` - bounded read-only provider command probes.
- `crates/devsweep-core/src/inventory/mod.rs` - read-only capacity inventory orchestration.
- `crates/devsweep-core/src/inventory/pnpm.rs` - bounded pnpm project-reference and orphan evidence
  inspection.
- `crates/devsweep-core/src/execution/mod.rs` - explicit-selection execution orchestration and the
  authorize/audit/dispatch state machine.
- `crates/devsweep-core/src/execution/audit.rs` - durable JSONL action journal and replay logic.
- `crates/devsweep-core/src/execution/command.rs` - bounded command and trash runner adapters.
- `crates/devsweep-core/src/execution/safety/mod.rs` - sole live authorization funnel.
- `crates/devsweep-core/src/execution/safety/protections.rs` - versioned atomic user-protection
  persistence.

### CLI And TUI

- `crates/devsweep-cli/src/main.rs` - minimal binary entrypoint delegating to `devsweep::run()`.
- `crates/devsweep-cli/src/lib.rs` - CLI library boundary exporting only `run`.
- `crates/devsweep-cli/src/application/mod.rs` - tracing initialization, `Cli::parse`, and typed dispatch.
- `crates/devsweep-cli/src/application/cli.rs` - private Clap definitions and command-surface tests.
- `crates/devsweep-cli/src/application/commands.rs` - command handlers, decoding, output, and boundary tests.
- `crates/devsweep-cli/src/bin/process_fixture.rs` - child/grandchild process-tree test fixture.
- `crates/devsweep-cli/src/tui/mod.rs` - crate-private TUI composition entrypoint.
- `crates/devsweep-cli/src/tui/terminal.rs` - raw-mode and alternate-screen lifecycle.
- `crates/devsweep-cli/src/tui/display.rs` - pure presentation shared by state and rendering.
- `crates/devsweep-cli/src/tui/app/` - app state, reducer transitions, selection, jobs, and tests.
- `crates/devsweep-cli/src/tui/runtime/mod.rs` - event loop, effects, channels, cancellation, and
  cleanup single-flight.
- `crates/devsweep-cli/src/tui/runtime/workers.rs` - worker execution and event translation.
- `crates/devsweep-cli/src/tui/runtime/tests.rs` - fake-service and runtime behavior tests.
- `crates/devsweep-cli/src/tui/render/` - pure layouts, views, overlays, themes, and render tests.
- `crates/devsweep-cli/src/tui/test_support.rs` - test-only keys, plans, and render helpers.

### Desktop Shell

- `desktop/src-tauri/src/lib.rs` - Tauri application composition and command registration.
- `desktop/src-tauri/src/commands.rs` - async scan, plan execution, and protection-list IPC commands.
- `desktop/src-tauri/src/scan.rs` - single-flight scan state, cancellation, and progress forwarding.
- `desktop/src-tauri/src/error.rs` - centralized structured command-error mapping.
- `desktop/src/App.tsx` - React workflow composition for scan, review, dry-run,
  confirmation, execution, and reporting.
- `desktop/src/api/` - generated IPC types, closed-world runtime decoders, the
  production Tauri bridge, and controlled fixture replay.
- `desktop/src/state/` - reducer-owned workflow state, digest invalidation, and
  derived selection/capacity rules.
- `desktop/src/pages/` and `desktop/src/components/` - accessible workflow views,
  target table, evidence, errors, confirmation, and result presentation.

The dependency direction starts at `main -> devsweep::run -> application ->
devsweep-core`. Core modules never import CLI or TUI code. TUI render code
remains read-only and backend discovery layers remain non-mutating.

## Agent skill packages

- `skills/devsweep-inspect/` - Production agent skill that inspects with the
  DevSweep CLI and returns cleanup or optimization advice. Recommend-only.

## Specs And Workflow

- `.trellis/spec/backend/index.md` - backend pre-development checklist and
  links for scanner, executor, CLI, logging, and data-contract work.
- `.trellis/spec/frontend/index.md` - ratatui/TUI pre-development checklist and
  links for render, state, event, and type-safety work.
- `.trellis/spec/desktop-frontend/index.md` - React/Tauri webview structure,
  state, IPC type-safety, accessibility, and quality checklist.
- `.trellis/workflow.md` and `.trellis/scripts/` - Trellis task lifecycle and
  context helpers.
- `.agents/skills/`, `.codex/agents/`, `.codex/hooks.json`, and `.claude/` -
  local platform scaffolding. These paths are ignored by git in this checkout
  unless policy changes.

## Generated Or Ignored Paths

- `target/` - Cargo build output.
- `dist/` - release archive output from `just release-archive`.
- `desktop/node_modules/` and `desktop/dist/` - desktop dependency and web build output.
- Do not add nested guidance for generated, vendored, dependency, cache, or
  build-output directories.
