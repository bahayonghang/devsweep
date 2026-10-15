# B: technical design

## Design and evidence

- Research baseline: installed mbx 1.24.0; cache dir points to shared actions, while clean <WORKSPACE> targets managed/private state. cache projects has no advertised JSON; internal sessions are not public API.
- Start with a bounded interface qualification spike using published docs and isolated fixtures. Do not invent flags or make human-readable table output cleanup authority.
- Keep generic Cargo symlink/reparse rejection. A dedicated mbx adapter must establish every link/root/workspace boundary through a closed reviewed contract.
- Exclude cache remove, gc, --under and manual root deletion. Shared global action-store cleanup is not part of the workspace action.
- Qualification must enumerate and bind the link entry, destination contents, private state and affected metadata, and disclose structural link removal in preview/confirmation. A version-matched source/public-contract audit must substantiate actual behavior beyond the mock. If insufficient, request authorization for a fully isolated disposable-workspace conformance run; do not use user cache storage. Test protected/changed links, shared-store preservation, other-workspace sentinels, busy partial outcomes and post-command disposition.
- Emit non-authoritative owner/footprint summaries for C. Exclude ambiguous overlap from exact totals and show why.
- Existing process-name-only Node busy detection is conservative. Improving it requires separate process-to-project evidence inside this task; do not casually relax it.

## Owned boundary

Core scan/project, dedicated mbx adapter if proven, rules registry/definitions/configuration, live execution/provider/safety, filesystem identity, model/DTO parity fixtures.

## Compatibility

Preserve the parent design's saved-report/plan, appearance and explicit-root contracts. New wire fields require generated Rust/TypeScript/decoder parity and old/new fixtures. No UI-derived cleanup authority.

## Dependencies and handoff

A's scope/compatibility policy. Publish the ownership/overlap contract before C's final aggregation and D's final integration.

## Rollback

Disable/revert the new executable provider while preserving truthful inspection; never weaken generic no-follow or expand a command to make acceptance pass.

