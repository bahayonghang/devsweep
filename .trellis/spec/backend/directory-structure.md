# Directory Structure

> How backend code is organized in this project.

---

## Overview

`devsweep` is a Cargo workspace with a reusable `devsweep-core` crate, a
binary-oriented `devsweep-cli` crate, and a Tauri shell under
`desktop/src-tauri`. The CLI library keeps the existing
`devsweep::run()` interface; core exposes only the scanner, plan, execution,
inventory, rule, cancellation, and service types required by current callers.
Keep other implementation items private by default.

The application boundary owns CLI parsing, process-wide tracing setup, command
dispatch, saved-input decoding, and user-facing output. Backend modules own the
cleanup domain, scanning, inventory, process execution, safety, and audit
behavior. The TUI rendering boundary lives in `crates/devsweep-cli/src/tui/`
and the frontend spec.

---

## Directory Layout

```text
crates/
├── devsweep-core/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs              # supported reusable module boundary
│   │   ├── services.rs         # frontend-neutral traits and adapters
│   │   ├── cargo_metadata.rs   # neutral Cargo probing and diagnostics
│   │   ├── execution/          # executor, audit, runners, live safety
│   │   ├── filesystem/         # identity, containment, reparse, sizing
│   │   ├── inventory/          # read-only capacity observations
│   │   ├── model/              # plan and scan domain values
│   │   ├── plan/               # validation and private canonical digest
│   │   ├── process/            # bounded process runner and cancellation
│   │   ├── rules/              # catalogue and private action registry
│   │   └── scan/               # project/global discovery and ranking
│   └── tests/
│       └── public_api.rs       # external-crate supported-surface proof
└── devsweep-cli/
    ├── Cargo.toml
    └── src/
        ├── main.rs              # minimal delegation to devsweep::run
        ├── lib.rs               # public run function; internals private
        ├── application/
        │   ├── cli.rs           # frozen Clap grammar and help manifests
        │   ├── commands/        # compiler-registered domain handlers
        │   ├── presentation/    # compiler-registered human renderers
        │   ├── output.rs        # V1 envelopes, sinks, stream lifecycle
        │   └── mod.rs           # tracing, TTY routing, typed dispatch
        ├── i18n/                # locale, catalogues, metadata, formatting
        ├── bin/process_fixture.rs # ProcessRunner child-tree fixture
        └── tui/                 # ratatui UI (see frontend spec)
desktop/
├── src/                         # React/TypeScript desktop frontend
└── src-tauri/
    └── src/
        ├── lib.rs               # Tauri composition and command registration
        ├── commands.rs          # async IPC bridge into devsweep-core
        ├── scan.rs              # scan single-flight/cancel/event orchestration
        └── error.rs             # tagged IPC error mapping
```

Behavior tests live with the owner, including `application::cli::tests`,
`application::commands::tests`,
`model::plan::tests`, `scan::project::tests`, `execution::tests`, and the
dedicated `tui/app/tests.rs`, `tui/runtime/tests.rs`, and
`tui/render/tests.rs` modules. `crates/devsweep-core/tests/public_api.rs` is the
one integration test proving use from an external crate boundary.

---

## Module Organization

- Put application composition under `crates/devsweep-cli/src/application/`.
  `cli.rs` owns the private Clap grammar, contextual help, and deterministic
  reference manifest. `commands/mod.rs` owns the stable compiler registration
  tree; each later domain task fills only its assigned handler file.
  `presentation/mod.rs` does the same for human renderers. `output.rs` owns
  create-new sinks, V1 envelopes, exit classification, and stream
  cancel/join/terminal behavior. `mod.rs` initializes tracing, applies bare-TUI
  and `status live` TTY rules, retains Clap process-exit semantics, and performs
  typed dispatch. Backend and TUI modules must not import `application`.
- Put CLI-owned locale selection, embedded catalogue validation, message
  metadata, interpolation safety, and binary unit formatting under
  `crates/devsweep-cli/src/i18n/`. Core keeps a separate closed presentation
  language wire tag and never imports the CLI crate. Shell adapters map that tag
  exhaustively; they do not duplicate catalogue or locale rules.
- Put JSON-facing and internal domain values under `crates/devsweep-core/src/model/`. `plan.rs` owns
  `UntrustedPlan`, `UntrustedTarget`, `CleanupIntent`, `CleanupPlan`,
  `CleanTarget`, and `CleanAction`; `scan.rs` owns `ScanReport`, `ScanHealth`,
  diagnostics, totals, and sizing warnings. Model code imports only standard
  library and Serde concerns, never process, scan, execution, filesystem, or
  TUI implementations.
- Put untrusted-plan validation and the opaque `ValidatedPlan` in
  `crates/devsweep-core/src/plan/mod.rs`; keep canonical digest implementation private in
  `crates/devsweep-core/src/plan/digest.rs`. Keep lexical identity normalization, live identity,
  containment, no-follow reparse probing, and bounded sizing under
  `crates/devsweep-core/src/filesystem/` so every caller consumes one filesystem safety boundary.
