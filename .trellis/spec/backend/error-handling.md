# Error Handling

> How errors are handled in this project.

---

## Overview

The current crate uses `anyhow::Result` for application-level command flow and
scanner filesystem failures. There are no custom error enums yet. Add one only
when callers need to branch on error categories instead of showing or
propagating context.

---

## Error Types

- Binary entrypoint: `fn main() -> anyhow::Result<()>`
- External crate entrypoint: `devsweep::run() -> anyhow::Result<()>`
- Crate-private TUI entrypoint: `tui::run() -> anyhow::Result<()>`
- Application command handlers:
  `application::commands::run_<command>(...) -> anyhow::Result<()>`
- Scan report pipeline:
  `scan::Sweeper::default().full_scan_report(...) -> anyhow::Result<ScanReport>`

Use `anyhow::Context` when a filesystem operation needs path-specific context:

```rust
let root = root
    .canonicalize()
    .with_context(|| format!("failed to access scan root {}", root.display()))?;
```

---

## Error Handling Patterns

- Use `?` to propagate command-level failures through `application::run` and
  `devsweep::run` to `main`.
- Use `anyhow::bail!` for explicit user-facing command failures. For example,
  `clean --execute` without `--plan PATH` fails before the executor runs.
- Saved plan/report decoding in `crates/devsweep-cli/src/application/commands.rs` adds file and
  JSON context, rejects inventory documents, and preserves version-specific
  rescan guidance before validation.
- Project scan root access failures are hard errors. Inaccessible nested entries are
  recorded as discovery diagnostics and skipped so one unreadable child does not
  abort the whole scan; the outcome is marked partial while sibling candidates
  remain.
- Bounded walks under `crates/devsweep-core/src/filesystem/sizing.rs` never convert I/O failure into
  a trusted `0 B`. They return a `SizeEstimate` that is either a complete total,
  a partial lower bound, or unknown. Incomplete and unknown estimates are not
  selected by default.
- Execution authorization and started-audit failures are fail-closed. A
  terminal audit failure after dispatch reports the result as unknown because
  the side effect may already have run.
- Tests may use `expect(...)` with a specific reason.

Example from `crates/devsweep-cli/src/application/commands.rs`:

```rust
if command.execute && command.plan.is_none() {
    anyhow::bail!("cleanup execution requires --plan PATH");
}
```

---

## API Error Responses

There is no HTTP API. CLI commands should return non-zero through propagated
errors and keep machine-readable JSON output clean. Do not mix diagnostics into
`stdout` when `--json` is active.

---

## Common Mistakes

- Do not `unwrap()` in production code for filesystem, terminal, JSON, or user
  input handling.
- Do not silently turn a missing scan root into an empty plan. That would hide a
  user mistake.
- Do not log an error and then also return it unless the caller cannot otherwise
  see the failure.
