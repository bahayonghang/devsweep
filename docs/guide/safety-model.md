# Safety Model

DevSweep separates discovery, planning, validation, and side effects. That
separation is the product boundary, not a convenience feature.

## Scan first

`scan` identifies eligible project artifacts and supported global providers. It
records evidence, a risk level, a cleanup intent, size information, and health
diagnostics in a versioned report. A scan does not run cleanup actions.

## Treat saved documents as untrusted input

`clean --plan PATH` parses a saved plan or scan report, validates its version
and shape, and rebuilds executable actions from DevSweep's trusted rule
registry. A serialized plan cannot choose an arbitrary program or shell
command. See [plans and reports](/reference/plan-and-report) for the boundary.

## Dry-run is the default

Without `--execute`, `clean` reports the selected targets and performs no
cleanup. Use that run to confirm that the saved plan still represents the
intended scope.

## Execution remains explicit

Execution requires both a plan path and `--execute`. Project artifacts normally
use trash-backed cleanup; Rust `target/` cleanup uses `cargo clean` with the
applicable manifest. Command-backed global cleanup keeps the program and argv
separate rather than constructing a shell string.

When execution is requested, DevSweep rechecks the selected target and its
authorization footprint before side effects. It can append JSONL audit records
to the supplied `--audit-log` path, or to `devsweep-audit.jsonl` by default.

## Explicit non-goals

- Permanent deletion is disabled in this build and is not exposed as a CLI
  option.
- Docker cleanup is deferred and is not scanned or executed.
- Cargo home is inspect-only. Credentials, installed binaries, registry
  internals, and Git cache internals are never made into cleanup actions.
- Symlinked directories and Windows reparse points are guarded rather than
  followed as cleanup targets.

## Protect paths you own

Use [`protect`](/guide/protection) to add an existing path to the persistent
protection list. Protection-list load errors fail closed instead of silently
falling back to an empty list.
