# C: Project artifact capacity advisories

## Goal

Show actionable, confidence-aware project warnings at the user-approved adjustable 5 GiB threshold after explicit scans.

## Parent and dependencies

Parent: [cleanup-home-redesign](../10-14-cleanup-home-redesign/prd.md).
A's observation/compatibility contract; B's owner/footprint contract before mbx acceptance. Non-mbx pure fixture work can begin once A is stable.

## Requirements

C1. Default T=5,368,709,120 bytes, inclusive. Aggregate owner-attributed build/test/tool-cache footprints only; show dependencies/venv separately, outside this subtotal.
C2. Exclude global/shared mbx storage; count overlapping aliases once; distinguish exact values, known lower bounds and unknown.
C3. Warn when a complete value or partial lower bound reaches T. A partial value below T or unknown cannot claim below threshold.
C4. Persist a positive whole-GiB setting with overflow-safe byte conversion, default-on-missing without write, and preserve corrupt/future/unknown data.
C5. Evaluate completed user-requested reports; setting changes can re-evaluate a snapshot without I/O. No timers, background scans, OS notifications, selection changes or cleanup.

## Acceptance criteria

- [x] T-1, T and T+1; several small outputs crossing T; multiple projects; nested/aliased/shared storage all produce correct totals and owner association.
- [x] Partial >=T yields at-least warning; partial <T and unknown yield incomplete assessment; missing global/provider values never become zero.
- [x] Changing threshold updates only advisories. The selected IDs, saved plan, preview digest and confirmation remain unchanged.
- [x] Read/write/validation tests cover default, invalid values, overflow, atomic write failure, concurrent edits, corrupt/future bytes and session-only recovery.
- [x] CLI/core/Tauri calculations tested over the same fixtures agree where exposed, without changing existing CLI report shape unnecessarily.

## Scope boundaries

Implementation authorized on 2026-10-14. Preserve the pre-existing desktop/src-tauri/Cargo.toml edit. No actual user-cache cleanup, unrelated modes, commits, archive or publication. Parent scope exclusions apply.

## Decision status

User product decisions and selected design are in the parent PRD. No additional product question blocks this plan; implementation depends on the stated upstream contracts.

