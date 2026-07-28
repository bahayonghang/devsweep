# Design: Plan Trust Boundary

## Boundary and compatibility

Plan input is untrusted data. Version 2 serializes declarative target intent
and observed facts only; it never serializes an executable program, argv, cwd,
or a free-form cleanup path that is treated as authority. `version == 1` and
all unknown versions fail before the executor is constructed with the exact
rescan guidance specified by D1. There is no v1 migration.

The implementation introduces an untrusted DTO layer and an opaque
`ValidatedPlan`. `Executor::run_plan` accepts only `&ValidatedPlan`; the CLI,
TUI, and tests must call the validation entry point first. Test-only builders
may create malformed validated values solely to exercise the executor's
defensive once ledger.

## Declarative v2 shape

Each untrusted target contains a stable target id, rule id, declared scope,
selection/default facts, evidence, an absolute observed target path when the
rule is path-backed, a scan-time footprint fact when one can be captured, and
one typed intent:

- `TrashProjectArtifact { rule_id, artifact_kind }`
- `RunBuiltInAction { provider_id, action_id }`
- `InspectOnly { rule_id }`

The DTO cannot represent permanent delete or arbitrary command execution.
`RuleRegistry` is the sole factory for actions. Its minimal contract resolves
`(intent, observed facts)` to an `ActionSpec` containing action identity,
expected risk/reversibility, allowed scope roots, and a reconstructed action.
It rejects unknown rule/action ids, wrong intent for a rule, scope mismatch,
and facts inconsistent with the registry. It covers the current project rules,
provider actions, and global inspect-only rules without attempting the richer
detector/resolver migration owned by `07-28-rule-registry-providers`.

Validation verifies exact v2 dispatch, non-empty unique ids, absolute paths,
scope containment, target/action path equality where an action is path-backed,
`NoopInspectOnly` non-selection, and registry-derived risk/reversible/action
facts. Validation produces a `ValidatedAction` containing only reconstructed
trusted values. `07-28-central-safety-policy` owns the meaning and capture
rules for the scan-time footprint fact and performs the execution-time
filesystem revalidation that closes TOCTOU gaps; this child only carries the
versioned DTO field and rejects malformed facts.

## Canonical identity

The canonical plan encoding is a dedicated ordered data structure, not a raw
re-serialization of untrusted JSON. It uses fixed struct field order, sorted
target entries by `ActionFingerprint`, normalized textual paths, and a
domain-separation prefix that includes the schema version. The digest is
SHA-256 of those bytes, rendered as lowercase hexadecimal. This requires the
well-maintained direct `sha2` crate dependency; dependency approval is a
pre-activation gate and no in-tree cryptographic implementation is permitted.

`ActionFingerprint` is the stable identity of normalized action footprint plus
registry action identity. Path normalization is lexical and platform-specific:
absolute paths only, normalized separators and trailing separators, and on
Windows case-insensitive comparison plus verbatim-prefix normalization. It
does not claim to prove the live filesystem object; the safety-policy task
will do that immediately before side effect. Equivalent fingerprints are a
validation error, not a merge. A hand-edited or duplicated external plan must
fail closed rather than silently shrink an operator's requested selection.

The executor also keeps an in-memory once ledger keyed by the validated
fingerprint. Repetition is rejected and audit-visible before a second action
can run. This defense remains even though a normal `ValidatedPlan` cannot
contain duplicates.

## Consumer contract

The scanner emits v2 declarative DTOs; it does not execute or validate actions.
CLI dry-run and execute use the same validated plan, and execution may only
reduce the explicit selected target set. The TUI uses the same validator before
dispatching its frozen manifest and later displays the digest prefix supplied
by this module. Durable audit receives the canonical plan digest from the
validated plan. `--allow-permanent-delete` is removed from CLI parsing, help,
documentation, and tests.

## Failure and rollback behavior

Every malformed or inconsistent plan returns a typed validation error before
any runner or trash call. Existing v1 saved plans are deliberately rejected;
the rollback path is to restore the prior release, not to add a lossy in-place
migration. Once v2 is shipped, changes to the canonical bytes, fingerprint,
or registry identity require a new versioned task and compatibility review.
