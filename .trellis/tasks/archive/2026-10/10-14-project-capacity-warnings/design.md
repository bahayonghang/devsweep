# C: technical design

## Design and evidence

- Use a pure core summary/advisory projection over backend ownership data, not JS path parsing or reused cleanup-risk/sizing-error tags.
- Expose a separate versioned read-only advisory DTO via Tauri if needed. Do not append arbitrary fields to closed ScanReport V1/health documents.
- Use a focused core-owned clean-advisories-v1.json setting beside existing settings; leave roots V1 and appearance V2 untouched.
- Return assessment, confidence, measured logical bytes, included kinds, threshold and report identity. These values are observations, never cleanup authority.
- Logical footprint is not physical reclaim; hardlink/shared allocation uncertainty must remain visible.

## Owned boundary

New focused core advisory model/projection and settings module, existing atomic persistence helpers, Tauri advisory/settings commands/DTO generation/decoders. D owns settings UI and bilingual warning copy.

## Compatibility

Preserve the parent design's saved-report/plan, appearance and explicit-root contracts. New wire fields require generated Rust/TypeScript/decoder parity and old/new fixtures. No UI-derived cleanup authority.

## Dependencies and handoff

A's observation/compatibility contract; B's owner/footprint contract before mbx acceptance. Non-mbx pure fixture work can begin once A is stable.

## Rollback

Remove advisory presentation/commands without altering reports, plans, root stores or cached artifacts; preserve unknown settings bytes.

