# Implementation Plan: Plan Trust Boundary

## Preconditions and activation gates

- Approve the direct production dependency `sha2` for SHA-256 canonical plan
  digests; do not substitute a non-cryptographic hasher.
- Keep all v2 serde changes in this child only.
- Freeze the registry interface in this child before the rule-registry child
  starts extending detector/resolver behavior.

## Ordered work

1. Write fail-red tests for v1/unknown version rejection, arbitrary commands,
   extra args, relative paths, unknown rule/action ids, mismatched target/action
   paths, inspect-only selection, and duplicate equivalent fingerprints.
2. Add the typed v2 untrusted DTOs and remove executable fields from their
   serde surface. Update scan fixtures to emit only declarative intent/facts.
3. Define the minimal `RuleRegistry` and `ActionSpec` contract, then implement
   current project/provider/global-rule action reconstruction from one source.
4. Implement `validate_plan` and opaque `ValidatedPlan`/`ValidatedAction`.
   Verify all invariants before returning any executable action.
5. Implement lexical platform-aware path normalization, canonical ordered
   serialization, SHA-256 digest, and fail-closed duplicate fingerprints.
   Add order-insensitive JSON/digest and change-sensitive digest tests.
6. Change executor, CLI, and TUI call sites to consume only `ValidatedPlan`.
   Add its once ledger and a test-only malformed validated-plan constructor to
   prove that a second matching action is rejected before runner invocation.
7. Remove `--allow-permanent-delete` from clap, help, README, and regression
   tests. Verify dry-run and execute share validation and execute cannot expand
   selection.
8. Run focused model/registry/executor/CLI/TUI tests, inspect serialized v2
   output manually, then run `just ci`.

## Verification matrix

| Risk | Required test/evidence |
| --- | --- |
| arbitrary execution | plans containing `cmd.exe`, PowerShell, `/bin/sh`, or extra argv cause zero runner calls |
| schema compatibility | v1/unknown versions fail with rescan wording before any side effect |
| object mismatch | target path/action path and scope escapes fail before runner construction |
| registry drift | unknown or incompatible rule/action/risk/reversibility values fail closed |
| identity ambiguity | JSON field/target ordering gives same digest; any logical action change changes digest |
| duplicate execution | Windows case/separator/trailing-slash variants and duplicate ids are rejected; direct executor test proves once ledger |
| CLI regression | `clean --help` has no permanent-delete flag; scan-save-dry-run-execute v2 path remains valid |

Run focused tests on the host and require Windows CI evidence for path case and
separator variants. If the Windows suite cannot run, this task cannot be
activated or declared complete; a Unix-only green run is insufficient for the
fingerprint contract. Keep a macOS/Linux path-normalization row in CI as well
to guard separator and absolute-path behavior.

## Review and rollback points

- Review every public serde type to ensure program/argv/cwd cannot reappear.
- Review registry ownership before accepting any provider/rule migration.
- Do not let scanner dedupe substitute for executor once protection.
- Before task start, ensure both manifests are curated and `sha2` approval is
  recorded. If validation breaks existing saved plans unexpectedly, retain the
  explicit v1 rescan error rather than silently accepting legacy commands.
