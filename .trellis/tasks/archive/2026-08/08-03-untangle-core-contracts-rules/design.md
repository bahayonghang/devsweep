# Core Contracts And Rules Design

## Target Ownership

```text
model/
  mod.rs       crate-private re-exports
  plan.rs      cleanup plan/target/intent/action types and versions
  scan.rs      reports, health, diagnostics, totals, sizing warnings

rules/
  mod.rs       lookup, catalogue, formatting, trusted resolve interface
  definitions.rs  project/global/provider rules and RuleDoc values
  registry.rs  private UntrustedTarget -> ActionSpec reconstruction

plan/
  mod.rs       validate_plan, scan projection, opaque validated types
  digest.rs    private canonical manifest and SHA-256 digest

cargo_metadata.rs
  scope/failure/probe types, production adapter, parser, process mapping
```

The precise plan/model split may keep tightly coupled types together when
moving them would create circular re-exports. The invariants are ownership,
dependency direction, and a smaller interface, not a target line count.

## Dependency Direction

```text
model <- process
model <- rules <- plan
process + model <- cargo_metadata
rules + cargo_metadata + model <- scanner
plan + rules + cargo_metadata + model <- execution safety
```

`model` itself imports only standard-library and serde concerns. `rules` may
import model and low-level path normalization, but never scanner/provider
implementations. `cargo_metadata` may import process and model diagnostic value
types; neither process nor model imports it back.

## Rule Interface

All definitions currently split across `rules.rs`, `scanner.rs`, and
`providers.rs` move under `rules::definitions`. Discovery code refers to the
rule definition for ID, risk, ecosystem, kind, reversibility, and intent facts
instead of copying values.

The registry becomes private implementation behind a crate-private resolver:

```rust
pub(crate) fn resolve_action(target: &UntrustedTarget) -> Result<ActionSpec>;
```

`ActionSpec` remains visible only to plan validation. Catalogue callers receive
structured `RuleDoc` values. Existing shared row/risk formatting may remain in
`rules` until the final application/TUI architecture is complete; it must not
pull implementation modules back into rules.

## Plan Interface

The plan module keeps opaque validated types. Callers can read validated
targets, default selected IDs, and digest, but cannot construct a validated
plan outside test-only internal support.

Canonicalization moves to `plan::digest`. Moving code must not alter:

- target ordering independence;
- normalized path identity;
- logical provider footprints;
- action/evidence fields bound into the digest;
- duplicate fingerprint rejection;
- saved-plan v1 guidance and unknown-version errors.

## Cargo Metadata Interface

Cargo metadata probing is neutral shared infrastructure:

```rust
pub(crate) trait CargoMetadataProbe { ... }
pub(crate) struct SystemCargoMetadataProbe;
pub(crate) enum CargoMetadataProbeResult { ... }
pub(crate) fn query_cargo_metadata(...) -> Result<CargoMetadataScope>;
```

The production and fake probe adapters justify the internal seam. Process
status/output conversion into `ScanProcessProbe` moves here, so report DTOs do
not know the runtime's `ProcessResult` type.

Scanner retains workspace-result caching. Execution safety retains live
revalidation policy. Only probe execution/parsing/classification moves.

## Compatibility

Serialized DTO structure is unchanged. Rust paths may change. Intermediate
crate-private re-exports are allowed to make this child reviewable, but no
public compatibility promise is introduced and the parent final child removes
remaining accidental exports.

## Risks And Controls

- Risk: serde drift during type moves. Control: retain derives/attributes
  verbatim and run existing round-trip/rejection tests before broader edits.
- Risk: rule fact drift. Control: move constants first, then consume the same
  values from scanner/provider/registry and run catalogue/validation tests.
- Risk: digest drift. Control: move canonicalization mechanically and compare
  all existing digest tests before refactoring helper names.
- Risk: Cargo metadata diagnostic drift. Control: retain typed status, capped
  output, truncation flags, failure detail, and truncated-output handling.

Rollback is the single child work commit after a passing gate.

