# CI and Releases

## Required CI jobs

| Job | Purpose |
| --- | --- |
| `rust` on Windows, Ubuntu, and macOS | Format, locked check, locked tests, and Clippy with warnings denied. |
| `desktop` on Windows | Node 22 desktop frontend types drift check, lint, typecheck, test, and Vite build as separate steps, then unsigned Tauri compile (`tauri build --no-bundle`). |
| `docs` | Node 22 documentation site build from the root `package-lock.json` (`npm ci` then `npm run docs:build`). |
| `msrv` | Check and test the exact `package.rust-version` from `Cargo.toml` on Ubuntu and Windows. |
| `cargo-audit` | RustSec advisory gate. |
| `cargo-deny` | License, source, and advisory policy from `deny.toml`. |
| `gitleaks` | Secret scan. |

Hosted CI runs on `pull_request` and `workflow_dispatch` only. Direct pushes to
`dev` or `main` do not start this workflow.

All jobs use read-only contents permission, cancel-in-progress concurrency, and
bounded timeouts. Hosted Desktop frontend checks are split into one step per npm
script so a native non-zero exit cannot be swallowed by a later command. Types
drift is compared before any generate-write. The comparison ignores
working-tree CRLF so Windows checkouts of the LF blob do not fail.

## Node and npm

The desktop app declares `engines.node` `>=22 <23` and `engines.npm` `>=10 <12`
(npm 10 or 11) in `desktop/package.json`. Hosted `desktop` and `docs` jobs use
Node 22. Local desktop gates must use that same Node 22 and npm 10 or 11 range;
newer system Node/npm (for example Node 26 / npm 12) is outside the supported
engines.

Install locked dependencies with `npm ci`:

- Documentation: repository root `package-lock.json`.
- Desktop: `desktop/package-lock.json`.

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

## Local desktop gate

```powershell
just desktop-web-check
```

The recipe runs these named npm scripts in order under `desktop/`:
`types:generate -- --check`, `lint`, `typecheck`, `test`, and `build`. Each
invocation's native exit code is checked; a non-zero status stops the entry.

On Windows PowerShell, chaining native commands with `;` does not stop on an
early failure. This recipe therefore checks `$LASTEXITCODE` after each npm
invocation instead of relying on the last command.

`types:generate --check` compares committed `desktop/src/api/types.gen.ts` to
the generator's `--stdout` result and does not write. Newlines are canonicalized
to LF before comparison, so a Windows CRLF working tree is not treated as drift.
Unflagged `npm run types:generate` still writes and is the explicit maintenance
path.

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
