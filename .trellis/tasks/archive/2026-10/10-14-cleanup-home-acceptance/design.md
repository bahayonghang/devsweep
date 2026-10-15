# F: technical design

## Design and evidence

- Consume the parent-owned read-only preflight manifest captured before any child product edits. Verify its source/binary and initial-diff identity before final comparisons; never manufacture a late baseline from changed code. Historical C3/C4 notes only guide methodology, never supply PASS.
- Driver asserts checked scope/root controls and submitted IPC before each scan; a wrong-scope run is invalid and must cancel/join before retry.
- Record host, OS/WebView/runtime versions, binary/source identity, warm-up/sample counts, fixture manifest and process creation identity.
- Compare baseline and final geometry at matching viewport/data. DPR emulation is distinct from actual OS scaling, browser fixtures from native UI.
- Use owned simulated executors and sentinel fixtures for destructive-path correctness; never invoke real gc/clean/uninstall to get screenshots.
- Read the final plan/implementation against scope exclusions; no unrelated product changes or delivery actions.

## Owned boundary

Existing integration tests and owned native/fixture evidence drivers where needed; task research/validation artifacts. No unrelated refactoring.

## Compatibility

Preserve the parent design's saved-report/plan, appearance and explicit-root contracts. New wire fields require generated Rust/TypeScript/decoder parity and old/new fixtures. No UI-derived cleanup authority.

## Dependencies and handoff

A/B/C/D/E implemented and individually checked. Parent completion requires this child and no unfulfilled mbx executable-cleanup requirement.

## Rollback

Block parent acceptance and route defects to their owning child. Do not hide failure behind reduced samples, changed thresholds or disabled tests.

