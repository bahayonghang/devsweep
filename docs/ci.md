# CI and Releases

## Required CI jobs

| Job | Purpose |
| --- | --- |
| `rust` on Windows, Ubuntu, and macOS | Format, locked check, locked tests, and Clippy with warnings denied. |
| `msrv` | Check and test the exact `package.rust-version` from `Cargo.toml` on Ubuntu and Windows. |
| `cargo-audit` | RustSec advisory gate. |
| `cargo-deny` | License, source, and advisory policy from `deny.toml`. |
| `gitleaks` | Secret scan. |

All jobs use read-only contents permission, cancel-in-progress concurrency, and
bounded timeouts.

## Local Rust gate

```powershell
just ci
```

The recipe checks formatting, synchronizes the local Cargo lock state, checks
all targets, runs all tests, and runs Clippy with warnings denied.

## Documentation build

Install the documentation dependency graph, then build the static site:

```powershell
npm ci
npm run docs:build
```

For local authoring, use `just docs`. It runs the VitePress development server
and does not publish anything.

## Release archive

Build a host-triple archive containing the executable, `LICENSE`, `README.md`,
and a SHA-256 sidecar:

```powershell
just release-archive
just release-smoke
```

The archive is written below `dist/`, which is generated output and is not
committed.

## MSRV

`Cargo.toml` `rust-version` is the single source of truth. CI fails when the
MSRV job toolchain does not match that value.
