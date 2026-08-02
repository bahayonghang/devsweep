# Untangle core contracts and rule ownership

## Goal

Establish an acyclic core for the larger architecture refactor by separating
serialized/domain contracts from runtime observations, giving Cargo metadata a
neutral owner, and making the rules module the sole owner of rule metadata and
trusted action reconstruction.

## Parent And Ordering

- Parent: `08-02-reorganize-rust-source-architecture`.
- This is the first implementation child and has no child-task dependency.
- Later backend, TUI, and entrypoint children depend on the interfaces produced
  here; this child must pass `just ci` and be archived before child 2 starts.

## Confirmed Problems

- `model` imports process-runner types only for diagnostic conversion
  (`src/model.rs:5`, `src/model.rs:170`, `src/model.rs:199`).
- `rules` imports scanner/provider implementations to build its catalogue
  (`src/rules.rs:22`, `src/rules.rs:346`), while those implementations import
  `rules` (`src/scanner.rs:20`, `src/providers.rs:20`).
- `registry` imports rule-document constants from scanner and providers
  (`src/registry.rs:8`), so rule identity and facts have no single owner.
- Cargo metadata probing is implemented in execution safety
  (`src/safety.rs:1065`) but is also a scanner dependency
  (`src/scanner.rs:21`).

## Requirements

- Split the model implementation into cohesive plan and scan/report ownership
  while preserving every serialized field, derive, version, and internal value
  semantic.
- Remove all process-runner imports from the model module. Convert
  `ProcessResult` observations into typed scan diagnostics at the neutral Cargo
  metadata/caller seam.
- Move Cargo metadata scope, failure, probe, adapter, classification, and parser
  code out of execution safety into a neutral private module used by both scan
  and live execution authorization.
- Move all scanner/provider rule documentation constants into the rules module.
  Scanner and provider code may look up rule definitions; rules must not import
  their implementations.
- Move trusted intent-to-action reconstruction under the rules/plan trust
  implementation so it is not an independent public module.
- Keep `ValidatedPlan` opaque and preserve validation, canonical fingerprint,
  digest, duplicate detection, path normalization, and v1 rejection behavior.
- Use private or `pub(crate)` visibility. Do not create final compatibility
  re-exports for the previous Rust module paths.
- Preserve all safety contracts and user-visible behavior; this child is
  structural only.

## Acceptance Criteria

- [ ] The model module imports no process, filesystem, scan, execution, or TUI
      implementation module and retains JSON compatibility tests.
- [ ] Rules own every built-in rule ID, facts, catalogue record, and trusted
      action template; rules no longer import scanner or provider modules.
- [ ] Scanner and execution safety both use the same neutral Cargo metadata
      implementation without importing one another for that behavior.
- [ ] Plan validation still rejects executable JSON fields, unknown rules,
      mismatched facts, duplicate fingerprints, path escapes, and plan v1.
- [ ] Canonical digest results and provider command reconstruction remain
      behaviorally unchanged.
- [ ] Scanner/provider catalogue completeness and uniqueness tests pass from
      the new rule owner.
- [ ] Affected backend directory/quality specs name the new core owners and no
      longer describe the retired flat rule/registry/validation layout.
- [ ] No scanner/model/rule code performs cleanup side effects.
- [ ] `just ci` passes.

## Out Of Scope

- Filesystem/process runner directory decomposition, broad scanner/provider
  file splitting, executor/audit splitting, TUI splitting, and final
  `lib.rs`/`main.rs` narrowing.
- Schema changes, new rules/providers, compatibility shims, or behavior fixes
  unrelated to preserving the refactor.
