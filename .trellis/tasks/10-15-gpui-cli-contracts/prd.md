# C01 Shared services and CLI feature parity

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
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

- [ ] C01-AC1: Parser/help and generated EN/zh-CN references reach every added leaf; old command/fixture compatibility passes.
- [ ] C01-AC2: CLI and direct service calls have equal reports, stable identities, refusal reasons and exit semantics for identical isolated inputs.
- [ ] C01-AC3: Tampering, selection/root/protection drift, missing confirm, partial scans, output collisions and broken pipes are covered before any fake side effect.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
