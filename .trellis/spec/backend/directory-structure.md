# Directory Structure

> How backend code is organized in this project.

---

## Overview

`devsweep` is currently a single Rust crate with a flat `src/` layout. Keep the
flat layout while modules are small. Split a module into a directory only when
the module has multiple cohesive submodules and the split removes real
complexity.

The backend boundary owns CLI parsing, cleanup-plan domain types, project
scanning, configuration/logging setup, and future execution/audit code. The TUI
rendering boundary lives in `src/tui.rs` and the frontend spec.

---

## Directory Layout

```text
src/
├── main.rs     # Binary entrypoint and command dispatch
├── lib.rs      # Public module exports for tests and future consumers
├── cli.rs      # clap command definitions only
├── config.rs   # process-wide initialization such as tracing
├── model.rs    # serializable cleanup plan and domain contract
├── scanner.rs  # marker-first project discovery, non-mutating
└── tui.rs      # ratatui placeholder/rendering boundary
```

There is no standalone `tests/` directory yet. Current tests live next to the
module that owns the behavior, such as `model::tests`, `scanner::tests`, and
`tui::tests`.

---

## Module Organization

- Put CLI argument shape in `src/cli.rs`. Keep command parsing structs free of
  filesystem scanning or cleanup side effects.
- Put serializable cross-layer data in `src/model.rs`. `CleanupPlan`,
  `CleanTarget`, `CleanAction`, and evidence/risk enums are the plan contract
  shared by CLI, scanner, future executor, and TUI.
- Put project discovery in `src/scanner.rs`. Scanner code creates
  `CleanTarget` values only; it must not delete files, move to trash, or run
  cleanup commands.
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
  `CleanTarget`, `CleanAction`, `RiskLevel`, `Evidence`.
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
- Do not put execution behavior into scanner modules. Execution belongs to the
  future execution-engine task.
