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
cargo run --bin devsweep -- tui
```

Scan the current directory and global providers as JSON:

```powershell
cargo run --bin devsweep -- scan --json
```

Save a plan, then dry-run cleanup:

```powershell
cargo run --bin devsweep -- scan . --json > plan.json
cargo run --bin devsweep -- clean --plan plan.json
```

Execute selected targets from a plan:

```powershell
cargo run --bin devsweep -- clean --plan plan.json --execute --audit-log devsweep-audit.jsonl
```

## Desktop Development

The Tauri 2 desktop MVP targets Windows and requires Node.js 22. Install its
locked frontend dependencies, then start the native development window:

```powershell
cd desktop
npm ci
npm run tauri -- dev
```

From the repository root, run the desktop frontend and Rust checks or build an
unsigned NSIS installer:

```powershell
just desktop-web-check
just desktop-test
just desktop-build
```

The desktop app uses the same core validation, confirmation digest, and
trash-backed execution path as the CLI. Cleanup remains review-first; moving a
path to the trash does not make its capacity available until the trash is
emptied.

## Documentation

The complete English and Simplified Chinese documentation site lives in
[`docs/`](docs/index.md). Install its locked dependencies with `npm ci`, then run
`just docs` to start the local site.

## Five-mode product

The accepted Windows product exposes five primary modes together: Clean,
Software, Optimize, Analyze, and Status, plus read-only History, Protection,
and Rules. Bare `devsweep` from an interactive TTY opens the TUI. The shipped
roots are `clean`, `software`, `optimize`, `analyze`, `status`, and `history`.
Old `tui`, `scan`, `inventory`, `protect`, and `rules` roots are unknown
commands, not aliases.

Machine JSON, NDJSON, plan identities, digests, and audit records stay
locale-neutral. Human CLI/TUI/desktop copy is English or Simplified Chinese.
Mutating commands require a saved domain plan, a live `sha256:` preview digest,
and `--confirm`. Software uninstall is current-user MSIX only. Optimize
executes DNS cache flush and can launch frozen Settings URIs; a Settings launch
is not maintenance completion. Analyze and Status never create cleanup
authority.

See [`docs/guide/cli-migration.md`](docs/guide/cli-migration.md) and
[`docs/safety-capability-matrix.md`](docs/safety-capability-matrix.md). Native
Windows evidence is recorded in
[`docs/validation/five-mode-native.md`](docs/validation/five-mode-native.md).
Local packaging remains unsigned. This task does not push, sign, publish, or
release.

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

## License

`devsweep` is distributed under the MIT License. See [`LICENSE`](LICENSE) for
the full text.

Design provenance relative to other public cleanup tools is recorded in
[`docs/provenance.md`](docs/provenance.md).
