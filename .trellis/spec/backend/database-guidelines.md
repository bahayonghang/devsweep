# Database Guidelines

> Database and persistence conventions for this project.

---

## Overview

`devsweep` uses versioned files rather than a database or ORM. Persisted
contracts include cleanup plans, execution audit records, shared presentation
language, and desktop preferences. The desktop-only settings contract lives in
[Desktop Preferences](./desktop-preferences.md); keep its file separate from
the exact shared language V1 document.

Treat this file as a guardrail: do not introduce a database abstraction for
scanner, CLI, or TUI work unless a task explicitly adds persistence.

---

## Current Data Contracts

The persisted cleanup plan contract is owned by `crates/devsweep-core/src/model/plan.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UntrustedPlan {
    pub version: u32,
    pub targets: Vec<UntrustedTarget>,
}
```

JSON output is a command/API boundary, not a database. The current schema is
v2: targets carry observed facts and a typed `CleanupIntent`, never
`program`, `args`, `cwd`, or a serialized executable action. Decode
`UntrustedPlan` at the file/CLI boundary, validate it into the opaque
`ValidatedPlan`, and pass only that value to the executor. Persisted-shape,
canonical-digest, and compatibility changes need round-trip and fail-closed
validation tests.

The audit JSONL contract and unconfirmed-start replay are owned by
`crates/devsweep-core/src/execution/audit.rs`. Each line is one
append-only action record and must include at least:

- `timestamp_epoch_ms`
- `target_id`
- `action`
- `command` when the action is command-backed
- `estimated_bytes`
- `status`
- `duration_ms`
- `error` for skipped or failed actions
- `partial`

Audit writes are part of execution, so the audit file must be opened before any
target action runs. If the audit file cannot be opened, execution must fail
before command or trash side effects.

Software removal uses a separate fixed
`%LOCALAPPDATA%\DevSweep\audit\v1\software.jsonl` contract. Each append is
flushed and synced, and an exclusive `software.lock` sidecar serializes writers
across processes. Records contain the exact tagged current-user MSIX identity,
inventory fingerprint, preview digest, transition, stable status/error codes,
reboot evidence, and installed-state requery result. They never contain vendor
commands, argv, environment, localized text, raw deployment output, or an
execution `partial`. Recovery may reconstruct only identity-only requery work;
it cannot reconstruct or redispatch a removal strategy from the journal.

---

## Migrations

There are no migrations. If future tasks add persistent config, audit storage,
or a database, they must also add:

- a versioned schema or file format owner
- migration or compatibility tests
- rollback behavior for failed writes
- explicit paths for where user data is stored

---

## Naming Conventions

- JSON fields use Serde defaults or explicit `snake_case` settings already
  present on enums under `crates/devsweep-core/src/model/`.
- Versioned persisted formats must include a top-level version field. The
  current cleanup plan uses `CLEANUP_PLAN_VERSION = 2`; v1 plans are rejected
  with rescan guidance rather than migrated into executable data.

---

## Common Mistakes

- Do not add a database crate as a convenience cache for scanning. Scanner
  output should be recomputable from filesystem evidence.
- Do not add long-term cleanup history, retention, or privacy behavior outside
  the audit-log owner. The current audit file is explicit command output, not a
  hidden application database.
- Do not use untyped `serde_json::Value` as an internal database substitute.
  Decode into domain structs at the boundary.
