# Core Contract Extension Verification

Date: 2026-08-03
Host: Windows, stable Rust toolchain

## Contract Results

- `ScanOptions`, `ScanPhase`, `ScanProgress`, `ExecutionReport`, and
  `ActionFailure` serialize and deserialize through the core types directly.
  Struct fields remain snake_case, enums use snake_case variants, data-bearing
  enums use a `type` tag, and boundary types reject unknown fields.
- A dry run returns a typed `ConfirmationDigest` covering the existing
  canonical validated manifest plus the sorted, deduplicated selected target
  ids. Execute requests with a supplied stale digest fail before audit or side
  effects with `ExecutionError::StaleConfirmation`.
- Selection normalization rejects unknown ids atomically, records one note for
  each duplicated id, rejects inspect-only targets before side effects, and
  exposes command irreversibility in `TargetOutcome.action`.
- Every deduplicated selected target has one outcome, including dry-run and
  targets left unprocessed after cancellation or fail-closed audit termination.
  Reports maintain `selected == attempted == outcomes.len()` and
  `succeeded + failed + skipped == attempted`; retained failure summaries map
  one-to-one to failed outcomes.
- Failure to persist an authorization denial or safety skip records a failed
  outcome, halts dispatch, and records every remaining selected target as
  skipped. No later cleanup side effect runs after the journal failure.
- Per-target and aggregate estimated-recoverable capacity preserves verified,
  partial-lower-bound, and unknown confidence instead of claiming bytes were
  released.

## Validation

| Gate | Result |
| --- | --- |
| `cargo test -p devsweep-core --locked --lib execution::tests` | PASS: 26 tests |
| Exact scan serde test with `--exact` | PASS: 1 test |
| `cargo test -p devsweep-core --locked --test public_api -- --exact external_crate_can_use_the_supported_core_surface` | PASS: 1 test |
| `cargo test -p devsweep-core --locked --lib scan::tests` | PASS: 13 tests |
| `cargo test --workspace --locked --all-targets` | PASS: 232 tests (70 CLI/TUI, 161 core, 1 public API) |
| `just ci` | PASS: format, lock sync, workspace check, 232 tests, Clippy with warnings denied |
| `$env:RUSTDOCFLAGS = '-D missing-docs'; cargo doc -p devsweep-core --locked --no-deps` | PASS |

Focused tests cover serde round trips, outcome/count consistency, capacity
confidence, stable and input-sensitive confirmation digests, stale execute
rejection, unknown/duplicate/inspect-only selection boundaries, irreversible
action projection, cancellation, and audit early termination.

## Deterministic Scan JSON Equivalence

The comparison repeated the archived child-1 procedure from
`08-03-core-api-extraction`: `tests/fixtures/make_scan_fixture.ps1` created the
fixed-timestamp Rust/Node/Python fixture, the baseline binary was reconstructed
from commit `4917126e822084d7174708fc5776aef6447709bb`, both JSON documents were
normalized with `jq -S`, and `git diff --no-index --exit-code` returned 0.

Evidence:

- `research/baseline.normalized.json`
- `research/after.normalized.json`
- Both files are 5,828 bytes with SHA-256
  `57063518d0a2172a002aa8483acef4b073de19f6c2008c3fe9efb8afaebe7489`.

No `scan --json` fields changed.

## Scope Boundary

Current CLI and TUI callers set `expected_digest: None` to preserve their
existing behavior; the TUI retains its pre-existing frozen-plan digest check.
The next Tauri bridge child is responsible for requiring the returned
selection-aware digest on its GUI execute command. This task adds no GUI
execution path.
