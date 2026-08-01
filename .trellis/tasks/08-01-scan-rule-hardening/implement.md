# Implementation Plan

## 1. Establish Contracts And Fixtures

- [ ] Read the backend and TUI pre-development checklists before source edits.
- [ ] Add typed scan-health, sizing-warning, report, inventory, and
  pre-dispatch audit outcome models at the shared model boundary.
- [ ] Add JSON compatibility tests for legacy plan-only input, report export,
  clean decoding, and rejection of untrusted executable fields.

## 2. Harden Path Safety

- [ ] Implement an injectable path-level reparse probe in the filesystem
  boundary, including Windows tag inspection and conservative errors.
- [ ] Route scanner descent, bounded size walks, marker/target validation, and
  ancestor revalidation through it.
- [ ] Add Windows-focused tests using a probe seam for Cloud Files-style tags,
  including root, child, marker, target, and ancestor cases; retain symlink
  tests on all platforms.

## 3. Preserve Scan Health

- [ ] Carry `ScanOutcome` diagnostics and completeness through `Sweeper`, CLI
  report serialization, and TUI worker events/state.
- [ ] Classify Cargo output truncation before parsing and include bounded
  process-output metadata in diagnostics.
- [ ] Make pre-dispatch denials write a durable terminal audit record and show
  target-specific failures in the CLI execution summary.

## 4. Bound Cargo Metadata Work

- [ ] Add a scan-lifetime workspace/member cache with an explicit maximum size
  and deterministic diagnostic reuse for identical manifests.
- [ ] Use cache membership rather than ancestry alone; probe nested manifests
  not proven to be workspace members.
- [ ] Keep executor live Cargo revalidation uncached and test that it still
  occurs before command dispatch.

## 5. Make Capacity Review Accurate

- [ ] Thread complete/partial/unknown size aggregates and warnings into report
  and TUI state.
- [ ] Add an explicit selected-target higher-budget rescan path with the same
  traversal guards and cancellation support.
- [ ] Add `__pycache__` project grouping/collapse as a presentation projection;
  cover selection and render behavior with state and TestBackend tests.

## 6. Add Read-Only Inventory

- [ ] Implement a non-executable inventory report and its CLI/TUI projections.
- [ ] Implement orphan-pnpm inspection with configuration/reference evidence.
- [ ] Assert that inventory and orphan findings cannot be selected, serialized
  as cleanup intents, or passed to the executor.

## 7. Validate And Review

- [ ] Run focused module tests after each contract boundary.
- [ ] Run CLI help and representative JSON/report commands manually; verify
  stdout remains valid JSON and stderr carries only diagnostics.
- [ ] Run TUI TestBackend smoke and state tests for health/totals/grouping.
- [ ] Run `just ci`.
- [ ] Inspect `git diff --check` and `git status --short`; preserve unrelated
  Trellis runtime modifications.

## Risk Controls

- Do not add direct filesystem deletion, trash execution, or subprocess calls
  to scanner, model, report, or render code.
- Do not downgrade an uncertain reparse probe to an allowed traversal.
- Do not make a cache entry authoritative for execution-time Cargo scope.
- Do not count a partial lower bound as reclaimable verified space.
- Do not treat an inventory observation as cleanup approval.

## Rollback Points

- After the path probe: revert only the new probe call sites if fixture
  coverage demonstrates an interoperability issue; retain fail-closed behavior.
- After report serialization: preserve plan-only decoding so scan output can be
  temporarily exported as its embedded plan.
- After inventory: the command is isolated from the executor and can be
  removed without changing cleanup plan semantics.
