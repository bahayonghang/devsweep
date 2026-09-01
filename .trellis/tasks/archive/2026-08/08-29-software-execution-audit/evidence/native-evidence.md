# Software execution native evidence

## Verified native non-uninstall evidence

- The actual `target/debug/devsweep.exe software preview` handler ran under a
  medium-integrity token (`S-1-16-8192`) with no new `consent.exe` process.
- The process tree contained only `devsweep.exe` and its Windows console host.
  The observed console-host PID was absent after the root process exited, so no
  product-owned process remained orphaned.
- The command performed current-user MSIX read-only revalidation and stopped at
  the expected locale-neutral stale-authority result: exit `3`, machine outcome
  `failed`, error code `invalid_software_authority`, and zero stderr bytes.
- Native Windows core tests verified manual MSI refusal, registry/machine/
  protected/stale/malformed identity refusal, digest and audit-lock failures
  before adapter calls, pre-dispatch cancellation, exact five-state terminal
  classification, crash recovery without redispatch, redaction, one terminal,
  unknown-version refusal, and durable journal reopen after `sync_all`.
- No MSI, machine, other-user, vendor, shell, elevation, install, signing, or
  package-removal command was run while collecting this evidence.

Raw observations are in `native-non-uninstall.log`; the stale plan used only a
synthetic nonexistent opaque id and is retained as
`native-preview-stale-plan.json`.

## UNVERIFIED - real-uninstall native scenarios only

The repository has no disposable current-user MSIX fixture. Per the approved
task boundary, no installed package was selected or removed and no fixture was
installed or signed. The following native scenarios therefore remain
`UNVERIFIED` until the user explicitly confirms one concrete tagged
current-user `PackageFullName` as disposable:

- real `RemovePackageAsync` success followed by exact-identity absence;
- real adapter reboot-required evidence with absent/present requery;
- real pre-dispatch and post-dispatch cancellation around that target;
- real 120-second timeout plus one OS cancel request and five-second grace;
- real crash/restart reconciliation after durable `dispatch_started`;
- real exact-identity requery denial/unavailability after dispatch;
- real definitive adapter failure or successful completion while the exact
  identity remains present.

No rollback, reinstall, restoration, or reversibility claim is made for any of
these scenarios.
