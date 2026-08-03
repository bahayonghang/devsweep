# Directory Structure

> How backend code is organized in this project.

---

## Overview

`devsweep` is a single, binary-oriented Rust crate. Its only external Rust
interface is `devsweep::run()`. Keep implementation modules private by default;
use a module directory when it hides multiple cohesive owners behind one
crate-internal interface, as `application/`, `model/`, `plan/`, and `rules/`
do.

The application boundary owns CLI parsing, process-wide tracing setup, command
dispatch, saved-input decoding, and user-facing output. Backend modules own the
cleanup domain, scanning, inventory, process execution, safety, and audit
behavior. The TUI rendering boundary lives in `src/tui/` and the frontend spec.

---

## Directory Layout

```text
src/
├── main.rs        # Minimal binary delegation to devsweep::run
├── lib.rs         # Public run function; all implementation modules private
├── cargo_metadata.rs # neutral Cargo scope probing, parsing, and diagnostics
├── application/
│   ├── mod.rs     # tracing initialization, Clap parse, typed dispatch
│   ├── cli.rs     # private Clap command definitions and parser tests
│   └── commands.rs # command handlers, input decoding, output, boundary tests
├── bin/
│   └── process_fixture.rs # child/grandchild fixture for ProcessRunner tests
├── execution/
│   ├── mod.rs     # Executor interface and authorize/audit/dispatch orchestration
│   ├── audit.rs   # durable JSONL journal and unconfirmed-start replay
│   ├── command.rs # ProcessRunner command adapter and trash adapter
│   └── safety/
│       ├── mod.rs # SafetyPolicy authorize funnel and live revalidation
│       └── protections.rs # versioned atomic user-protection persistence
├── filesystem/
│   ├── mod.rs     # shared identity, containment, reparse, and sizing interface
│   ├── identity.rs # lexical normalization and live file identity
│   ├── containment.rs # containment and current-executable checks
│   ├── reparse.rs # no-follow platform reparse/symlink probing
│   └── sizing.rs  # bounded cancelable tree sizing
├── inventory/
│   ├── mod.rs     # read-only capacity observation orchestration
│   └── pnpm.rs    # bounded pnpm reference and orphan evidence inspection
├── model/
│   ├── mod.rs     # model interface
│   ├── plan.rs    # v2 plan DTOs and internal cleanup domain values
│   └── scan.rs    # scan reports, health, diagnostics, totals, sizing warnings
├── plan/
│   ├── mod.rs     # UntrustedPlan -> opaque ValidatedPlan boundary
│   └── digest.rs  # private canonical manifest and SHA-256 digest
├── process/
│   ├── mod.rs     # ProcessRunner request/result and execution policy interface
│   ├── cancel.rs  # cooperative cancellation observers
│   ├── capture.rs # bounded stream capture and output sanitization
│   └── tree.rs    # Windows Job Object / Unix process-group lifecycle
├── rules/
│   ├── mod.rs     # catalogue, lookup, formatting, and trust interface
│   ├── definitions.rs # sole owner of built-in rule facts and ids
│   └── registry.rs # private trusted intent-to-action reconstruction
├── scan/
│   ├── mod.rs     # Sweeper merge/progress interface and sole ranking call site
│   ├── ranking.rs # pure ordering and conservative default-selection policy
│   ├── project/
│   │   ├── mod.rs # marker-first traversal and reviewed rescan
│   │   ├── cargo.rs # scan-lifetime Cargo workspace cache
│   │   └── dedupe.rs # root and target footprint dedupe
│   └── global/
│       ├── mod.rs # global provider target construction
│       └── probe.rs # bounded read-only provider command probing
└── tui/           # ratatui TUI module (see frontend spec)
```

There is no standalone `tests/` directory. Tests live with the owner, including
`application::cli::tests`, `application::commands::tests`,
`model::plan::tests`, `scan::project::tests`, `execution::tests`, and the
dedicated `tui/app/tests.rs`, `tui/runtime/tests.rs`, and
`tui/render/tests.rs` modules.

---

## Module Organization

- Put application composition under `src/application/`. `cli.rs` owns only
  private Clap types. `commands.rs` owns command handlers, saved plan/report
  decoding, and CLI formatting. `mod.rs` initializes tracing, retains
  `Cli::parse()` process-exit semantics, and performs typed dispatch. Backend
  and TUI modules must not import `application`.
- Put JSON-facing and internal domain values under `src/model/`. `plan.rs` owns
  `UntrustedPlan`, `UntrustedTarget`, `CleanupIntent`, `CleanupPlan`,
  `CleanTarget`, and `CleanAction`; `scan.rs` owns `ScanReport`, `ScanHealth`,
  diagnostics, totals, and sizing warnings. Model code imports only standard
  library and Serde concerns, never process, scan, execution, filesystem, or
  TUI implementations.