- Put every built-in rule id, catalogue fact, and declarative rule table in
  `crates/devsweep-core/src/rules/definitions.rs`. `rules/mod.rs` exposes catalogue/lookup
  operations, while private `rules/registry.rs` reconstructs trusted
  actions for plan validation. Scanner and provider implementations consume
  rule facts; rules must not import those implementations.
- Put Cargo metadata scope/probe types, process-result classification, and JSON
  parsing in neutral `crates/devsweep-core/src/cargo_metadata.rs`.
  `scan/project/cargo.rs` owns scan-lifetime result caching and
  `execution/safety/mod.rs` owns live
  authorization policy; both use this module without importing one another.
- Put the complete discovery pipeline under `crates/devsweep-core/src/scan/`. Project traversal,
  Cargo caching, dedupe, and reviewed rescan live in `scan/project/`; global
  provider probing and target construction live in `scan/global/`. Both return
  unranked observations. `Sweeper` in `scan/mod.rs` owns merge, cumulative
  progress, and the sole production call to private `scan/ranking.rs`.
- Put storage-only inspection under `crates/devsweep-core/src/inventory/`. `inventory/mod.rs`
  produces typed observations; private `inventory/pnpm.rs` owns bounded pnpm
  reference inspection. Inventory never creates a `CleanupPlan` or reaches
  execution.
- Put execution behavior under `crates/devsweep-core/src/execution/`. `Executor` consumes only a
  `ValidatedPlan`, runs exactly the explicit `ExecutionRequest.selected` set,
  keeps rules-reconstructed command program/argv separate, delegates trash
  moves through a small runner boundary, and orders authorization, durable
  started audit, dispatch, and terminal audit. `execution/audit.rs` is the sole
  owner of audit JSONL and replay; `execution/safety/` owns live authorization
  and user protections. Every side effect must receive an `AuthorizedAction`
  from `SafetyPolicy::authorize`; do not reintroduce a bypass path.
- Put all external-process spawning under `crates/devsweep-core/src/process/`. Global provider probes,
  Cargo metadata, inventory, and the execution command runner must call
  `ProcessRunner` rather than
  `Command::output()`. The runner owns timeout, output caps with tail capture,
  neutral cwd by default, process-tree termination (Windows Job Object / Unix
  process group), typed status, and `sanitize_process_output` for display and
  audit. Keep program and argv separate; never shell-compose or shell out to
  `kill` / `taskkill`. Cancellation is observed through `CancelObserver`; the
  shared token type is owned by the true-cancellation task.
- Keep Software support actions in `crates/devsweep-core/src/software/`:
  `updates.rs` owns the read-only `winget upgrade --disable-interactivity`
  probe and its column-position table parser; `startup.rs` owns startup
  listing and the current-user `Explorer\StartupApproved` write;
  `leftovers.rs` owns leftover discovery, the leftover plan digest, and the
  Recycle Bin move. Registry access stays in the `arp.rs` `native_registry`
  helper, and every support record goes through `execution/audit.rs`.
- Keep `crates/devsweep-cli/src/main.rs` to `devsweep::run()` delegation only. It must not parse
  arguments, initialize tracing, decode files, format output, or dispatch
  individual commands.
- Keep `crates/devsweep-cli/src/lib.rs` binary-oriented: `run` is its deliberate
  external Rust interface and CLI/TUI modules remain private. Keep
  `crates/devsweep-core/src/lib.rs` limited to supported reusable modules, and
  promote only the transitive type closure required by current CLI and service
  signatures. Keep `unreachable_pub` enabled in both crates and do not add
  compatibility re-exports for old implementation paths.
- Keep `desktop/src-tauri` as a thin adapter over `devsweep-core`; IPC payloads
  use core Serde models directly. Do not add parallel DTOs or import the CLI
  crate. Long-running core calls run off the async runtime thread, and the
  adapter owns only command state, event forwarding, app-data paths, and typed
  boundary error mapping.

Example from `crates/devsweep-cli/src/application/commands/mod.rs`:

```rust
mod analyze;
mod clean;
mod history;
mod optimize;
mod protect;
mod rules;
mod software;
mod status;
```

---

## Naming Conventions

- Rust module files use `snake_case`.
- Domain types use explicit names that describe cleanup semantics:
  `UntrustedPlan`, `CleanupIntent`, `ValidatedPlan`, `CleanTarget`,
  `CleanAction`, `RiskLevel`, and `Evidence`.
- Scanner rule identifiers use dotted lowercase strings such as
  `rust.target`, `node.node_modules`, and `python.pytest_cache`.
- CLI command structs use `<CommandPath>Command`, for example
  `CleanScanCommand`, `SoftwareInventoryCommand`, and `StatusSnapshotCommand`.

---

## Common Mistakes

- Do not add a service/repository layer until there is real persistence or a
  second backend consumer.
- Do not duplicate cleanup-plan fields in CLI or TUI code. Import the model
  types instead.
- Do not put execution behavior under `crates/devsweep-core/src/scan/`. Scan code produces action
  data; `crates/devsweep-core/src/execution/` is the only module that may run cleanup commands or
  move targets to trash.
