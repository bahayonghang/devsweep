# C01 implementation entry points

Inspected on 2026-10-15 while C16 is executing. This is read-only preparation,
not C01 acceptance or a new domain authority contract. C16 must pass its required
dependency gate before C01 implementation begins.

## Existing boundary

- `crates/devsweep-cli/src/application/cli.rs` owns frozen Clap grammar. Software
  currently has inventory/plan/preview/uninstall; Analyze has scan only.
- Software command handlers are a directory, `application/commands/software/`
  (`mod.rs`, `inventory.rs`, `execution.rs`), not `commands/software.rs`.
- `application/commands/analyze.rs` owns scan handling, output envelopes and tests.
- Core `services.rs` already has frontend-neutral Clean adapters. Do not add a new
  generic framework merely to expose domain functions that already have a stable API.
- Preserve create-new output sinks, locale-neutral machine results and existing
  dispatcher/error/exit rules; generated help and both reference docs must match Clap.

## API inventory and missing strict wrappers

| Capability | Existing core entry point | C01 work before CLI exposure |
| --- | --- | --- |
| Updates check | `software/updates.rs::check_software_updates` | Add read-only CLI route and human/machine presentation; no update execution in C01. |
| Startup list | `software/startup.rs::list_startup_entries` | Expose source completeness, current-user vs machine scope and state faithfully. |
| Startup toggle | `software/startup.rs::set_startup_enabled` | The existing API only takes entry ID, requested bool and confirmed bool. Add a saved-plan/live-preview wrapper with live identity binding and durable audit before CLI mutation. |
| Leftover discovery | `software/leftovers.rs::discover_software_leftovers` | Consume validated inventory and explicit stable software IDs, preserving inspect-only/ownership evidence. |
| Leftover plan/execute | `plan_software_leftovers`, `execute_software_leftovers` | Existing V1 plan binds software identity, succeeded uninstall operation and candidate IDs. Add a reusable standalone live-preview route without treating serialized preview as authority. |
| Analyze reveal | `analysis/actions.rs::reveal_analyze_node` | Validate saved snapshot/node input and supported platform; reconstruct the path through core, never from a displayed path. |
| Analyze recycle | `preview_analyze_trash`, `execute_analyze_trash` | Existing API consumes retained snapshot plus node IDs, live digest and confirmation. Define versioned saved selection/plan shape and strict decoding for the CLI boundary. |

Startup entries deliberately contain no executable path or command line. Stable IDs
identify source/name, not necessarily unchanged registry content. The new wrapper
must bind the live entry/approval evidence required to refuse replacement and state
drift; do not weaken privacy by putting raw command lines in CLI output. Preserve
the legacy bool API's existing supported behavior until its consumers migrate.

Leftover plans require a succeeded uninstall audit operation. A user-written JSON
claim must not stand in for that durable record. C01 exposes the existing operation;
broader cleanup/provider support and reversible startup backup belong to C05.

Analyze V1 previews contain display paths, but execute accepts a snapshot plus node
IDs and reconstructs the plan. Preserve that separation when introducing a saved
CLI plan. Corrupted snapshots, duplicate IDs, out-of-root paths, partial observations,
selection changes and live filesystem/protection drift need refusal tests.

## Technical freeze checklist for C01 implementer

1. Freeze additive grammar from the parent CLI proposal without renaming old leaves.
2. Define only the missing versioned saved shapes; reuse existing V1 domain DTOs when
   their meaning remains unchanged. Preserve deny-unknown-fields on old contracts.
3. Trace each mutation through validation, live digest, confirmation, audit and
   read-back. Validate output destinations before dispatch so a collision cannot
   hide a real action result. Keep all command program/argv values separate.
4. Define narrow injectable boundary tests proving real handler routing and core
   parity; never run changes against actual startup entries or user files.
5. Update exact file ownership before editing shared CLI/core registries. C16 may
   touch Windows bindings in these modules, so begin from its accepted source state.
