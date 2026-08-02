# Getting Started

DevSweep is a Rust CLI for planning and executing developer-workspace cleanup.
It is designed around reviewable plans rather than implicit deletion.

## Prerequisites

- Rust 1.88.0 or later to build from this repository.
- Node.js 18 or later only when serving or building this documentation site.

This repository contains a small `process_fixture` test binary as well as the
main program. Select `devsweep` explicitly when using `cargo run`.

## Build and inspect the CLI

```powershell
just build
cargo run --locked --bin devsweep -- --help
```

The public commands are `tui`, `scan`, `inventory`, `clean`, `protect`, and
`rules`. The [CLI reference](/reference/cli) lists their arguments.

## Create a first plan

Scan the current project and serialize the report for review:

```powershell
cargo run --locked --bin devsweep -- scan . --json > plan.json
```

The resulting document is a scan report containing a cleanup plan and scan
health. Read [plans and reports](/reference/plan-and-report) before editing or
reusing it.

## Dry-run before execution

Run the saved plan without side effects first:

```powershell
cargo run --locked --bin devsweep -- clean --plan plan.json
```

Only after reviewing the plan should you request execution:

```powershell
cargo run --locked --bin devsweep -- clean --plan plan.json --execute --audit-log devsweep-audit.jsonl
```

`--execute` is intentionally opt-in. It does not enable permanent deletion,
Docker cleanup, or Cargo-home cleanup.

## Open the interactive interfaces

```powershell
just dev
just docs
```

`just dev` opens the terminal UI. `just docs` starts this VitePress site; run
`npm ci` first when the documentation dependencies are not installed.
