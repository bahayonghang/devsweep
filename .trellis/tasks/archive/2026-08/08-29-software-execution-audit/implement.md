# Implement - Software Execution and Durable Audit

Start only after later approval and the Software inventory/preview contract,
including the exact WinRT binding, passes.

## 1. Add non-deserializable strategies and audit state machine

1. Add `ValidatedSoftwareAction` constructors reachable only from live preview
   revalidation and exact eligible current-user MSIX identities; prove every MSI
   context and registry-only identity has no constructor.
2. Add a locked versioned audit with durable pre-side-effect
   `dispatch_started`, monotonic transitions, one closed terminal, durable flush,
   stable codes, and redaction.
3. Implement startup recovery of nonterminal `dispatch_started` records by
   identity-only requery with no adapter retry.
4. Test all-MSI/manual, machine/other-user MSIX, registry/protected/stale/
   malformed identities and digest/lock failures before any adapter call.

Focused validation:

```powershell
rtk cargo test -p devsweep-core software::execution
rtk cargo test -p devsweep-core software::audit
```

Rollback point: disable/remove execution constructors while retaining readable
audits and inventory; never downgrade an unknown audit version.

## 2. Implement the exact MSIX adapter and terminal table

1. MSIX: send exact `PackageFullName` to the inventory task's dedicated MTA
   worker and call `PackageManager::RemovePackageAsync`.
2. Serialize one uninstall, bound monitoring to 120 seconds, treat cancellation
   before dispatch as canceled, and after dispatch request OS cancellation at
   most once, wait five seconds, then requery at 0/2/10 seconds.
3. Apply the first-match evidence table exactly and classify only removed,
   reboot-required, still-present, failed, or unknown-after-dispatch; never call
   any result reversible and never emit execution `partial`.
4. Add crash points before/after durable `dispatch_started`, restart recovery,
   evidence-conflict, adapter-unfinished, absent+reboot, and present+failure/
   success fixtures; prove recovery performs no second removal call.

Focused validation:

```powershell
rtk cargo test -p devsweep-core software::execution::msix
```

Rollback point: remove the two adapters and leave every Software entry
unselectable/manual; do not substitute vendor commands or elevation.

## 3. Frozen CLI, full, and native gates

Implement only `software preview` and `software uninstall` in the owned handler.

```powershell
rtk cargo test -p devsweep-cli software
rtk git diff --check
rtk just ci
```

Native disposable current-user MSIX scenarios record process/integrity tree,
no UAC, success, reboot, cancel before/after dispatch, timeout, crash/restart,
requery unavailable, audit durability, and irreversible confirmation text. The
repository contains no disposable MSIX fixture: before any uninstall, display
the exact tagged current-user package identity and pause for the user to confirm
that concrete target. Do not install or sign a fixture. If no disposable target
is supplied, keep the native uninstall evidence `UNVERIFIED` and pause. Stop for
any MSI/machine/other-user execution, any rollback/reinstall promise, new
dependency, unbounded wait, or frozen CLI change.
