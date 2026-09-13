# devsweep

`devsweep` is a safety-first developer cleanup planner and executor. It scans project build artifacts and global developer caches, writes an observation, lets you save an explicit plan, and executes only after a live preview digest and `--confirm`.

## Safety Model

- Cleanup is observational until you save a **new** plan. `clean scan` JSON is an observation, not executable authority.
- Dry-run is `clean preview`. It prints a live `sha256:` digest and performs no cleanup.
- Execution requires a saved plan, that live digest, and `--confirm`. There is no `--execute` flag and no `--audit-log` path.
- Saved plans are declarative v2 documents. They carry observed facts and a typed cleanup intent; DevSweep validates them and reconstructs trusted actions from its built-in rule registry.
- Every cleanup target carries evidence and a risk level. Evidence explains why the target was considered cleanable.
- Global package-manager cleanup is command-backed. npm, pip, pnpm, and Yarn actions use registry-owned argv templates, not shell strings from a plan file.
- Project cleanup is trash-backed by default for directories such as `node_modules`, `.venv`, and tool caches.
- Rust project `target` cleanup prefers `cargo clean --manifest-path <Cargo.toml>`.
- Cargo home is inspect-only in the MVP. `devsweep` does not delete `%USERPROFILE%\.cargo`, `~/.cargo`, credentials, installed binaries, registry internals, or git cache internals.
- Permanent delete is disabled in this build and is not exposed as a CLI flag.
- Docker cleanup is deferred and is not part of the MVP.
- Mutating commands append only to the fixed stores under `%LOCALAPPDATA%\DevSweep\audit\v1\<domain>.jsonl`. Legacy `%APPDATA%\devsweep\audit.jsonl` is not used, imported, or searched.

The scanner does not follow symlinked cleanup directories. On Windows, reparse-point directories are skipped by the same guard; junction-specific behavior depends on the OS exposing the reparse-point metadata.

## Usage

Shipped roots are `clean`, `software`, `optimize`, `analyze`, `status`, and `history`. Old `tui`, `scan`, `inventory`, `protect`, and `rules` roots are unknown commands, not aliases.

Open the TUI from an interactive stdin/stdout TTY (no `tui` subcommand):

```powershell
devsweep
```

From this repository, the equivalent is `cargo run --locked --bin devsweep` with both streams attached to a TTY. `devsweep tui` is rejected (exit 2). `just dev` still passes `tui` today; that recipe belongs to a later release-contract fix and is not the current tutorial.

Save an observation, create a new plan from exact target IDs, then dry-run with a live preview digest:

```powershell
devsweep clean scan --root . --scope all --format json --output observation.json
devsweep clean plan --observation observation.json --select TARGET_ID --output plan.json
devsweep clean preview --plan plan.json
```

Read target IDs from the observation JSON (`data.plan.targets[].id` in the V1 envelope). Observation JSON is not a runnable plan; `clean preview` and `clean execute` require the **new** plan file.

Execute only after the live digest and explicit confirmation. Confirm the grammar with `--help`; do not treat this as a first-run command:

```powershell
devsweep clean execute --help
devsweep clean execute --plan plan.json --preview-digest sha256:DIGEST --confirm
```

Related inspect commands:

```powershell
devsweep analyze scan --root . --format json
devsweep software inventory --source all
devsweep optimize list
devsweep status snapshot
devsweep history list
devsweep clean protect list
devsweep clean rules list
```

`--language <en|zh-CN>` localizes human output only. It is invalid with JSON or NDJSON.

See [`docs/guide/cli-migration.md`](docs/guide/cli-migration.md) for labeled old→new mappings. Native Windows evidence is historical; see [`docs/validation/five-mode-native.md`](docs/validation/five-mode-native.md).

## Desktop Development

The Tauri 2 desktop MVP targets Windows and requires Node.js 22. Install its
locked frontend dependencies, then start the native development window from the
repository root:

```powershell
cd desktop
npm ci
```

```powershell
just tdev
```

`just tdev` runs `npm run tauri -- dev` in `desktop/`. The equivalent from that
directory is `npm run tauri -- dev`.

From the repository root, run the desktop frontend and Rust checks or build an
unsigned NSIS installer:

```powershell
just desktop-web-check
just desktop-test
just desktop-build
just tinstall
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
and Rules. Bare `devsweep` from an interactive TTY opens the TUI.

Machine JSON, NDJSON, plan identities, digests, and audit records stay
locale-neutral. Human CLI/TUI/desktop copy is English or Simplified Chinese.
Mutating commands require a saved domain plan, a live `sha256:` preview digest,
and `--confirm`. Software uninstall is current-user MSIX only. Optimize
executes DNS cache flush and can launch frozen Settings URIs; a Settings launch
is not maintenance completion. Analyze and Status never create cleanup
authority.

See [`docs/safety-capability-matrix.md`](docs/safety-capability-matrix.md).
Local packaging remains unsigned. This repository does not push, sign, publish,
or release from documentation tasks.

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
