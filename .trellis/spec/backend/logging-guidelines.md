# Logging Guidelines

> How logging is done in this project.

---

## Overview

Logging uses `tracing` and `tracing-subscriber`. Initialization is centralized
in `src/config.rs` and writes to `stderr` so JSON plans can remain clean on
`stdout`.

Current code has minimal logging. Prefer explicit CLI output for user-visible
command summaries and reserve tracing for diagnostics that should not pollute
machine-readable output.

---

## Initialization

`src/config.rs` owns process-wide tracing setup:

```rust
pub fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .try_init();
}
```

Call initialization once from `main` before command dispatch.

---

## Log Levels

- `trace` / `debug`: internal scan decisions, skipped entries, future worker
  progress. Do not enable by default.
- `info`: high-level lifecycle events that are useful in non-JSON mode.
- `warn`: recoverable failures such as an inaccessible nested directory.
- `error`: terminal failures that cause a command to return an error.

---

## What To Log

- Future long-running scan or execution job boundaries.
- Recoverable filesystem failures when they explain missing targets.
- Audit-log write failures from `src/execution/audit.rs`. Started-record
  durability failures block dispatch; terminal-record failures surface an
  unknown execution result.

---

## What Not To Log

- Do not write tracing output to `stdout`.
- Do not log secrets, environment variable dumps, credential files, or full
  command output from package managers without filtering.
- Do not log every visited path at `info`; cleanup scans can traverse large and
  private directory trees.
- Do not duplicate audit events through tracing. Append-only action records and
  replay belong exclusively to `src/execution/audit.rs`.

---

## Common Mistakes

- Do not add `println!` diagnostics to code paths that also support `--json`.
- Do not call `tracing_subscriber::fmt().init()` directly from modules. Use
  `config::init_tracing()` so repeated initialization stays harmless.
