# C11 Native cleanup workbench

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C03](../10-15-gpui-clean-coverage/prd.md), [C10](../10-15-gpui-shell-assets/prd.md), [C15](../10-15-gpui-ai-tool-cleanup/prd.md).

## Goal

Build the GPUI review-and-clean workflow over accepted CLI/core capabilities.

## Requirements

C11-R1. Provide independent global-cache/project-artifact scans, first-use root setup, retained reports, coverage diagnostics and actionable progress.

C11-R2. Present provider groups/project ownership, occupancy/cost/confidence, search/sort, bounded rows and full-report selection.

C11-R3. Use a staged review: selection, live preview, explicit confirmation, execution, truthful result/history; invalidate on all relevant changes.

C11-R4. Expose protection/ignore actions and rebuild-cost explanation without hiding partial or inspect-only items.

C11-R5. Include AI-tool review from C15 with separate cache/session/worktree categories, explicit sensitive-data selection and recovery status.

## Acceptance criteria

- [ ] C11-AC1: The same isolated inputs and selection produce equal CLI/GPUI preview digests and outcome semantics, including C15 AI cleanup/recovery contracts and fixture identities.
- [ ] C11-AC2: 10k-item search/select/scroll and cancel stay responsive with bounded rows; selections survive navigation and do not miss offscreen rows.
- [ ] C11-AC3: Unavailable, empty, partial, failed, canceled, stale preview, mixed execution and Recycle Bin results have usable native UI states.
- [ ] C11-AC4: Keyboard and UIA can complete the review flow; no automatic broad machine scan runs on first launch.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
