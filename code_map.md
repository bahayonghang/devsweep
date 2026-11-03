# Code Map

Start here before broad grep or repo-wide search. This repository is a Cargo
workspace with reusable core and CLI/TUI crates plus Trellis/Codex scaffolding.

## Root

- `Cargo.toml` - virtual workspace membership and shared package metadata.
- `justfile` - local command registry. `just ci` is the canonical gate.
  `just tdev` starts the Tauri desktop development window.
- `.github/workflows/ci.yml` - CI parity for format, check, test, and clippy on
  Windows, Ubuntu, and macOS, plus Windows/Ubuntu MSRV jobs. Hosted runs start
  from pull requests and manual `workflow_dispatch`, not from every push to
  `dev`.
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
- `crates/devsweep-core/src/digest_encoding.rs` - private lowercase hex encoding;
  domain modules retain ownership of digest preimages and wire prefixes.
- `crates/devsweep-core/src/services.rs` - frontend-neutral scan, inventory,
  and cleanup service traits plus production adapters.
- `crates/devsweep-core/src/model/mod.rs` - public domain-model interface.
- `crates/devsweep-core/src/model/plan.rs` - v2 persisted plan DTOs and internal cleanup domain
  values.
- `crates/devsweep-core/src/model/scan.rs` - scan report, root/provider coverage, health, diagnostics, totals, and sizing
  warning contracts.
- `crates/devsweep-core/src/plan/mod.rs` - `UntrustedPlan` to opaque `ValidatedPlan` trust boundary.
- `crates/devsweep-core/src/plan/digest.rs` - private canonical manifest and SHA-256 digest.
- `crates/devsweep-core/src/rules/mod.rs` - rule catalogue, display formatting, and trusted
  intent-resolution interface.
- `crates/devsweep-core/src/rules/definitions.rs` - sole owner of built-in rule facts and IDs.
- `crates/devsweep-core/src/rules/registry.rs` - private typed intent-to-action reconstruction.
- `crates/devsweep-core/src/rules/configuration.rs` - shared trusted configured global-cache roots.
- `crates/devsweep-core/src/rules/generated.rs` - marker, generated-content, repository and live-owner proof for new project outputs.
- `crates/devsweep-core/src/rules/windows_caches.rs` - closed current-user browser/editor cache discovery and temporary-entry eligibility.
- `crates/devsweep-core/src/scan_preferences.rs` - explicit desktop scan-root validation and atomic persistence.
- `crates/devsweep-core/src/project_footprints.rs` - pure per-owner logical occupancy and overlap projection over completed observations.
- `crates/devsweep-core/src/clean_advisories/` - pure project capacity advice, confidence-aware subtotals and the separate whole-GiB threshold store.
- `crates/devsweep-core/src/filesystem/mod.rs` - shared filesystem safety and sizing interface.
- `crates/devsweep-core/src/filesystem/identity.rs` - lexical normalization and live file identity.
- `crates/devsweep-core/src/filesystem/containment.rs` - containment and current-executable checks.
- `crates/devsweep-core/src/filesystem/reparse.rs` - no-follow symlink/reparse-point inspection.
- `crates/devsweep-core/src/filesystem/sizing.rs` - bounded, cancelable, no-follow tree sizing.
- `crates/devsweep-core/src/process/mod.rs` - bounded process runner and typed request/result policy.
- `crates/devsweep-core/src/process/cancel.rs` - cooperative cancellation observers.
- `crates/devsweep-core/src/process/capture.rs` - bounded stream capture and output sanitization.
- `crates/devsweep-core/src/process/tree.rs` - Windows Job Object and Unix process-group lifecycle.
- `crates/devsweep-core/src/process/executable.rs` - shared executable lookup for provider discovery and binding.
- `crates/devsweep-core/src/process/provider_dispatch.rs` - native provider dispatch and closed Windows Node-shim identities without shell execution.
- `crates/devsweep-core/src/scan/mod.rs` - `Sweeper` merge/progress/report interface and sole
  production ranking call site.
- `crates/devsweep-core/src/scan/ranking.rs` - pure ordering and conservative default selection.
- `crates/devsweep-core/src/scan/project/mod.rs` - marker-first traversal, diagnostics, and reviewed
  target rescan.
- `crates/devsweep-core/src/scan/project/cargo.rs` - scan-lifetime Cargo workspace cache.
- `crates/devsweep-core/src/scan/project/dedupe.rs` - scan-root and target-footprint deduplication.
- `crates/devsweep-core/src/scan/global/mod.rs` - global provider target construction.
- `crates/devsweep-core/src/scan/global/probe.rs` - bounded read-only provider command probes.
- `crates/devsweep-core/src/scan/global/mbx.rs` - public mbx version/shared-location coverage without storage traversal or cleanup authority.
- `crates/devsweep-core/src/inventory/mod.rs` - read-only capacity inventory orchestration.
- `crates/devsweep-core/src/inventory/pnpm.rs` - bounded pnpm project-reference and orphan evidence
  inspection.
