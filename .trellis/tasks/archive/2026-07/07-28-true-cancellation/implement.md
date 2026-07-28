# Implementation Plan: Real Cancellation and Worker Exit

## Preconditions

- Consume race-hardening terminal state transitions.
- Consume ProcessRunner typed `Canceled` and process-tree termination.
- Consume scan-reliability partial outcomes and walker checkpoints.

## Ordered work

1. Add token tests and fail-red fake workers for a blocked Nth action, scan,
   sizing, and provider cancellation.
2. Define the shared token/observer plus no-op adapter.
3. Thread the observer through scanner, size, sweep, providers, executor, and
   ProcessRunner request paths.
4. Prevent later executor actions after cancellation and report partial work.
5. Retain runtime token and join handle. Replace fabricated cancel events with
   actual worker terminal confirmation.
6. Add D9 quit overlay choices and require join before terminal exit.
7. Add late-event, latency, and child/grandchild cleanup tests. Run `just ci`.

## Verification matrix

| Scenario | Required evidence |
| --- | --- |
| blocked Nth action | request cancel; Nth finishes safely; N+1 runner calls equal zero |
| scan/size | checkpoint exits within measured target and produces partial result |
| provider command | token reaches ProcessRunner and tree terminates |
| terminal state | no late event revives a confirmed canceled job |
| quit continue | overlay closes and worker continues unchanged |
| quit cancel | worker joins and no child/grandchild remains before terminal exit |
| report | completed and unstarted work are represented truthfully |

Measure walker response against the 250 ms target and target-boundary response
against the 100 ms target. Run Windows Job Object, Linux process-group, and
macOS dynamic cancellation evidence. Missing process-tree proof is a no-go for
that platform. Run `cargo test --all-targets` and `just ci`.

## Review points

- Review every long-running loop and action boundary for token checks.
- Review registry cleanup so joins and permits cannot leak or double-release.
- Review quit handling to prove mutation workers never detach silently.
- Do not change ProcessRunner tree mechanics in this child.
