# Implementation Plan: Central Safety Policy and Live Revalidation

## Preconditions

- `ValidatedAction` and the v2 footprint field are available from
  `07-28-plan-validation`.
- `ProcessRunner` is available for bounded cargo metadata calls from
  `07-28-process-runner-cancellation`.
- Preserve existing default dry-run behavior and route both command and trash
  mutations through the same authorization boundary.

## Ordered work

1. Add fail-red authorization fixtures for every protection category, allowed
   legal descendant, failed current-exe lookup, root escape, marker loss,
   renamed/recreated target, and ancestor link/reparse replacement.
2. Add platform file-identity/footprint capture and comparison at scan and live
   execution boundaries, plus structured `Denial` reasons.
3. Implement `SafetyPolicy`, opaque `AuthorizedAction`, protection matching,
   and default-deny authorized-footprint proof. Update executor interfaces so
   direct runner/trash access is impossible outside the authorization funnel.
4. Implement OS app-data policy resolution, versioned JSON load/store, atomic
   writes, and `protect add|remove|list` CLI commands with tests for canonical
   add, missing-path remove, corrupt config, and failed write.
5. Add live ancestor/link/reparse, marker, containment, self-exe, and audit-log
   revalidation before every side effect.
6. Migrate Rust target discovery to bounded `cargo metadata`; bind display,
   estimate, guard, action, and live recheck to one resolved target scope.
7. Add custom target-dir, workspace, environment override, and metadata-failure
   fixtures; surface safe downgrade/inspect-only diagnostics in JSON and TUI.
8. Search all production command/trash call sites to prove they receive an
   `AuthorizedAction`, run platform link fixtures, then run `just ci`.

## Verification matrix

| Scenario | Required evidence |
| --- | --- |
| legal descendant | `~/.gradle/caches`, project `node_modules`, and nested dependency `.git` authorize |
| protected path | home, scan root, volume root, `.ssh`, own VCS metadata, and user-protected path deny with correct category |
| TOCTOU | rename/recreate, marker removal, root escape, and ancestor link/reparse changes deny before runner/trash call |
| identity lookup failure | injected current-exe or file-identity failure denies, never permits |
| protection config | add/list/remove persist canonical entries; corrupt/unwritable config fails closed |
| Cargo scope | custom target dir, workspace target, and `CARGO_TARGET_DIR` align display/estimate/guard/action; changed metadata denies |
| funnel completeness | source-level call-site review and integration tests show no direct mutation bypasses `authorize` |

Run dynamic symlink/reparse/junction tests on Windows, Linux, and macOS. A
platform without permission to create a required link fixture is a documented
no-go for that platform, not a silent green. Run `cargo test --all-targets`,
the focused protect CLI commands, and `just ci`.

## Review and rollback points

- Review authorized-footprint logic against each protection class so legal
  global/project descendants are not overblocked.
- Review policy file error paths for fail-closed behavior and no temporary
  in-memory fallback.
- Review all `Command`/trash call sites after migration, including test helpers.
- Do not activate until upstream validation/runner interfaces and link-fixture
  evidence are available.
