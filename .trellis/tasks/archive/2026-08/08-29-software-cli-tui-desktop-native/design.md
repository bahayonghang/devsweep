# Design - Software Presentation

Use a mode-local typed state machine: loading inventory, ready/selecting,
previewing, preview-ready, confirming, uninstalling, and terminal outcomes. Rows
key only by tagged identity. Grouping is presentational and cannot merge selection
identity. A detail panel explains source, scope, evidence, refusal, and expected
Windows-owned interaction without showing dangerous registry fields or the
MSIX installed path. It consumes the inventory task's size union verbatim:
reported estimates are labelled estimated, measured lower bounds are labelled
at least, and unknown is never rendered as zero. It consumes the last-used union
verbatim; `unknown/no_supported_exact_source` is explicit, and install/update
dates never appear in that column.

The sticky summary reports selected current-user MSIX count and evidenced
app-package size only;
unknown related data is explicit. Confirmation repeats exact app identity,
irreversibility, scope, and preview digest. TUI/Desktop use the shell coordinator;
CLI uses the frozen staged grammar. Rollback removes presentation registration
while keeping inventory and audits accessible to tests/diagnostics.

The terminal reducer accepts only `removed`, `reboot_required`, `still_present`,
`failed`, and `unknown_after_dispatch`; `partial` belongs only to source/evidence
DTOs and is rejected as an execution terminal. Crash/restart fixtures replay the
durable audit terminal selected by the core state machine and never redispatch.

The task owns the Software human renderer and final fixture parity, not parser or
handler grammar. `software inventory|plan|preview|uninstall` remains exactly as
frozen by the CLI task.

## Exact change list

| Path | Ownership and planned change |
| --- | --- |
| `crates/devsweep-cli/src/application/presentation/software.rs` | bilingual human renderer over typed outcomes |
| `crates/devsweep-cli/src/tui/modes/software/` | mode-local reducer/views/confirmation/tests |
| `desktop/src-tauri/src/software.rs` | typed inventory/preview/uninstall/audit commands/events |
| `desktop/src-tauri/src/lib.rs` | register Software IPC after core gates |
| `desktop/src/modes/software/` | reducer, selectors, workbench, detail, confirm, outcome tests |
| `desktop/src/api/fixtures/software/` | tagged inventory/refusal/size/last-used, preview, five-terminal outcome, source-partial, and restart fixtures |
| `desktop/scripts/generate-types.mjs` | generate Software DTOs |
| `docs/guide/software.md` and `docs/zh/guide/software.md` | standard-user irreversible Software guide |

The CLI contract task first creates `application/presentation/mod.rs`; this task
fills only its frozen `presentation/software.rs` renderer and does not own the
module root.

The task consumes the shell-owned updated desktop spec and does not edit it.
