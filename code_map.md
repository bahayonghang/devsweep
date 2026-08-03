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

- `src/main.rs` - minimal binary entrypoint delegating to `devsweep::run()`.
- `src/lib.rs` - binary-oriented crate boundary; exports only `run` and keeps
  all implementation modules private.
- `src/cargo_metadata.rs` - neutral Cargo scope probes, result classification,
  JSON parsing, and diagnostics shared by scanning and live authorization.
- `src/application/mod.rs` - tracing initialization, `Cli::parse`, and typed
  command dispatch.
- `src/application/cli.rs` - private Clap command and option definitions plus
  command-surface tests.
- `src/application/commands.rs` - scan, inventory, clean, protect, and rules
  handlers; saved-input decoding; CLI formatting; command-boundary tests.
- `src/model/mod.rs` - private domain-model interface.
- `src/model/plan.rs` - v2 persisted plan DTOs and internal cleanup domain
  values.
- `src/model/scan.rs` - scan report, health, diagnostics, totals, and sizing
  warning contracts.
- `src/plan/mod.rs` - `UntrustedPlan` to opaque `ValidatedPlan` trust boundary.
- `src/plan/digest.rs` - private canonical manifest and SHA-256 digest.
- `src/rules/mod.rs` - rule catalogue, display formatting, and trusted
  intent-resolution interface.
- `src/rules/definitions.rs` - sole owner of built-in rule facts and IDs.
- `src/rules/registry.rs` - private typed intent-to-action reconstruction.
- `src/filesystem/mod.rs` - shared filesystem safety and sizing interface.
- `src/filesystem/identity.rs` - lexical normalization and live file identity.
- `src/filesystem/containment.rs` - containment and current-executable checks.
- `src/filesystem/reparse.rs` - no-follow symlink/reparse-point inspection.
- `src/filesystem/sizing.rs` - bounded, cancelable, no-follow tree sizing.
- `src/process/mod.rs` - bounded process runner and typed request/result policy.
- `src/process/cancel.rs` - cooperative cancellation observers.
- `src/process/capture.rs` - bounded stream capture and output sanitization.
- `src/process/tree.rs` - Windows Job Object and Unix process-group lifecycle.
- `src/bin/process_fixture.rs` - child/grandchild process-tree test fixture.
- `src/scan/mod.rs` - `Sweeper` merge/progress/report interface and sole
  production ranking call site.
- `src/scan/ranking.rs` - pure ordering and conservative default selection.
- `src/scan/project/mod.rs` - marker-first traversal, diagnostics, and reviewed
  target rescan.
- `src/scan/project/cargo.rs` - scan-lifetime Cargo workspace cache.
- `src/scan/project/dedupe.rs` - scan-root and target-footprint deduplication.
- `src/scan/global/mod.rs` - global provider target construction.
- `src/scan/global/probe.rs` - bounded read-only provider command probes.
- `src/inventory/mod.rs` - read-only capacity inventory orchestration.
- `src/inventory/pnpm.rs` - bounded pnpm project-reference and orphan evidence
  inspection.
- `src/execution/mod.rs` - explicit-selection execution orchestration and the
  authorize/audit/dispatch state machine.
- `src/execution/audit.rs` - durable JSONL action journal and replay logic.
- `src/execution/command.rs` - bounded command and trash runner adapters.
- `src/execution/safety/mod.rs` - sole live authorization funnel.
- `src/execution/safety/protections.rs` - versioned atomic user-protection
  persistence.
- `src/tui/mod.rs` - crate-private TUI composition entrypoint.
- `src/tui/terminal.rs` - raw-mode and alternate-screen lifecycle.
- `src/tui/display.rs` - pure presentation shared by state and rendering.
- `src/tui/app/mod.rs` - single mutable `App` owner and root update routing.
- `src/tui/app/events.rs` - job, UI, effect, and worker event protocol.
- `src/tui/app/input.rs` - normal, filter, overlay, confirmation, and quit input.
- `src/tui/app/selection.rs` - target rows, grouping, cursors, and selection.
- `src/tui/app/worker.rs` - scan, inventory, and clean result transitions.
- `src/tui/app/jobs.rs` - jobs, logs, progress, and frozen confirmation state.
- `src/tui/app/tests.rs` - reducer and state-machine tests.
- `src/tui/runtime/mod.rs` - event loop, effects, channels, cancellation, and
  cleanup single-flight.
- `src/tui/runtime/services.rs` - private service traits and production adapters.
- `src/tui/runtime/workers.rs` - worker execution and event translation.
- `src/tui/runtime/tests.rs` - fake-service and runtime behavior tests.
- `src/tui/render/mod.rs` - pure root layout and view routing.
- `src/tui/render/theme.rs` - shared semantic styles.
- `src/tui/render/format.rs` - render-only typed formatting.
- `src/tui/render/targets.rs` - category, target, detail, and rules views.
- `src/tui/render/inventory.rs` - capacity inventory views.
- `src/tui/render/jobs.rs` - job, log, and diagnostic views.
- `src/tui/render/overlays.rs` - confirmation, progress, help, error, and quit
  overlays.
- `src/tui/render/tests.rs` - ratatui `TestBackend` coverage.
- `src/tui/test_support.rs` - test-only keys, plans, and render helpers.

The dependency direction starts at `main -> devsweep::run -> application`.
`application` composes private scan, inventory, execution, rules, and TUI
interfaces. Those implementation modules do not import `application`; TUI
render code remains read-only and backend discovery layers remain non-mutating.

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
