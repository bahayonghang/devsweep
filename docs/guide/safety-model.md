# Safety Model

DevSweep separates discovery, planning, validation, and side effects. That
separation is the product boundary, not a convenience feature.

## Scan first

`clean scan` identifies eligible project artifacts and supported global
providers. It records evidence, a risk level, a cleanup intent, size
information, and health diagnostics in a versioned observation. A scan does not
run cleanup actions.

Save the observation with `--format json --output FILE`. That JSON is not a
runnable plan.

## Treat saved documents as untrusted input

`clean plan --observation FILE --select TARGET_ID --output FILE` reads the
observation, keeps only the exact selected identities, and writes a **new**
versioned plan. `clean preview --plan FILE` and `clean execute --plan FILE`
decode that saved plan as untrusted input, validate its version and shape, and
rebuild executable actions from DevSweep's trusted rule registry. A serialized
plan cannot choose an arbitrary program or shell command. See
[plans and reports](/reference/plan-and-report) for the boundary.

## Preview is the dry-run

`clean preview` reports the selected targets, calculates a live `sha256:`
digest, and performs no cleanup. Use that run to confirm that the saved plan
still represents the intended scope. There is no `--execute` dry-run mode.

## Execution remains explicit

Execution requires a saved plan, the matching live preview digest
(`sha256:` plus 64 lowercase hexadecimal characters), and `--confirm`. Project
artifacts normally use trash-backed cleanup; Rust `target/` cleanup uses
`cargo clean` with the applicable manifest. Command-backed global cleanup keeps
the program and argv separate rather than constructing a shell string.

When execution is requested, DevSweep rechecks the selected target and its
authorization footprint before side effects. It appends JSONL audit records
only to the fixed Clean V1 store
`%LOCALAPPDATA%\DevSweep\audit\v1\clean.jsonl`. There is no `--audit-log` flag.
Legacy `%APPDATA%\devsweep\audit.jsonl` is not opened, imported, converted, or
searched.

## Explicit non-goals

- Permanent deletion is disabled in this build and is not exposed as a CLI
  option.
- Docker cleanup is deferred and is not scanned or executed.
- Cargo home is inspect-only. Credentials, installed binaries, registry
  internals, and Git cache internals are never made into cleanup actions.
- Symlinked directories and Windows reparse points are guarded rather than
  followed as cleanup targets.

## Protect paths you own

Use [`clean protect`](/guide/protection) to add an existing path to the
persistent protection list. Mutations require `--path` and `--confirm`.
Protection-list load errors fail closed instead of silently falling back to an
empty list.
