# Database Guidelines

> Database and persistence conventions for this project.

---

## Overview

`devsweep` currently has no database, ORM, migrations, or durable application
state. The only persisted data contract in the codebase is the serializable
cleanup plan emitted as JSON from `src/model.rs`.

Treat this file as a guardrail: do not introduce a database abstraction for
scanner, CLI, or TUI work unless a task explicitly adds persistence.

---

## Current Data Contracts

The cleanup plan contract is owned by `src/model.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupPlan {
    pub version: u32,
    pub targets: Vec<CleanTarget>,
}
```

JSON output is a command/API boundary, not a database. It must remain
round-trip tested when fields are added or renamed.

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
  present on enums in `src/model.rs`.
- Versioned persisted formats must include a top-level version field. The
  current cleanup plan uses `CLEANUP_PLAN_VERSION`.

---

## Common Mistakes

- Do not add a database crate as a convenience cache for scanning. Scanner
  output should be recomputable from filesystem evidence.
- Do not store user paths or cleanup history before the audit-log task defines
  retention and privacy behavior.
- Do not use untyped `serde_json::Value` as an internal database substitute.
  Decode into domain structs at the boundary.
