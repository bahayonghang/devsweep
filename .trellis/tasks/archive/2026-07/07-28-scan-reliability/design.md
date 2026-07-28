# Design: Partial Scan and Size Reliability

## Outcome model and ownership

Discovery returns `Result<ScanOutcome>` rather than treating every filesystem
failure identically. A scan-root open/metadata failure remains an `anyhow`
error with root-path context. A failure below a successfully opened root becomes
a typed diagnostic and scanning continues with siblings. `ScanOutcome` carries
the discovered candidates, diagnostics, and an aggregate completeness state.

The task owns the meaning of diagnostics and completeness. It does not own the
serialized cleanup-plan schema: `07-28-plan-validation` owns the v2 wire
format. Before implementation, the two tasks must record one interface shape
for partial diagnostics and estimate data. Until that shape lands, this task
may implement internal outcome/estimate propagation but must not independently
add serde fields to `CleanupPlan`.

## Diagnostic semantics

Each diagnostic has a stage (`discovery` or `size`), path, operation category,
and sanitized human-readable detail. It must not leak raw platform error data
into JSON stdout when the command promises machine-readable output. Root
failure is never downgraded. Nested `symlink_metadata`, `read_dir`, and child
recursion failures add a warning and continue, preserving all previously found
and later sibling candidates.

`ScanCompleteness` is complete only when every requested root completes without
child diagnostics. Any warning marks the affected root and final aggregate as
partial. Global-provider diagnostics use the same representation where possible
without turning a missing optional executable into a hard scan failure.

## Size estimates and selection

`SizeEstimate` distinguishes a known empty directory from a failed traversal:

- `logical_bytes: Some(0), complete: true` means a verified empty tree.
- `logical_bytes: Some(n), complete: false` means at least `n` bytes were
  observed and one or more branches could not be read.
- `logical_bytes: None, complete: false` means no trustworthy total exists.

Warnings accumulate instead of converting I/O failure to zero. A target with an
incomplete or unknown estimate is never selected by default, regardless of
freshness. Rendering uses `>= n (incomplete)` for a partial lower bound and
`unknown` when no lower bound exists. It must not format either state as `0 B`.
Ranking keeps its existing order in this task; `scan-walker-budgets` later owns
budget and ranking changes.

## Pipeline and presentation

Scanner/provider outcomes flow through `sweep` into the TUI worker and CLI.
The TUI displays partial status and diagnostics alongside the cumulative scan
snapshot without resetting explicit selections. Text `scan --global` reports
provider/global discovery only; it does not print a project-root count when no
project roots were scanned. JSON output serializes the v2 partial status only
through the agreed plan-validation contract and keeps diagnostics out of
unstructured stderr/stdout cross-contamination.

## Cancellation and rollback

Traversal points receive a no-op cancellation-check abstraction only; token
ownership and real cancellation semantics stay in `07-28-true-cancellation`.
No budget, pruning, or new worker lifetime policy is introduced here. Rollback
returns to root-only errors and the old size representation, but must retain
the fail-red permission fixtures until a corrected partial model is restored.

## Platform evidence

Unix coverage uses an actual permission-denied child fixture and verifies it is
effective before asserting partial output; test runs as a privileged user that
cannot produce denial are not evidence. Windows coverage uses a deny ACL child
fixture and proves `read_dir` or metadata access really fails. If either
environment cannot create an effective denial fixture, record that as a no-go
for the corresponding platform rather than silently skipping the behavior.
