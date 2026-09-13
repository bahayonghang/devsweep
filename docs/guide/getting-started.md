# Getting Started

DevSweep is a Rust CLI for planning and executing developer-workspace cleanup.
It is designed around reviewable plans rather than implicit deletion.

## Prerequisites

- Rust 1.88.0 or later to build from this repository.
- Node.js only when serving or building this documentation site, or when
  developing the desktop app (Node.js 22).

This repository contains a small `process_fixture` test binary as well as the
main program. Select `devsweep` explicitly when using `cargo run`.

## Build and inspect the CLI

```powershell
just build
cargo run --locked --bin devsweep -- --help
```

The shipped roots are `clean`, `software`, `optimize`, `analyze`, `status`, and
`history`. There is no root `tui`, `scan`, `inventory`, `protect`, or `rules`
command. The [CLI reference](/reference/cli) lists arguments. If you still have
old invocations, use the labeled [CLI migration](/guide/cli-migration) table; it
is not a current tutorial.

## Create a first plan

Scan, save the observation, then write a **new** plan from exact target IDs:

```powershell
cargo run --locked --bin devsweep -- clean scan --root . --scope all --format json --output observation.json
```

Read target IDs from `data.plan.targets[].id` in that V1 envelope. Then:

```powershell
cargo run --locked --bin devsweep -- clean plan --observation observation.json --select TARGET_ID --output plan.json
```

The observation file is a scan report (often wrapped in the machine envelope).
It is **not** an executable plan. `clean preview` and `clean execute` require
the new plan file. Read [plans and reports](/reference/plan-and-report) before
editing or reusing either document.

## Preview before execution

Preview is the dry-run. It calculates a live `sha256:` digest and performs no
cleanup:

```powershell
cargo run --locked --bin devsweep -- clean preview --plan plan.json
```

Execute only after that live digest and `--confirm`. Inspect the contract with
`--help`; do not run execute as a first-run command:

```powershell
cargo run --locked --bin devsweep -- clean execute --help
```

```powershell
cargo run --locked --bin devsweep -- clean execute --plan plan.json --preview-digest sha256:DIGEST --confirm
```

There is no `--execute` flag and no `--audit-log` path. Permanent deletion,
Docker cleanup, and Cargo-home cleanup stay disabled.

## Open the interactive interfaces

The TUI is bare `devsweep` from an interactive stdin/stdout TTY:

```powershell
cargo run --locked --bin devsweep
```

`devsweep tui` is rejected (exit 2). `just dev` currently still passes `tui`;
that recipe is a later release-contract fix, not the current tutorial.

```powershell
just docs
```

`just docs` starts this VitePress site; run `npm ci` first when the
documentation dependencies are not installed.

```powershell
just tdev
```

`just tdev` starts the Tauri desktop development window from the repository
root. Install `desktop/` dependencies with `npm ci` first (Node.js 22).
