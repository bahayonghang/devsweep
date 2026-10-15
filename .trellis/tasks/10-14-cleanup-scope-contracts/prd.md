# A: Scope and global cleanup contracts

## Goal

Expose independent global/project scans while retaining one trusted cleanup implementation and making project cleanup opt-in.

## Parent and dependencies

Parent: [cleanup-home-redesign](../10-14-cleanup-home-redesign/prd.md).
None. Parent planning package must be reviewed before implementation.

## Requirements

A1. Global-only discovery works with no roots; project-only discovery requires explicit validated roots. Keep CLI --scope projects|global|all and its existing default behavior.
A2. Reuse uv and the current global provider catalogue. Model availability, capability, coverage and sizing separately; do not add an unnecessary new uv implementation or swap clean for prune.
A3. Set project selected_by_default=false through the shared policy. Keep explicit CLI selection and global provider risk rules intact; update TUI/native consumption tests.
A4. Keep saved ScanReport V1 and CleanupPlan V2 compatibility. Advisory/overview fields must not be silently added to deny_unknown_fields documents.
A5. Never add direct deletion or shell command assembly; use current saved plan, live preview digest and confirmation.

## Acceptance criteria

- [x] Empty-root global request succeeds; project validation remains fail-closed; every submitted scope has exact expected flags.
- [x] Existing CLI parser/help/plan compatibility and global provider authority tests pass for supported/missing/unsupported/failed providers.
- [x] All project targets are initially unselected, including old Rust outputs; inspect-only remains impossible to select.
- [x] CLI/Tauri use the same rule IDs and cleanup semantics; old observations/plans and current JSON decoders have explicit tested compatibility.

## Scope boundaries

Implementation authorized on 2026-10-14. Preserve the pre-existing desktop/src-tauri/Cargo.toml edit. No actual user-cache cleanup, unrelated modes, commits, archive or publication. Parent scope exclusions apply.

## Decision status

User product decisions and selected design are in the parent PRD. No additional product question blocks this plan; implementation depends on the stated upstream contracts.

