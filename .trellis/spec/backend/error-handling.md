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
- TUI entrypoint: `pub fn run() -> anyhow::Result<()>`
- Scanner entrypoint:
  `ProjectScanner::new().scan_roots(&[PathBuf]) -> anyhow::Result<CleanupPlan>`

Use `anyhow::Context` when a filesystem operation needs path-specific context:

```rust
let root = root
    .canonicalize()
    .with_context(|| format!("failed to access scan root {}", root.display()))?;
```

---

## Error Handling Patterns

- Use `?` to propagate command-level failures to `main`.
- Use `anyhow::bail!` for explicit user-facing command failures. Current
  example: `clean --execute` fails because execution is not implemented yet.
- Scanner root access failures are hard errors. Inaccessible nested entries are
  skipped so one unreadable child does not abort the whole scan.
- Tests may use `expect(...)` with a specific reason.

Example from `src/main.rs`:

```rust
if command.execute {
    anyhow::bail!("cleanup execution is not implemented in the foundation build");
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
