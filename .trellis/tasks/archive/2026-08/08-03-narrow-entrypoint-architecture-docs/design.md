# Entrypoint And Architecture Documentation Design

## Application Interface

The final external Rust interface is intentionally one function:

```rust
// src/lib.rs
mod application;
// remaining implementation modules are private

pub fn run() -> anyhow::Result<()> {
    application::run()
}
```

```rust
// src/main.rs
fn main() -> anyhow::Result<()> {
    devsweep::run()
}
```

The actual implementation may add crate documentation or a direct re-export of
the function, but it must not expose internal modules to make the binary compile.

## Application Ownership

```text
application/
  mod.rs       tracing initialization, Cli::parse, command dispatch
  cli.rs       clap structs/enums only
  commands.rs  scan/inventory/clean/protect/rules handlers and CLI formatting
```

At roughly the current `main.rs` production size, one `commands.rs` remains
cohesive enough. Split a command into its own file only if the final code shows
an independent parsing/I/O policy and tests; do not create five forwarding
files for visual symmetry.

The application module calls private model/plan/scan/inventory/execution/rules/
TUI interfaces. Those modules do not import application.

Saved-input decoding stays adjacent to the clean command because it is an
application trust-entry concern. It still rejects inventory reports, rejects
legacy plan v1 with rescan guidance, validates scan report version, rejects
unknown executable fields through typed deserialization, and adds path context
to file/JSON failures.

## Visibility Policy

- `pub`: only the deliberate crate entrypoint and unavoidable fields/items of
  that interface.
- `pub(crate)`: implementation values genuinely shared across top-level
  modules.
- `pub(super)`: directory-child collaboration.
- private: default for implementation and helpers.

A private module may contain a `pub(crate)` interface; that does not make it a
supported external Rust interface. Existing `pub mod` lines and public types
must be reviewed rather than mechanically changed.

Temporary re-exports from earlier children are deleted after all call sites use
the final paths. There is no deprecated compatibility layer.

## Documentation Ownership

- `code_map.md` is the primary maintained navigation map and must list every
  final production module/directory with its true responsibility.
- `.trellis/spec/backend/directory-structure.md` records final backend module
  ownership, dependency direction, and visibility.
- Frontend directory/component/state/hook/type/quality specs record the final
  `app/runtime/render/display` implementation and existing behavioral contracts.
- Backend quality/error/logging specs keep behavioral contracts and update only
  stale file paths or owner names.
- `README.md` changes only if it names a retired Rust path or source layout.
- Historical `design.md` remains historical evidence and is not normalized to
  the final tree.

## Public Interface Verification

Use source inspection and generated docs:

```powershell
rg -n "^pub (mod|use) " src
rg -n "^pub (fn|struct|enum|trait|type|const|static)" src/lib.rs src
cargo doc --locked --no-deps
```

Public items inside private modules may appear in source search but must not be
externally reachable. Inspect generated `devsweep` documentation to confirm the
external surface. Do not add a new development dependency solely for interface
inspection.

## Compatibility

The application move is behavior-preserving. Exact human-readable output and
tests move with the handlers. Machine-readable JSON continues to use stdout
only. Rust path source compatibility is deliberately not preserved.

## Risks And Rollback

- Narrowing visibility can expose hidden coupling late. Fix the caller/owner
  relation rather than re-publicizing the whole module.
- Moving `Cli::parse` changes process-exit behavior if replaced with
  `try_parse`; retain the current parsing semantics unless tests require an
  internal injected-args helper.
- Moving command tests can drop binary-crate coverage. Preserve every named
  assertion and add a thin entrypoint compile check through all-target CI.
- Documentation can drift if written before final paths settle. Update it after
  code and structural searches, then compare `rg --files src` to `code_map.md`.

Rollback is the final child work commit. Earlier completed child commits remain
valid independently.

