# Implement Optimize catalogue, authorizer, execution, and audit

## Goal

Own the closed standard-user maintenance catalogue, fixed DNS refresh, Settings handoffs, preview digests, authorization, locking, outcomes, and audit.

## Requirements

- R1: Define a compile-time versioned catalogue with stable operation id, action
  class, platform preflight, privilege class, fixed action identity, effects,
  risks, cancellation boundary, and audit semantics. Unknown ids fail closed.
- R2: Plan exact ids, re-run preflight, preview identical resolved operations,
  hash a digest, and require confirmation. Use a maintenance-specific authorizer,
  exclusive journal lock, one operation, no automatic retry, and no CleanupPlan.
- R3: DNS refresh resolves the native absolute System32 `ipconfig.exe` and fixed
  `/flushdns`, with separate program/argv, 10-second bound, captured bounded
  output, and no shell. A WOW64 process uses the `Sysnative` alias constructed
  from `GetWindowsDirectoryW`; native processes use `GetSystemDirectoryW`.
  Detection uses `IsWow64Process2`; any API/path/identity failure closes without
  PATH, environment, SysWOW64, or redirection fallback.
- R4: Guidance-only entries never dispatch. Reject security, UAC, firewall,
  Windows Update, registry/service/performance, pagefile/power/hibernation,
  Storage Sense mutation, cache rebuild, script, and user-supplied command paths.
- R5: Cancellation before dispatch is canceled; after dispatch use observed
  success/failure or `unknown_after_dispatch`. Audit all transitions durably with
  stable codes and no localized/system output as identity.
- R6: The only Settings actions are exactly:
  `settings.storage_recommendations` -> `ms-settings:storagerecommendations`
  (build >=22000), `settings.search` -> `ms-settings:search` (build >=22000),
  and `settings.energy_recommendations` ->
  `ms-settings:energyrecommendations` (build >=22624). The first two project
  floors are conservative Windows 11 gates; the energy floor follows Microsoft
  documentation. Build preflight must use a manifest-independent OS build query;
  query failure is unavailable and may not fall back to environment text or a
  manifest-sensitive helper. `ShellExecuteExW` may report only `launched`;
  unsupported build, policy block, or launch failure is a stable non-success
  outcome.

## Acceptance Criteria

- [ ] AC1 (R1, R4, R6): Exhaustive catalogue tests snapshot every allowed id/class/preflight/
      fixed action and OS-build-query failure, and prove unknown/rejected
      categories cannot resolve.
- [ ] AC2 (R2, R6): Dry-run and execute resolve byte-equivalent action identities; stale
      plan, changed preflight, digest mismatch, lock conflict, and unsupported OS
      fail before dispatch.
- [ ] AC3 (R3, R5): Process tests prove exact System32 binary/fixed argv, no shell/path
      search, timeout/output bounds, one child, cancellation/unknown semantics,
      no UAC, and no admin process.
- [ ] AC4 (R4, R5, R6): Settings adapter tests prove exact allowlist and launched-only outcome;
      guidance cannot enter executor. Audit durability/redaction, native scenarios,
      focused tests, and `just ci` pass.

## Out of Scope

- Any operation outside the closed research catalogue, elevation, scripting,
  shell execution, automatic retry, or claiming Settings actions completed.
