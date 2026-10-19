# C15 AI tool cleanup and recovery through the CLI

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C01](../10-15-gpui-cli-contracts/prd.md), [C03](../10-15-gpui-clean-coverage/prd.md).

## Goal

Add first-class Claude Code Codex Cursor and related AI-tool cleanup with explicit data classes recovery and active-use protection.

## Requirements

C15-R1. Define versioned Windows-aware adapters for Claude Code, Codex, Cursor and a finite researched set of additional AI tools, including custom home/config roots and installation ownership.

C15-R2. Separate regenerable caches/temp/debug artifacts, superseded tool versions, conversation/session archives, Git worktrees and installed CLI tools. Inventory size, last-use evidence, active-owner state, rebuild cost and recovery guarantees.

C15-R3. Provide CLI inventory/plan/preview/execute plus recovery inspection/restore planning. A restore intent executes through the same live preview/digest/confirmation path and revalidates bundle integrity, destination and provider/index state. General cache cleaning never bulk-selects transcripts, worktrees, settings, credentials or models.

C15-R4. For opt-in session cleanup, use supported provider behavior or an atomically consistent recovery bundle; validate content/metadata/index consistency, preserve active sessions and abort on unsupported versions. Unknown active ownership, inability to acquire required locks or detect changing indexes means refusal/inspect-only. A backup never authorizes permanent deletion.

C15-R5. For worktree removal, prove owning repository/registration, clean tracked state, no untracked/ignored valuable data, no exclusive/unpushed commits, no active processes and recoverability; age is only a filter. Keep inspect-only when a safe provider operation would violate current deletion policy.

C15-R6. Emit a typed installed-tool identity for optional handoff to Software; never delete bin/config folders or implement a second uninstaller. C04/C05 own actual provider execution and C11/C12 own integrated UI acceptance. Expose the AI review as a Clean subview through C11, CLI first.

## Acceptance criteria

- [x] C15-AC1: Claude Code/Codex/Cursor fixture matrices cover default/custom roots, current and superseded versions, active/unknown sessions, locked stores, malformed metadata and unrelated adjacent data.
- [x] C15-AC2: No current version, active session, credentials, memory, settings, skill/MCP definitions, models or project content is selected by generic cache cleanup. The AI planner tests remain. This new test is the generic scan: `generic_clean_scan_does_not_select_protected_ai_paths` via `Sweeper::default().full_scan_report`.
- [x] C15-AC3: No-GUI session recovery/read-back proves content and provider-index consistency; restore refuses external changes. Inject failure before recovery durability and at each bundle/index/move/registration stage: no premature mutation, recoverable or explicitly unknown partial outcomes, no automatic redispatch and no permanently destructive fallback. Locked/changing indexes and unknown active ownership refuse mutation. Bundle failure, move failure, unknown-after-dispatch, and external-edit refusal remain covered by `ac3_bundle_failure_leaves_home_unchanged`, `ac3_failed_move_is_not_retried`, `ac3_disappeared_move_is_unknown_and_not_retried`, and `ac3_restore_refuses_external_edit`. `ac3_unknown_sessions_active` and `ac3_unregistered_worktree` exit 0.
- [x] C15-AC4: Dirty/untracked/ignored/unpushed/submodule/shared/active worktrees remain protected. Native fixture worktree removal/recovery uses a disposable repository only. `ac4_worktree_protections_refuse_deletion_policy` exited 0 on 2026-10-17. Protected classes stay unselected. The clean disposable worktree returns `deletion_policy` before a bundle write or a trash move; `git worktree remove` was not run.
- [x] C15-AC5: Large session inventories remain bounded and cancelable; CLI and direct-service reports/preview digests match and unsupported provider versions explain the refusal. GPUI parity belongs to C11/C14 after this child passes.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
