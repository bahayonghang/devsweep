# Reorganize Rust source architecture

## Goal

Make the Rust source tree easier to navigate, review, test, and extend by
replacing oversized or mixed-responsibility modules with cohesive modules and
by aligning implementation details with the repository's Rust conventions.

The intended outcome is an internal architecture refactor of a binary-oriented
crate. User-visible CLI, TUI, cleanup-plan, safety, and execution behavior must
remain compatible, while existing Rust module paths may be reorganized or
removed to produce a smaller deliberate crate interface.

## Background

- The repository is a single Rust crate whose source currently combines
  command dispatch, application orchestration, domain contracts, scanning,
  execution, safety policy, process management, and a ratatui frontend.
- `code_map.md` identifies `src/main.rs` as both CLI dispatch and top-level
  command handlers, and `src/tui.rs` as the entrypoint, state, update/effect
  loop, worker event handling, rendering helpers, and tests.
- Cleanup is dry-run by default, permanent delete remains disabled, scanners
  and model code may only create plans, cleanup commands must keep program and
  argv separate, Cargo home is inspect-only, and Docker cleanup is out of
  scope. A refactor must preserve these contracts.
- The existing split is structurally valid and passes the current quality gate,
  but several modules have grown beyond the responsibilities recorded in the
  maintained code map and Trellis specs.

## Confirmed Facts

- The production portions of the largest files are approximately 2,012 lines
  in `src/tui/app.rs`, 1,928 in `src/tui/render.rs`, 1,225 in
  `src/safety.rs`, 1,043 in `src/scanner.rs`, and 1,015 in
  `src/executor.rs`; their remaining lines are mostly colocated tests.
- `App` owns target and inventory state, selection projections, filters,
  overlays, jobs, logs, cleanup progress, scan snapshots, and quit coordination
  in one struct (`src/tui/app.rs:33`), while one reducer handles both keyboard
  and worker events (`src/tui/app.rs:110`).
- The TUI renderer owns every screen, modal, style, layout, formatter, and
  display sanitizer behind one entry function (`src/tui/render.rs:63`) and
  contains more than eighty production render/format helpers.
- Rule metadata is declared in scanner/provider implementation modules and
  re-imported by the catalogue and registry. `rules` imports `providers` and
  `scanner` (`src/rules.rs:22`), while both implementation modules import
  `rules` (`src/providers.rs:20`, `src/scanner.rs:20`), creating a conceptual
  dependency cycle.
- `scanner` imports Cargo metadata probe types from `safety`
  (`src/scanner.rs:21`), although those probe types are scanner discovery
  infrastructure inside `src/safety.rs:1065`; `safety` in turn depends on rule
  tables (`src/safety.rs:13`).
- JSON/report model code depends on process-runner result types solely to build
  scan diagnostics (`src/model.rs:5`), reversing the desired direction between
  data contracts and runtime infrastructure.
- `src/main.rs` owns five command handlers plus plan-file decoding and output
  formatting (`src/main.rs:32` through `src/main.rs:255`), so the binary
  entrypoint is not a thin composition layer.
- `src/lib.rs:1` through `src/lib.rs:18` publicly exports nearly every internal
  module. Repository-wide search finds no consumer of those module paths other
  than this package's own binary (`src/main.rs:5`), but external consumers
  cannot be ruled out from repository evidence.
- Historical `design.md` already anticipated directory modules for scanners,
  providers, execution, filesystem infrastructure, and TUI views. Its module
  direction is useful evidence, but its obsolete permanent-delete and Docker
  ideas are not requirements.
- Baseline `just ci` passes on 2026-08-02: format, check, 221 tests (216 library
  and 5 binary), and Clippy with warnings denied.

## Requirements

- Inventory every production module under `src/`, its responsibilities,
  dependencies, public surface, and colocated tests before selecting module
  boundaries.
- Define a cohesive target module tree based on domain and responsibility,
  following idiomatic Rust module visibility and naming conventions.
- Split mixed-responsibility or oversized modules only where the split creates
  an independently understandable boundary; avoid speculative abstraction.
- Reduce unnecessary public visibility, duplicated orchestration, and
  non-idiomatic patterns identified by compiler or Clippy evidence.
- Treat the package as binary-oriented: retain only the Rust crate interface
  needed by the binary and deliberate external entrypoints. Do not add
  compatibility re-exports for current internal module paths.
- Preserve serialized plan compatibility, CLI command/flag behavior, TUI
  workflows, filesystem safety rules, audit semantics, and supported-platform
  behavior.
- Keep tests close to their owning behavior or in focused integration-test
  modules, and retain meaningful regression coverage through file moves.
- Update maintained navigation or architecture documentation when source paths
  or responsibilities change.
- Keep generated `target/` and `dist/` outputs outside version control and
  preserve unrelated working-tree changes.
- Do not add pass-through layers or one-adapter abstraction traits merely to
  reduce file length. Each new module must hide meaningful implementation
  complexity behind a smaller interface or establish clear code ownership.

## Key Decisions

- Treat `devsweep` as a binary-oriented crate and expose `devsweep::run()` as
  the deliberate Rust interface.
- Do not preserve source compatibility for the current accidental public
  module paths; do not add compatibility shims or re-exports.
- Preserve all CLI, TUI, serialized data, safety, audit, cancellation, and
  supported-platform behavior.
- Use deep modules and real production/test seams; do not optimize for a target
  file count or line count.
- Implement through sequential child tasks. The parent owns requirements and
  final integration review but no direct product-code edits.

## Task Map

1. `08-03-untangle-core-contracts-rules`: centralize model/plan/rule/Cargo
   metadata ownership and remove dependency reversals.
2. `08-03-modularize-backend-runtime-discovery`: depends on child 1; modularize
   filesystem, process, scan, inventory, execution, safety, and audit.
3. `08-03-decompose-tui-internals`: depends on child 2; split TUI state/update,
   runtime, display, and render implementation.
4. `08-03-narrow-entrypoint-architecture-docs`: depends on children 1-3; make
   the crate interface deliberate, reconcile docs/specs, and run final
   integration acceptance.

## Acceptance Criteria

- [ ] The final source tree has documented, cohesive ownership boundaries and
      no production module retains unrelated application, domain, I/O, and UI
      responsibilities merely for historical convenience.
- [ ] Binary entrypoints are thin composition/dispatch layers; reusable logic
      is owned by library modules with the narrowest practical visibility.
- [ ] `src/lib.rs` exposes a deliberate minimal interface instead of publicly
      exporting implementation modules solely for the package's own binary.
- [ ] Existing CLI commands and flags, serialized cleanup-plan contract, TUI
      user flows, safety authorization funnel, and execution/audit behavior
      remain compatible.
- [ ] Scanner and model layers still cannot execute cleanup; permanent delete
      remains disabled; Cargo home remains inspect-only; Docker cleanup is not
      introduced.
- [ ] Existing tests remain meaningful after relocation, and focused tests are
      added for any boundary whose behavior could regress during extraction.
- [ ] `code_map.md` and any directly affected architecture guidance match the
      final module tree.
- [ ] `just ci` passes on the completed refactor.

## Out of Scope

- New cleanup providers, new CLI/TUI features, schema changes, Docker cleanup,
  and enabling permanent deletion.
- Unrelated dependency upgrades, broad formatting churn, or performance work
  without evidence tied to the architecture refactor.
- Compatibility shims or re-exports for the current Rust module paths. This
  refactor may be source-breaking for unverified downstream Rust callers.
