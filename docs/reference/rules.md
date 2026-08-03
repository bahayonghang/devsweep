# Rules

Run the current catalog on your machine with:

```powershell
cargo run --locked --bin devsweep -- rules
```

The command prints rule ID, risk, action kind, and a short summary. It is the
authoritative list for the installed binary.

## Project rules

Current project-level rule IDs cover Rust `target/`, Node `node_modules` and
common build caches, Python virtual environments and tool caches, and Python
`__pycache__` directories. Some project artifacts are trash-backed; Rust
`target/` uses `cargo clean` after validating the relevant Cargo metadata and
project marker.

Examples of current IDs include:

```text
rust.target
node.node_modules
node.next_cache
node.turbo
python.venv_dot
python.pytest_cache
python.__pycache__
```

## Global providers and caches

Supported global provider rules include npm, pip, pnpm, and Yarn cache
commands, plus selected caches for Go, Gradle, Maven, Ivy, NuGet, JetBrains,
and Hugging Face. Command-backed providers use fixed registry-owned argument
templates rather than strings from a saved plan.

Some entries are intentionally inspect-only, including Cargo home, Maven's
local repository, the direct Go module-cache inspection entry, JetBrains cache
roots, and Hugging Face hub models. The separate `go.mod_cache.clean` provider
rule uses `go clean -modcache`. Docker is listed as deferred rather than a
current cleanup target.

## Risk and action labels

Risk levels help prioritize review; they do not replace the required explicit
execution decision. An `inspect only` or `deferred` rule is not eligible for
cleanup execution. Always inspect `rules` after upgrading DevSweep, because the
catalog is part of the program version's behavior.
