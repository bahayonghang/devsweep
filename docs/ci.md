# CI and release gates

## Required jobs

| Job | Purpose |
| --- | --- |
| `rust` on windows/ubuntu/macos | `cargo fmt`, `check --locked`, `test --locked`, `clippy --locked -D warnings` |
| `msrv` | Exact `package.rust-version` from `Cargo.toml` on ubuntu + windows |
| `cargo-audit` | RustSec advisory gate |
| `cargo-deny` | license/source/advisory policy via `deny.toml` |
| `gitleaks` | secret scan |

All jobs set `permissions: contents: read`, workflow concurrency cancel-in-progress,
and per-job `timeout-minutes`.

## Local parity

```powershell
just ci
```

Release archive (host triple, LICENSE+README, SHA-256):

```powershell
just release-archive
just release-smoke
```

## MSRV

`Cargo.toml` `rust-version` is the single source. CI fails if the MSRV job
toolchain does not match that field.