- `crates/devsweep-core/src/execution/mod.rs` - explicit-selection execution orchestration and the
  authorize/audit/dispatch state machine.
- `crates/devsweep-core/src/execution/audit.rs` - durable JSONL action journal and replay logic.
- `crates/devsweep-core/src/execution/command.rs` - bounded command and trash runner adapters.
- `crates/devsweep-core/src/execution/provider.rs` - live provider footprint/tool/configuration binding for preview and execute.
- `crates/devsweep-core/src/execution/safety/mod.rs` - sole live authorization funnel.
- `crates/devsweep-core/src/execution/safety/protections.rs` - versioned atomic user-protection
  persistence.

### Five-mode core

- `crates/devsweep-core/src/analysis/mod.rs` - bounded read-only Analyze walker and snapshots.
  The walker never creates cleanup authority.
- `crates/devsweep-core/src/analysis/model.rs` - Analyze V1 snapshot, node, warning, and budget
  types.
- `crates/devsweep-core/src/analysis/walker.rs` - two-worker traversal with accounted caps.
- `crates/devsweep-core/src/analysis/actions.rs` - node path rebuild, Explorer reveal, and the
  confirmed `analyze.trash` Recycle Bin plan run by the Clean executor.
- `crates/devsweep-core/src/software/mod.rs` - installed-software inventory and preview planning.
  Never reads vendor uninstall command fields as authority.
- `crates/devsweep-core/src/software/plan.rs` - Software selection plan validation.
- `crates/devsweep-core/src/software/execution/mod.rs` - current-user MSIX dispatch only.
- `crates/devsweep-core/src/software/updates.rs` - read-only `winget upgrade` probe and table parser.
- `crates/devsweep-core/src/software/startup.rs` - startup items; current-user `StartupApproved` write only.
- `crates/devsweep-core/src/software/leftovers.rs` - leftover discovery, plan digest, and Recycle Bin move.
- `crates/devsweep-core/src/optimize/mod.rs` - closed eight-id Windows maintenance catalogue.
- `crates/devsweep-core/src/optimize/catalogue.rs` - exhaustive operation ids and badges.
- `crates/devsweep-core/src/optimize/plan.rs` - one-operation plan and live preview digest.
- `crates/devsweep-core/src/optimize/execution.rs` - DNS flush and frozen Settings URI launch.
- `crates/devsweep-core/src/status/mod.rs` - bounded read-only Status snapshots and live sampling.
- `crates/devsweep-core/src/status/sampler.rs` - snapshot/live producer and lifecycle events.
- `crates/devsweep-core/src/status/channel.rs` - bounded lossless Status delivery with cancellation and terminal reserves.
- `crates/devsweep-core/src/status/pdh.rs` - read-only PDH probe for GPU engine and thermal-zone counters.
- `crates/devsweep-core/src/history/mod.rs` - inspect-only reader for the three fixed V1 audit
  stores.

### CLI And TUI

- `crates/devsweep-cli/src/main.rs` - minimal binary entrypoint delegating to `devsweep::run()`.
- `crates/devsweep-cli/src/lib.rs` - CLI library boundary exporting only `run`.
- `crates/devsweep-cli/src/application/mod.rs` - tracing initialization, `Cli::parse`, bare-TUI
  routing, and typed dispatch. Shipped roots: `clean`, `software`, `optimize`, `analyze`,
  `status`, `history`.
- `crates/devsweep-cli/src/application/cli.rs` - private Clap definitions and command-surface tests.
- `crates/devsweep-cli/src/application/output.rs` - V1 JSON/NDJSON envelopes, create-new sinks, and
  exit classes.
- `crates/devsweep-cli/src/application/commands/mod.rs` - compiler-registered dispatch tree.
- `crates/devsweep-cli/src/application/commands/clean.rs` - `clean {scan,plan,preview,execute}`.
- `crates/devsweep-cli/src/application/commands/software/mod.rs` - `software {inventory,plan,preview,uninstall}`.
- `crates/devsweep-cli/src/application/commands/optimize.rs` - `optimize {list,plan,preview,run}`.
- `crates/devsweep-cli/src/application/commands/analyze.rs` - `analyze scan`.
- `crates/devsweep-cli/src/application/commands/status.rs` - `status {snapshot,live}`.
- `crates/devsweep-cli/src/application/commands/history.rs` - `history {list,show}`.
- `crates/devsweep-cli/src/application/commands/protect.rs` - `clean protect {list,add,remove}`.
- `crates/devsweep-cli/src/application/commands/rules.rs` - `clean rules {list,show}`.
- `crates/devsweep-cli/src/application/presentation/mod.rs` - bilingual human renderers over typed
  outcomes.
