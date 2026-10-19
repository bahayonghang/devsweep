# C01 Shared services and CLI feature parity

Status: **implemented, not accepted**. Windows x64-first scope confirmed. C16-AC3 and local Rust 1.88.0 remain FAIL or NOT_RUN.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C16](../10-15-gpui-latest-dependencies/prd.md).

## Goal

Expose existing desktop-only Rust capabilities through stable CLI commands and freeze shared operation contracts.

## Requirements

C01-R1. Expose existing update checks, startup inventory/toggle, leftover discovery/plan/preview/execute, and Analyze reveal/trash through additive CLI subcommands.

C01-R2. Keep existing commands, locale-neutral output/exit classes, saved-plan versions and TUI routing compatible; version new shapes explicitly.

C01-R3. Move only frontend-independent orchestration out of Tauri where needed. CLI and GPUI consume the same Rust service boundary.

C01-R4. Bring new CLI mutations under saved plan, live digest, explicit confirmation and audit contracts; the existing startup confirmed-bool API needs a compatible strict wrapper rather than being assumed sufficient.

## Acceptance criteria

- [x] C01-AC1: Parser/help and generated EN/zh-CN references reach every added leaf; old command/fixture compatibility passes. Evidence: rustc 1.99.0, `just ci` exit 0 on 2026-10-17 (`english_and_chinese_references_exactly_match_the_clap_manifest`, `every_frozen_root_and_nested_shape_parses`, `cli_contract` 13 passed, `five_mode_contract` 8 passed).
- [x] C01-AC2: CLI and direct service calls have equal reports, stable identities, refusal reasons and exit semantics for identical isolated inputs. Evidence: `c01_shipped_parity` 8 passed, and `just ci` exit 0 on 2026-10-17. The three success previews still compare live identities and digests. `startup_preview_refusal_matches_the_core_service`, `leftover_preview_refusal_matches_the_core_service`, and `analyze_trash_preview_refusal_matches_the_core_service` each call the core service and spawn `CARGO_BIN_EXE_devsweep` on the same stale fixture. The CLI refusal codes match the core codes `startup_entry_changed`, `stale_candidate`, and `changed_since_snapshot`, each process exits 3, and the fixture bytes stay unchanged.
- [x] C01-AC3: Tampering, selection/root/protection drift, missing confirm, partial scans, output collisions and broken pipes are covered before any fake side effect. Evidence: existing tamper, drift, and output-collision tests remain. `analyze_trash_execute_without_confirm_leaves_the_fixture` exits 2 with the fixture unchanged. `analyze_trash_execute_closed_stdout_does_not_move_the_fixture` exits 0 with `broken_pipe` and no fixture or clean-journal change. `just ci` exit 0 on 2026-10-17.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
