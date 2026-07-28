# Implementation Plan: TUI Race Hardening

## Preconditions

- Keep `CleanupPlan` serialization unchanged.
- Do not add a canonical digest or external-plan fingerprint here.
- Record the runner and cancellation hand-off points so
  `07-28-true-cancellation` can replace the minimal dispatch gate without
  changing the app-state contract.

## Ordered work

1. Add fail-red app tests for confirmation snapshot execution, scan-triggered
   invalidation, and zero emitted `StartClean` effects after invalid Enter.
2. Introduce the private `ExecutionManifest` and extend `ConfirmState` with
   invalidation state. Build the manifest at modal open and make confirmation
   consume only that snapshot.
3. Route scan events through confirmation invalidation before scan-state
   mutation; render the disabled/re-confirmation state and log rejected Enter.
4. Add explicit selection overrides and update staged scan merging so existing
   target choices survive while new targets receive default selection.
5. Add fail-red transition tests for every late event after `Cancelling` and
   every terminal state. Centralize job transition validation, remove fabricated
   cancellation completion, and make the interim cancellation wording honest.
6. Add app-level rejection tests for `c` and `s` during a mutation job. Add a
   runtime-owned clean-dispatch gate plus a blocking service test proving a
   duplicate `StartClean` creates no second worker.
7. Add scanner fixtures for duplicate roots, overlapping roots, exact matching
   candidates, and equal paths with distinct actions. Implement evidence merge
   and action-aware coverage reduction without exporting a second fingerprint
   abstraction.
8. Run focused TUI/scanner tests, inspect all state transitions and emitted
   effects, then run the canonical `just ci` gate.

## Verification matrix

| Defect | Fail-red evidence | Passing evidence |
| --- | --- | --- |
| F-02 confirmation drift | open modal, inject scan update, press Enter | no clean effect or runner call; re-confirmation message is logged |
| F-07 app single-flight | active clean, press `c` then `s` | no second clean/scan effect and rejection is logged |
| F-07 runtime single-flight | blocking fake clean service, dispatch two clean effects | exactly one worker/service call starts |
| F-07 duplicate footprint | duplicate and overlapping root fixtures | each equal footprint/action appears once with merged evidence |
| F-03 terminal revival | inject late progress/finish/cancel events after each terminal status | terminal status remains unchanged and event is only logged |
| F-20 selection reset | deselect an initially selected target, inject staged scan update | matching target remains deselected; new target gets default selection |

Run `cargo test tui`, focused scanner tests, `cargo test --all-targets`, and
`just ci`. On Windows, run the case/separator duplicate fixture. The task is
not complete without Windows evidence for that platform-specific normalization;
CI must retain a Windows row for the focused suite before this child is
archived.

## Risk and rollback points

- Do not let the runtime gate become a parallel cancellation registry; stop
  after the minimal permit contract and hand control to true cancellation.
- Do not merge candidates that differ in action identity or rule semantics.
- If an event transition breaks normal completion, revert only the transition
  helper change and retain its fail-red tests until the contract is corrected.
- Before activation, verify that all manifest entries below resolve and no
  `_example` rows remain.
