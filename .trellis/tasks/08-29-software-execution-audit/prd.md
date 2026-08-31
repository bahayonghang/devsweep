# Implement Software execution and durable audit

## Goal

Own current-user MSIX-only strategy reconstruction, revalidation, single-flight dispatch, cancellation semantics, deterministic post-state/restart recovery, the closed execution terminals, and audit. Every MSI remains inventory/manual in V1.

## Requirements

- R1: Accept only a live-revalidated Software preview plus matching digest and
  explicit confirmation. Reconstruct a strategy only for an eligible exact
  current-user MSIX `PackageFullName`; reject every MSI context, registry-only,
  or other identity before strategy construction. Never consume serialized or
  vendor argv.
- R2: Run one uninstall at a time under `asInvoker`, with an exclusive audit lock,
  pre-dispatch cancellation, bounded monitoring, exit/reboot interpretation, and
  post-dispatch installed-state requery. Never request UAC or fall back to admin.
- R3: After dispatch, cancellation/timeout/crash is not reported canceled unless
  requery proves the terminal state. Report exactly `removed`,
  `reboot_required`, `still_present`, `failed`, or
  `unknown_after_dispatch` with the evidence precedence in `design.md`;
  Software execution has no `partial` terminal. Source/inventory partial remains
  upstream evidence only.
- R4: Write a durable redacted versioned audit before/after each transition.
  Audit contains tagged identity and outcome, not command strings, plan secrets,
  guessed leftovers, or localized text.
- R5: Every uninstall is irreversible from DevSweep's perspective. Preview and
  confirmation state this explicitly; no rollback/reinstall promise exists.
  Cancel/timeout after dispatch is resolved by authoritative requery or reported
  `unknown_after_dispatch`, never as restored or safely canceled.

## Acceptance Criteria

- [ ] AC1 (R1, R2): Strategy tests prove exact current-user identity reconstruction and
      reject all MSI contexts, machine/other-user MSIX, registry-only, protected,
      stale, malformed, unsupported, and digest-mismatched inputs before dispatch.
- [ ] AC2 (R2, R3, R5): Fake adapter/integration tests cover success, still present, reboot,
      failure, timeout, cancel before/after dispatch, crash between durable
      `dispatch_started` and adapter completion, restart requery, audit-lock
      conflict, evidence conflicts, and requery unavailable/unknown. Fixtures
      prove the five-terminal table exactly and contain no execution `partial`.
- [ ] AC3 (R1, R2): Process/API evidence proves program and argv remain separate where a
      process is used, no shell/vendor text, one operation, no UAC/elevated child,
      and no orphan process owned by DevSweep.
- [ ] AC4 (R3, R4, R5): Audit durability/redaction/version tests, native disposable-package
      evidence, focused tests, and `just ci` pass.

## Out of Scope

- Any MSI uninstall, machine/other-user MSIX uninstall, vendor uninstall strings, force-kill, rollback/reinstall,
  leftover cleanup, batch parallelism, or administrator helper.
