# Design - Clean Workbench

## State machine

```text
idle -> scanning -> report_ready -> selecting -> previewing -> preview_ready
                                                      -> confirming -> executing
any active observation -> canceling -> canceled/partial
execution -> completed/partial/failed/unknown
```

Each report, selection, plan, and preview carries immutable identity. UI state
never synthesizes a plan. Execution IPC accepts only the saved plan and current
preview digest. Operation ids reject late progress/completion after cancellation.

## Presentation

The workbench uses a compact scan scope row, status/progress strip, grouped target
list, expandable evidence/path details, and sticky bottom summary/action. The
empty scanning surface may use restrained CSS-native sweep motion only after the
shell-owned spec update passes R6/AC5; otherwise it remains motion-free. It is
never a hero image or evidence-bearing graphic. Size labels include evidence
state and uncertainty. Search and sort do not mutate underlying selection
identity.

CLI/TUI/Desktop share typed outcomes and catalogue keys, but each presentation
owns its interaction adapter. Desktop virtualizes only if profiling proves it is
needed; accessibility and deterministic row state take precedence.

## Clean V1 audit contract

This task replaces the legacy Clean journal writer with the sole V1 writer at
`%LOCALAPPDATA%\DevSweep\audit\v1\clean.jsonl`. The envelope is a closed union
with `schema_version: 1`, `domain: "clean"`, `record_kind`, `operation_id`, UTC
Unix-millisecond timestamp, monotonic transition, stable outcome/error codes,
and redacted typed evidence. `record_kind` has only `execution_transition` and
`protection_mutation`; neither variant contains command strings, argv,
environment, raw action paths, plan payloads, or localized text. The Protection
task owns emitting add/remove mutation transitions through the accepted writer;
it does not create another journal or extend the union during implementation.

The writer resolves only the fixed V1 path, creates its parent when safe, and
holds an OS-visible exclusive sidecar lock from version validation through
append, flush, and durable close. Lock/path/create/append/flush failure blocks
the side effect. An unknown version or corrupt existing line preserves every
original byte and fails closed. The writer never probes the removed
`--audit-log` argument or `%APPDATA%\devsweep\audit.jsonl`. Golden fixtures form
the History handoff and prove a reader cannot reconstruct executable text or a
raw cleanup target path.

## Dependencies and rollback

Depends on the CLI contract, shell, and accepted bounded-sizing child. Existing
cleanup model/executor remains canonical. Rollback restores the previous Clean
page/commands without touching plans or audit data; schema migration is explicit
and versioned rather than in-place reinterpretation.

The handler consumes the frozen CLI forms `clean scan`, `clean plan`,
`clean preview`, and `clean execute`; it may not edit the parser, aliases,
digest grammar, output behavior, or exit classes.

## Exact change list

| Path | Ownership and planned change |
| --- | --- |
| `crates/devsweep-core/src/services.rs` | expose existing Clean services through typed mode adapter only |
| `crates/devsweep-core/src/execution/audit.rs` | sole fixed-path Clean V1 envelope/writer, exclusive lock, durability, version refusal, and redaction fixtures |
| `crates/devsweep-core/src/execution/mod.rs` | emit validated Clean execution transitions through the accepted V1 writer |
| `crates/devsweep-cli/src/application/commands/clean.rs` | new frozen-grammar Clean handler |
| `crates/devsweep-cli/src/application/presentation/clean.rs` | bilingual Clean human renderer over typed outcomes |
| `crates/devsweep-cli/src/tui/modes/clean/` | Clean-local reducer/effects/views/tests |
| `desktop/src-tauri/src/clean.rs` | extract typed Clean commands and operation-id events |
| `desktop/src-tauri/src/lib.rs` | register Clean adapter after shell contract |
| `desktop/src/modes/clean/` | Clean reducer/pages/components/tests |
| `desktop/src/api/fixtures/clean/` | versioned scan/plan/preview/execution fixtures |
| `crates/devsweep-core/tests/fixtures/history/clean-v1/` | redacted Clean execution/protection records plus corrupt/unknown/legacy non-discovery fixtures for History handoff |
| `desktop/scripts/generate-types.mjs` | include frozen Clean DTO fixtures |
| `docs/guide/clean.md` and `docs/zh/guide/clean.md` | update staged command and safety flow |

The CLI contract task first creates `application/commands/mod.rs` and
`application/presentation/mod.rs`; this task fills only its frozen `clean.rs`
handler/renderer submodules and does not own either module root.

The task does not own `application/cli.rs`, shell/spec files, sizing
implementation, Protection/Rules, Software, Optimize, Analyze, or Status files.