- Put untrusted-plan validation and the opaque `ValidatedPlan` in
  `src/plan/mod.rs`; keep canonical digest implementation private in
  `src/plan/digest.rs`. Keep lexical identity normalization, live identity,
  containment, no-follow reparse probing, and bounded sizing under
  `src/filesystem/` so every caller consumes one filesystem safety boundary.
- Put every built-in rule id, catalogue fact, and declarative rule table in
  `src/rules/definitions.rs`. `src/rules/mod.rs` exposes catalogue/lookup
  operations, while private `src/rules/registry.rs` reconstructs trusted
  actions for plan validation. Scanner and provider implementations consume
  rule facts; rules must not import those implementations.
- Put Cargo metadata scope/probe types, process-result classification, and JSON
  parsing in neutral `src/cargo_metadata.rs`. `src/scan/project/cargo.rs` owns
  scan-lifetime result caching and `src/execution/safety/mod.rs` owns live
  authorization policy; both use this module without importing one another.
- Put the complete discovery pipeline under `src/scan/`. Project traversal,
  Cargo caching, dedupe, and reviewed rescan live in `scan/project/`; global
  provider probing and target construction live in `scan/global/`. Both return
  unranked observations. `Sweeper` in `scan/mod.rs` owns merge, cumulative
  progress, and the sole production call to private `scan/ranking.rs`.
- Put storage-only inspection under `src/inventory/`. `inventory/mod.rs`
  produces typed observations; private `inventory/pnpm.rs` owns bounded pnpm
  reference inspection. Inventory never creates a `CleanupPlan` or reaches
  execution.
- Put execution behavior under `src/execution/`. `Executor` consumes only a
  `ValidatedPlan`, runs exactly the explicit `ExecutionRequest.selected` set,
  keeps rules-reconstructed command program/argv separate, delegates trash
  moves through a small runner boundary, and orders authorization, durable
  started audit, dispatch, and terminal audit. `execution/audit.rs` is the sole
  owner of audit JSONL and replay; `execution/safety/` owns live authorization
  and user protections. Every side effect must receive an `AuthorizedAction`
  from `SafetyPolicy::authorize`; do not reintroduce a bypass path.
- Put all external-process spawning under `src/process/`. Global provider probes,
  Cargo metadata, inventory, and the execution command runner must call
  `ProcessRunner` rather than
  `Command::output()`. The runner owns timeout, output caps with tail capture,
  neutral cwd by default, process-tree termination (Windows Job Object / Unix
  process group), typed status, and `sanitize_process_output` for display and
  audit. Keep program and argv separate; never shell-compose or shell out to
  `kill` / `taskkill`. Cancellation is observed through `CancelObserver`; the
  shared token type is owned by the true-cancellation task.
- Keep `src/main.rs` to `devsweep::run()` delegation only. It must not parse
  arguments, initialize tracing, decode files, format output, or dispatch
  individual commands.
- Keep `src/lib.rs` binary-oriented: `run` is the deliberate external Rust
  interface and implementation modules remain private. Cross-top-level-module
  collaboration uses `pub(crate)` only where needed; directory children prefer
  `pub(super)` or private items. Keep the crate-level `unreachable_pub` lint
  enabled so the canonical Clippy gate rejects accidental public implementation
  items. Do not add compatibility re-exports for old implementation paths.

Example from `src/application/commands.rs`:

```rust
let options = ScanOptions {
    include_projects: command.projects || !command.global,
    include_global: command.global || !command.projects,
    roots: command.roots.clone(),
};
let report = Sweeper::default().full_scan_report(&options, &mut |_| {})?;
```

---

## Naming Conventions

- Rust module files use `snake_case`.
- Domain types use explicit names that describe cleanup semantics:
  `UntrustedPlan`, `CleanupIntent`, `ValidatedPlan`, `CleanTarget`,
  `CleanAction`, `RiskLevel`, and `Evidence`.
- Scanner rule identifiers use dotted lowercase strings such as
  `rust.target`, `node.node_modules`, and `python.pytest_cache`.
- CLI command structs use `<CommandName>Command`, for example `ScanCommand` and
  `CleanCommand`.

---

## Common Mistakes

- Do not add a service/repository layer until there is real persistence or a
  second backend consumer.
- Do not duplicate cleanup-plan fields in CLI or TUI code. Import the model
  types instead.
- Do not put execution behavior under `src/scan/`. Scan code produces action
  data; `src/execution/` is the only module that may run cleanup commands or
  move targets to trash.
