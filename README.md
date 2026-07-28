# devsweep

`devsweep` is a safety-first developer cleanup planner and executor. It scans project build artifacts and global developer caches, emits an auditable cleanup plan, and keeps cleanup actions explicit.

## Safety Model

- `devsweep clean` is a dry-run by default. Add `--execute` only when you want to run selected actions from a saved plan.
- Saved plans are declarative v2 documents. They carry observed facts and a typed
  cleanup intent; devsweep validates them and reconstructs trusted actions from
  its built-in rule registry before either dry-run or execution.
- Every cleanup target carries evidence and a risk level. Evidence explains why
  the target was considered cleanable.
- Global package-manager cleanup is command-backed. npm, pip, pnpm, and Yarn
  actions use registry-owned argv templates, not shell strings from a plan file.
- Project cleanup is trash-backed by default for directories such as `node_modules`, `.venv`, and tool caches.
- Rust project `target` cleanup prefers `cargo clean --manifest-path <Cargo.toml>`.
- Cargo home is inspect-only in the MVP. `devsweep` does not delete `%USERPROFILE%\.cargo`, `~/.cargo`, credentials, installed binaries, registry internals, or git cache internals.
- Permanent delete is disabled in this build and is not exposed as a CLI flag.
- Docker cleanup is deferred and is not part of the MVP.

The scanner does not follow symlinked cleanup directories. On Windows, reparse-point directories are skipped by the same guard; junction-specific behavior depends on the OS exposing the reparse-point metadata.

## Usage

Open the TUI:

```powershell
cargo run -- tui
```

Scan the current directory and global providers as JSON:

```powershell
cargo run -- scan --json
```

Save a plan, then dry-run cleanup:

```powershell
cargo run -- scan . --json > plan.json
cargo run -- clean --plan plan.json
```

Execute selected targets from a plan:

```powershell
cargo run -- clean --plan plan.json --execute --audit-log devsweep-audit.jsonl
```

## Validation

The canonical local gate is:

```powershell
just ci
```

Equivalent commands:

```powershell
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

## Release Archive

Build a Windows single-binary archive:

```powershell
just release-archive
```

The archive is written to `dist/devsweep-x86_64-pc-windows-msvc.zip`.
