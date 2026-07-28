# Directory Structure

> How backend code is organized in this project.

---

## Overview

`devsweep` is currently a single Rust crate with a flat `src/` layout. Keep the
flat layout while modules are small. Split a module into a directory only when
the module has multiple cohesive submodules and the split removes real
complexity.

The backend boundary owns CLI parsing, cleanup-plan domain types, project
scanning, configuration/logging setup, execution/audit code, and provider
code. The TUI rendering boundary lives in `src/tui/` and the frontend spec.

---

## Directory Layout

```text
src/
├── main.rs        # Binary entrypoint and command dispatch
├── lib.rs         # Public module exports for tests and future consumers
├── cli.rs         # clap command definitions only
├── config.rs      # process-wide initialization such as tracing
├── executor.rs    # dry-run, command/trash execution, and audit JSONL
├── fs_size.rs     # parallel tree sizing with symlink safety
├── model.rs       # v2 untrusted JSON DTOs and internal scan/action types
├── path_identity.rs # lexical canonical path identity for plan validation
├── path_safety.rs # current-exe containment guard helpers
├── plan_validation.rs # UntrustedPlan -> opaque ValidatedPlan boundary + digest
├── providers.rs   # global tool-cache discovery (npm/pip/pnpm/yarn/cargo/...)
├── ranking.rs     # cleanup-plan ordering and conservative default-selection pass
├── registry.rs    # trusted rule/intent -> action reconstruction
├── rules.rs       # declarative rule tables + catalogue aggregation & row formatting
├── scanner.rs     # marker-first project discovery, non-mutating
├── sweep.rs       # scan→merge→rank pipeline owner (sole ranking call site)
└── tui/           # ratatui TUI module (see frontend spec)
```

There is no standalone `tests/` directory yet. Current tests live next to the
module that owns the behavior, such as `model::tests`, `scanner::tests`, and
`tui::tests`.

---

## Module Organization

- Put CLI argument shape in `src/cli.rs`. Keep command parsing structs free of
  filesystem scanning or cleanup side effects.
- Put JSON-facing cross-layer data in `src/model.rs`. `UntrustedPlan`,
  `UntrustedTarget`, and `CleanupIntent` are the versioned persisted contract;
  they describe observed facts and intent, never executable argv or an
  authoritative cleanup path. `CleanupPlan`, `CleanTarget`, and `CleanAction`
  are internal typed scan/execution values, not serde input.
- Put untrusted-plan validation, canonical digesting, and the opaque
  `ValidatedPlan` in `src/plan_validation.rs`; put rule/intent action
  reconstruction in `src/registry.rs`. Keep lexical identity normalization in
  `src/path_identity.rs` so CLI and TUI consume one trust boundary.
- Put cleanup-plan post-processing in `src/ranking.rs`. Ranking may reorder
  targets, calculate size/age scores, and make auto-selection more conservative
  from existing `CleanTarget` fields; it must not inspect new filesystem state
  or execute cleanup behavior.
- Put project discovery in `src/scanner.rs`. Scanner code creates
  `CleanTarget` values only; it must not delete files, move to trash, or run
  cleanup commands.
- Put execution behavior in `src/executor.rs`. It consumes only a
  `ValidatedPlan`, runs exactly the explicit `ExecutionRequest.selected` set,
  keeps registry-reconstructed command program/argv separate, delegates trash
  moves through a small runner boundary, and owns audit JSONL writes.
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