- `crates/devsweep-cli/tests/cli_contract.rs` - exhaustive parser/help contract.
- `crates/devsweep-cli/tests/five_mode_contract.rs` - shipped-mode reachability and fail-closed roots.
- `crates/devsweep-cli/src/bin/process_fixture.rs` - child/grandchild process-tree test fixture.
- `crates/devsweep-cli/src/tui/mod.rs` - crate-private TUI composition entrypoint (bare `devsweep`).
- `crates/devsweep-cli/src/tui/shell/mod.rs` - five-mode shell navigation and presentation settings.
- `crates/devsweep-cli/src/tui/modes/` - Clean, Software, Optimize, Analyze, and Status adapters.
- `crates/devsweep-cli/src/tui/terminal.rs` - raw-mode and alternate-screen lifecycle.
- `crates/devsweep-cli/src/tui/display.rs` - pure presentation shared by state and rendering.
- `crates/devsweep-cli/src/tui/app/` - app state, reducer transitions, selection, jobs, and tests.
- `crates/devsweep-cli/src/tui/runtime/mod.rs` - event loop, effects, channels, cancellation, and
  cleanup single-flight.
- `crates/devsweep-cli/src/tui/runtime/workers.rs` - worker execution and event translation.
- `crates/devsweep-cli/src/tui/runtime/mailbox.rs` - latest scan observation and bounded ordered worker lifecycle delivery.
- `crates/devsweep-cli/src/tui/runtime/tests.rs` - fake-service and runtime behavior tests.
- `crates/devsweep-cli/src/tui/render/` - pure layouts, views, overlays, themes, and render tests.
- `crates/devsweep-cli/src/tui/support/` - History, Protection, and Rules surfaces.
- `crates/devsweep-cli/src/tui/test_support.rs` - test-only keys, plans, and render helpers.

### Desktop Shell

- `desktop/src-tauri/src/lib.rs` - Tauri application composition and frozen invoke names.
- `desktop/src-tauri/src/async_runtime.rs` - bounded asynchronous command workers, with a separate unchanged blocking executor.
- `desktop/src-tauri/src/commands.rs` - shared IPC helpers (scan start/cancel, presentation
  settings, protections).
- `desktop/src-tauri/src/scan.rs` - single-flight scan state, cancellation, progress forwarding and acknowledged delivery.
- `desktop/src-tauri/src/scan_preferences.rs` - explicit root selection, session validation and persistence IPC.
- `desktop/src-tauri/src/clean.rs` - Clean-mode IPC.
- `desktop/src-tauri/src/software.rs` - Software-mode IPC.
- `desktop/src-tauri/src/optimize.rs` - Optimize-mode IPC.
- `desktop/src-tauri/src/analyze.rs` - Analyze-mode IPC.
- `desktop/src-tauri/src/status.rs` - Status-mode IPC.
- `desktop/src-tauri/src/fonts.rs` - main-only, read-only DirectWrite family catalogue.
- `desktop/src-tauri/src/desktop_preferences.rs` - ordered V2 preference commits and events.
- `desktop/src-tauri/src/tray.rs` - tray icon, menu, tooltip, HUD window toggle, and app exit.
- `desktop/src-tauri/src/hud.rs` - `HudSampler` (2 s while the HUD is visible) and the `hud-status` event type.
- `desktop/src-tauri/src/support.rs` - History, Protection, and Rules IPC.
- `desktop/src-tauri/src/error.rs` - centralized structured command-error mapping.
- `desktop/src/App.tsx` - React shell host: locale gate, operation coordinator, and AppShell.
- `desktop/src/lifecycle.ts` - window controls, close coordination and native decorative-presentation visibility observation.
- `desktop/src/app-shell/` - five-mode registry, routing, and accessible shell chrome.
- `desktop/src/modes/` - Clean, Software, Optimize, Analyze, and Status workbenches.
- `desktop/src/hud/` - second Vite entry (`desktop/hud.html`) for the tray HUD window.
- `desktop/src/support/` - History, Protection, and Rules pages.
- `desktop/src/pages/` - remaining Clean workflow views composed by `CleanWorkbench` (scan,
  preview, review, execute). Not the primary five-mode surface.
- `desktop/src/api/` - generated IPC types, closed-world runtime decoders, the
  production Tauri bridge, and controlled fixture replay.
- `desktop/src/state/` - reducer-owned workflow state, digest invalidation, and
  derived selection/capacity rules.
