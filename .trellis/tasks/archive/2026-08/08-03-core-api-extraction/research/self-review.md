# Core API Extraction Self-Review

Date: 2026-08-03

## Scope Review

- The change is limited to the reviewed workspace split, public service/API
  closure, build/CI adaptation, tests, evidence, and directly affected specs.
- No Serde derivation, execution-report shape, digest confirmation contract, or
  cleanup behavior from the next task was introduced.
- Source moves remain behavior-preserving; content changes are visibility,
  cross-crate imports, rustdoc, and test/build plumbing required by the split.

## Safety Review

- Dry-run remains the default and execution still consumes only a validated
  plan through the existing safety authorization funnel.
- Permanent deletion remains disabled; scanner/model layers still create plans
  only and command program/argv remain separate.
- `devsweep-core` contains no terminal output or process-exit calls and has no
  Clap, Ratatui, or Crossterm dependency.
- The deterministic fixture generator restricts recursive replacement to a
  dedicated leaf and rejects reparse-point path components.

## Findings Resolved

- The first JSON comparison exposed Cargo workspace inheritance in the nested
  Rust fixture. Giving that fixture an empty `[workspace]` restored a valid,
  field-for-field comparison rather than filtering the drift.
- A clean Linux target exposed that cached `process_fixture` output masked the
  new cross-crate test dependency. The helper now builds the CLI-owned fixture
  into the current test target root, and timeout assertions start after setup.
- The independent check narrowed test/injection-only APIs, made verification
  commands reproducible, fixed release recipes, and synchronized stale spec
  paths and service ownership.

## Acceptance Verdict

- `just ci`, MSRV 1.88, clean Windows tests, Ubuntu WSL tests/Clippy, macOS
  all-target compilation, rustdoc, dependency boundaries, release smoke, CLI
  equivalence, and full JSON equivalence pass. Exact commands and results are
  recorded in `research/verification.md`.
- Hosted GitHub Actions were not run because this goal forbids pushing. The PRD
  acceptance uses its documented local-equivalent option; no claim is made
  about completed hosted jobs.
- No blocking review finding remains for this child task.
