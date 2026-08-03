# Narrow entrypoint and reconcile architecture docs

## Goal

Finish the architecture migration by making `devsweep::run()` the deliberate
external Rust interface, moving command orchestration behind it, removing
accidental public implementation exposure, and reconciling every maintained
architecture guide with the implemented source tree.

## Parent And Ordering

- Parent: `08-02-reorganize-rust-source-architecture`.
- Depends on completed and archived core, backend, and TUI children.
- This is the final implementation child and owns cross-child structural and
  documentation integration before the parent acceptance review.

## Confirmed Decisions

- The user selected a binary-oriented crate interface and explicitly allowed
  current Rust module paths to break.
- CLI commands/flags, TUI behavior, serialized JSON, audit semantics, safety,
  and supported-platform runtime behavior remain compatible.
- No compatibility re-exports or shim modules are required for unverified
  downstream Rust callers.

## Requirements

- Create a private application module that owns tracing initialization, Clap
  parsing, command dispatch, command handlers, saved-plan/report input decoding,
  and CLI output formatting.
- Reduce `src/main.rs` to delegation through `devsweep::run()`.
- Reduce the external library interface to the deliberate `run` entrypoint.
  Internal modules and types use private or `pub(crate)` visibility according
  to actual cross-module needs.
- Remove all temporary public or crate-private compatibility re-exports left by
  earlier children unless the final implementation genuinely consumes them.
- Preserve CLI help, defaults, command output channel rules, exit/error
  behavior, plan/report/inventory decoding, and TUI dispatch.
- Move binary command tests to the application owner without weakening their
  assertions.
- Update `code_map.md` to the exact final source tree and responsibilities.
- Audit and update backend/frontend Trellis directory, quality, state, and
  component guidance affected by the final tree. Do not rewrite historical
  `design.md` as current truth.
- Audit README and command documentation for stale module/interface claims;
  change only directly affected text.
- Run final structural searches and the canonical `just ci` gate.

## Acceptance Criteria

- [ ] `src/main.rs` contains only the binary entrypoint delegation and no
      command-specific business/orchestration logic.
- [ ] The generated public crate documentation exposes `devsweep::run()` and no
      accidental internal module tree or compatibility shim.
- [ ] Clap command/flag/default/help tests and saved-plan/report/inventory input
      rejection tests pass from the application module.
- [ ] CLI JSON output remains clean on stdout and tracing/diagnostics remain on
      stderr.
- [ ] No obsolete flat module import or temporary compatibility re-export
      remains in production code.
- [ ] `code_map.md` and relevant backend/frontend specs describe the actual
      final tree, interfaces, dependency direction, and test locations.
- [ ] Historical `design.md` remains identified as historical and is not used
      to reintroduce permanent delete or Docker behavior.
- [ ] All parent safety and compatibility acceptance criteria are satisfied.
- [ ] `just ci` passes on the integrated source tree.

## Out Of Scope

- Compatibility re-exports, a supported Rust library SDK, CLI/TUI feature
  changes, dependency upgrades, release publishing, or rewriting historical
  product design.
