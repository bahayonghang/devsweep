# Implementation Plan: Partial Scan and Size Reliability

## Preconditions

- Agree the v2 partial-diagnostic and estimate serialization shape with
  `07-28-plan-validation`; this child must not modify `CleanupPlan` serde on
  its own.
- Preserve root access failures as hard errors and preserve existing
  symlink/reparse protections.
- Keep traversal budgets, prune rules, and ranking-semantics changes out of
  this child.

## Ordered work

1. Add fail-red fixtures for an unreadable nested child, unreadable root, and
   a size traversal that fails after observing some entries. Prove the fixture
   actually denies access before testing outcome behavior.
2. Introduce the internal diagnostic/completeness types and update recursive
   scanner calls to downgrade nested access errors while propagating root
   errors with context.
3. Introduce `SizeEstimate`, replace error-to-zero fallbacks, and accumulate
   lower bounds plus warnings through the size walker.
4. Thread outcomes through sweep, CLI, TUI worker, app state, and render
   helpers. Preserve explicit selection while marking incomplete targets as not
   selected by default.
5. Integrate the agreed v2 plan schema extension through the plan-validation
   owner. Add JSON round-trip and no-diagnostic pollution tests.
6. Correct the global-only textual summary so it reports global/provider scan
   scope rather than unscanned roots.
7. Add a no-op cancellation check at safe traversal boundaries without defining
   the token implementation.
8. Run focused scanner/fs-size/sweep/ranking/TUI/CLI tests, platform permission
   tests, `scan --global` manual output, then `just ci`.

## Verification matrix

| Behavior | Required evidence |
| --- | --- |
| inaccessible child | siblings remain in plan, outcome is partial, one diagnostic names the child stage/path |
| inaccessible root | command returns non-zero hard error; no empty/partial plan is substituted |
| failed size traversal | partial lower bound or unknown estimate, never trusted `0 B` |
| empty directory | verified `Some(0), complete=true` remains visibly distinct from failure |
| default selection | incomplete/unknown target is unselected without preventing explicit later user selection |
| JSON/TUI parity | both expose partial state using the plan-validation-owned v2 fields |
| global summary | `scan --global` has no invented project-root count |

Run focused tests on Windows, Linux, and macOS CI. Permission fixtures that do
not produce a real access failure are a no-go, not a skip-with-green. Run
`cargo test --all-targets` and `just ci` after integration.

## Review and rollback points

- Review every former `(0, ...)` error path to ensure only a real empty tree
  yields a trusted zero.
- Keep diagnostics structured until the final CLI/TUI presentation boundary.
- Do not add process execution or mutation to scanning/size code.
- Before activation, verify plan-validation has accepted the wire contract or
  an explicit written interface record exists in both task designs.