- `desktop/src/components/` - shared table, evidence, error, and confirmation widgets.
- `desktop/src/stage/Planet.tsx` - decorative renderer lifecycle, motion preferences and fallback.
- `desktop/src/stage/earth-renderer.ts` - bounded geographic Clean WebGL renderer; other modes use `planet-renderer.ts`.
- `desktop/src/stage/earth-texture.ts` - asynchronous bounded local texture loading.
- `desktop/public/earth/` and `desktop/public/THIRD_PARTY_NOTICES.txt` - packaged Earth assets and exact redistribution evidence.
- `desktop/scripts/prepare-earth-assets.py` - offline reproducible Earth asset conversion.

### GPUI desktop (`desktop-gpui/`, standalone workspace)

- `desktop-gpui/src/main.rs` - native entry; loads preferences, then `view::run`.
- `desktop-gpui/src/ui.rs` - design tokens (dark/light/high contrast), components, packaged icon table.
- `desktop-gpui/src/view.rs` - `ShellView`: actions, key bindings, top bar, status bar, background-job dispatch; `panel_entries` is a test-only text projection.
- `desktop-gpui/src/view/page_*.rs` - Clean, Software/Optimize, Analyze, Status, and Settings/support pages.
- `desktop-gpui/src/shell.rs` - `Session` state machine and the generation-guarded `BackgroundJob` queue.
- `desktop-gpui/src/clean_workbench.rs`, `clean_ai.rs`, `clean_map.rs` - Clean review, AI subview, disk-map projection.
- `desktop-gpui/src/software_review.rs`, `optimize_board.rs`, `workbench_review.rs` - Software/Optimize review models; host reads only in `load_host`.
- `desktop-gpui/src/analyze*.rs`, `status_panel.rs` - Analyze board/window and Status projections.
- `desktop-gpui/assets/` - beaver master PNG/ICO, licensed Earth still, original `icons/*.svg`.
- `desktop-gpui/vendor/gpui_windows/` - patched GPUI Windows backend.
- Linux review build: `cargo build --features linux-preview` (X11 backend, not a delivery target).

### Performance evidence tools

- `tools/measure-readonly.py` - isolated read-only CLI/native workloads and task-owned process-tree sampling.
- `tools/inspect-native-readonly.mjs` - packaged native-page metadata/screenshot capture through an explicit loopback debug port.
- `tools/planet-render-benchmark.mjs` and `desktop/vite.planet-benchmark.config.ts` - production Planet comparison and visual matrix.
- `tools/clean-render-benchmark.mjs` and `desktop/vite.clean-benchmark.config.ts` - full-report Clean selection/preview measurements and pagination visuals.
- `tools/native-clean-smoke.mjs` - owned packaged-app offline Earth, explicit-root UI and complete CLI/Tauri report parity smoke; no cleanup execution.
- `tools/native-earth-smoke.mjs` - packaged Earth scale/locale captures and native route/minimize resource lifecycle checks.
- `tools/native-operation-smoke.mjs` - one read-only Software/Optimize/Analyze UI and IPC sample per invocation for alternating builds.
- `tools/native-scan-consumer.mjs` - native paused-consumer scan delivery and cancellation evidence.
- `tools/native-status-lifecycle.mjs` and `tools/summarize-native-status.py` - same-PID Status stop and HUD visibility evidence with separate post-stop CPU accounting.
- `tools/status-sink-smoke.py` - release CLI slow-reader/broken-pipe and create-new JSON output checks with natural-exit evidence.
- `tools/provider-golden-parity.py` - isolated repaired-provider B3/C2 release CLI envelope parity, fixture preservation and offline semantic reassessment.
- `tools/owned-console-smoke.py` - hidden owned-console Status signal/file and TUI input checks; keeps unsupported resize and failed attempts explicit.
- `tools/analyze-layout-benchmark.mjs` - deterministic inherited Analyze layout fixture and percentile gate.
- `tools/run-analyze-react-commit.mjs` - production Analyze component commit/DOM gate in an isolated browser.

The dependency direction starts at `main -> devsweep::run -> application ->
devsweep-core`. Core modules never import CLI or TUI code. TUI render code
remains read-only and backend discovery layers remain non-mutating.

## Agent skill packages

- `skills/devsweep-inspect/` - Production agent skill that inspects with the
  globally installed DevSweep CLI, recommends cleanup, and may execute a
  confirmed Cleanup Plan. Recommend-only until the user confirms a displayed list.

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
- `node_modules/` - documentation-site dependencies from root `npm ci`.
- `desktop/node_modules/` and `desktop/dist/` - desktop dependency and web build output.
- `docs/.vitepress/dist/` and `docs/.vitepress/.temp/` - VitePress build output.
- `__pycache__/` and `*.pyc` - Python tool and test bytecode.
- Do not add nested guidance for generated, vendored, dependency, cache, or
  build-output directories.
