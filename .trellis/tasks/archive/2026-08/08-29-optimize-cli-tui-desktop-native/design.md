# Design - Optimize Presentation

Use catalogue DTOs as the sole rendering source. Present compact operation rows
with a categorical badge: Runs here, Opens Windows Settings, or Guidance only.
Expanded details describe exact effect/risk/capability. Selection stores stable
ids; preview returns resolved identities and digest. The bottom action label and
confirmation change by class and cannot collapse launch into execution.

TUI/Desktop use shell coordination and operation-id scoped events; CLI uses the
frozen staged grammar. Terminal copy distinguishes success, launched, canceled
before start, failed, unsupported, and unknown after dispatch. Rollback removes
the mode registration while leaving its versioned audit readable.

All surfaces consume the catalogue child's typed build capability and refusal
evidence, including `RtlGetVersion` query failure. This task does not query the
OS build, add a fallback version helper, or reinterpret an unavailable result.

## Exact change list

| Path | Ownership and planned change |
| --- | --- |
| `crates/devsweep-cli/src/application/presentation/optimize.rs` | bilingual human renderer over exact catalogue DTOs |
| `crates/devsweep-cli/src/tui/modes/optimize/` | mode reducer/rows/preview/confirm/outcomes/tests |
| `desktop/src-tauri/src/optimize.rs` | typed list/preview/run/history commands/events |
| `desktop/src-tauri/src/lib.rs` | register Optimize IPC after core gate |
| `desktop/src/modes/optimize/` | reducer, catalogue workbench, details, confirm/outcome tests |
| `desktop/src/api/fixtures/optimize/` | all eight ids/build/refusal/outcome fixtures |
| `desktop/scripts/generate-types.mjs` | generate Optimize DTOs |
| `docs/guide/optimize.md` and `docs/zh/guide/optimize.md` | exact closed catalogue and no-admin guide |

The CLI contract task first creates `application/presentation/mod.rs`; this task
fills only its frozen `presentation/optimize.rs` renderer and does not own the
module root.

This task does not own parser/catalogue/WOW64/URI decisions or the shell-owned
desktop spec. Any requested id/URI/build/path change returns to the core task.
