# Directory Structure

> How backend code is organized in this project.

---

## Overview

`devsweep` is a single Rust crate. Keep small implementation modules flat;
use a module directory when it hides multiple cohesive owners behind one
interface, as `model/`, `plan/`, and `rules/` do.

The backend boundary owns CLI parsing, cleanup-plan domain types, project
scanning, configuration/logging setup, execution/audit code, and provider
code. The TUI rendering boundary lives in `src/tui/` and the frontend spec.

---

## Directory Layout

```text
src/
├── main.rs        # Binary entrypoint and command dispatch
├── lib.rs         # Public module exports for tests and future consumers
├── cargo_metadata.rs # neutral Cargo scope probing, parsing, and diagnostics
├── cli.rs         # clap command definitions only
├── config.rs      # process-wide initialization such as tracing
├── executor.rs    # dry-run, command/trash execution, and audit JSONL
├── fs_size.rs     # parallel tree sizing with symlink safety
├── inventory.rs   # read-only capacity observations and pnpm-store inspection
├── path_identity.rs # lexical canonical path identity + live file identity
├── path_safety.rs # current-exe containment guard helpers
├── process_runner.rs # bounded external command runner (timeout/caps/tree kill)
├── safety.rs      # SafetyPolicy authorize funnel, protections, user list
├── providers.rs   # global tool-cache discovery (npm/pip/pnpm/yarn/cargo/...)
├── ranking.rs     # cleanup-plan ordering and conservative default-selection pass
├── scanner.rs     # marker-first project discovery, non-mutating
├── sweep.rs       # scan→merge→rank pipeline owner (sole ranking call site)
├── bin/
│   └── process_fixture.rs # child/grandchild fixture for ProcessRunner tests
├── model/
│   ├── mod.rs     # model interface
│   ├── plan.rs    # v2 plan DTOs and internal cleanup domain values
│   └── scan.rs    # scan reports, health, diagnostics, totals, sizing warnings
├── plan/
│   ├── mod.rs     # UntrustedPlan -> opaque ValidatedPlan boundary
│   └── digest.rs  # private canonical manifest and SHA-256 digest
├── rules/
│   ├── mod.rs     # catalogue, lookup, formatting, and trust interface
│   ├── definitions.rs # sole owner of built-in rule facts and ids
│   └── registry.rs # private trusted intent-to-action reconstruction
└── tui/           # ratatui TUI module (see frontend spec)
```

There is no standalone `tests/` directory yet. Current tests live next to the
module that owns the behavior, such as `model::plan::tests`, `scanner::tests`, and
`tui::tests`.

---

## Module Organization

- Put CLI argument shape in `src/cli.rs`. Keep command parsing structs free of
  filesystem scanning or cleanup side effects.
- Put JSON-facing and internal domain values under `src/model/`. `plan.rs` owns
  `UntrustedPlan`, `UntrustedTarget`, `CleanupIntent`, `CleanupPlan`,
  `CleanTarget`, and `CleanAction`; `scan.rs` owns `ScanReport`, `ScanHealth`,
  diagnostics, totals, and sizing warnings. Model code imports only standard
  library and Serde concerns, never process, scan, execution, filesystem, or
  TUI implementations.
- Put untrusted-plan validation and the opaque `ValidatedPlan` in
  `src/plan/mod.rs`; keep canonical digest implementation private in
  `src/plan/digest.rs`. Keep lexical identity normalization in
  `src/path_identity.rs` so CLI and TUI consume one trust boundary.
- Put every built-in rule id, catalogue fact, and declarative rule table in
  `src/rules/definitions.rs`. `src/rules/mod.rs` exposes catalogue/lookup
  operations, while private `src/rules/registry.rs` reconstructs trusted
  actions for plan validation. Scanner and provider implementations consume
  rule facts; rules must not import those implementations.
- Put Cargo metadata scope/probe types, process-result classification, and JSON
  parsing in neutral `src/cargo_metadata.rs`. Scanner owns scan-lifetime result
  caching and safety owns live authorization policy; both use this module
  without importing one another.
- Put cleanup-plan post-processing in `src/ranking.rs`. Ranking may reorder
  targets, calculate size/age scores, and make auto-selection more conservative
  from existing `CleanTarget` fields; it must not inspect new filesystem state
  or execute cleanup behavior.
- Put project discovery in `src/scanner.rs`. Scanner code creates
  `CleanTarget` values only; it must not delete files, move to trash, or run
  cleanup commands.
- Put storage-only inspection in `src/inventory.rs`. Inventory produces typed
  observations and inspect-only findings; it never creates a `CleanupPlan` or
  reaches the executor.
- Put execution behavior in `src/executor.rs`. It consumes only a
  `ValidatedPlan`, runs exactly the explicit `ExecutionRequest.selected` set,
  keeps rules-reconstructed command program/argv separate, delegates trash
  moves through a small runner boundary, and owns audit JSONL writes.
- Put the central execution-time safety funnel in `src/safety.rs`. Command and
  trash side effects must receive an `AuthorizedAction` from
  `SafetyPolicy::authorize`; do not reintroduce a bypass path.
- Put all external-process spawning in `src/process_runner.rs`. Providers and
  the executor command runner must call this port rather than
  `Command::output()`. The runner owns timeout, output caps with tail capture,
  neutral cwd by default, process-tree termination (Windows Job Object / Unix
  process group), typed status, and `sanitize_process_output` for display and
  audit. Keep program and argv separate; never shell-compose or shell out to
  `kill` / `taskkill`. Cancellation is observed through `CancelObserver`; the
  shared token type is owned by the true-cancellation task.
- Put binary orchestration in `src/main.rs`. It wires `clap` input to module
  entrypoints and handles user-facing command output.
- Export a module from `src/lib.rs` only when integration tests, the binary, or
  a future crate boundary need it.

Example from `src/main.rs`:

```rust
let include_projects = command.projects || !command.global;
let plan = if include_projects {
    ProjectScanner::new().scan_roots(&command.roots)?
} else {
    CleanupPlan::empty()
};
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
- Do not put execution behavior into scanner modules. Scanner produces action
  data; executor is the only module that may run commands or move targets to
  trash.
