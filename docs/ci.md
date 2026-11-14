# CI and Releases

## Required CI jobs

| Job | Purpose |
| --- | --- |
| `rust` on Windows, Ubuntu, and macOS | Format, locked check, locked tests, and Clippy with warnings denied. |
| `docs` | Node 22 documentation site build from the root `package-lock.json` (`npm ci` then `npm run docs:build`). |
| `msrv` | Check and test core/CLI on the exact `package.rust-version` from `Cargo.toml` on Ubuntu and Windows. |
| `cargo-audit` | RustSec advisory gate. |
| `cargo-deny` | License, source, and advisory policy from `deny.toml`. |
| `gitleaks` | Secret scan. |

Hosted CI runs on `pull_request` and `workflow_dispatch` only. Direct pushes to
`dev` or `main` do not start this workflow.

Non-Windows Rust jobs and all MSRV jobs select `-p devsweep-core -p devsweep-cli`
explicitly. The GPUI desktop app (`desktop-gpui/`) is a standalone workspace with
its own local gate (`just gpui`) and is not part of hosted CI.

All jobs use read-only contents permission, cancel-in-progress concurrency, and
bounded timeouts.

## Node and npm

The documentation site uses Node 22. Install its locked dependencies from the
repository root `package-lock.json` with `npm ci`.

## Local Rust gate

```powershell
just ci
```

The recipe checks formatting, checks all targets with `--locked`, runs all
tests, and runs Clippy with warnings denied. It does **not** update `Cargo.lock`.

Refresh the lockfile only with the explicit maintenance recipe:

```powershell
just sync-lock
```

## Documentation build

Install the documentation dependency graph from the root lockfile, then build
the static site. Hosted CI runs the same commands on Node 22; a non-zero status
fails the `docs` job.

```powershell
npm ci
npm run docs:build
```

For local authoring, use `just docs`. It runs the VitePress development server
and does not publish anything.

## Release archive

`just dev` runs the workspace CLI binary with no extra argv. Interactive
terminal entry is bare `devsweep` and requires stdin and stdout TTYs. There is
no `tui` subcommand.

Build a host-triple archive containing the executable, `LICENSE`, `README.md`,
and a SHA-256 sidecar:

```powershell
just release-archive
just release-smoke
```

`just release-smoke` uses the exact `devsweep.exe` from that new archive, not a
`target/release` leftover. It prepares a task-owned temporary project fixture,
then runs `clean scan` → `clean plan` → `clean preview` with `--scope projects`
only. It does not run `clean execute`, `software uninstall`, or `optimize run`,
and it does not substitute `--help` or a handmade empty plan for those commands.

The archive is written below `dist/`, which is generated output and is not
committed.

## MSRV

`Cargo.toml` `rust-version` is the single source of truth. CI fails when the
MSRV job toolchain does not match that value.
