# Windows Software Boundary

## Decision Summary

Software is a separate versioned domain. It does not extend CleanupPlan,
CleanupIntent, CleanAction, or the path-oriented cleanup authorizer.

## Inventory

- ARP: HKCU/HKLM uninstall roots, enumerated in explicit 32-bit and 64-bit views.
- MSI: machine plus current-user managed/unmanaged product contexts through
  Windows Installer identity APIs.
- MSIX: current-user packages through PackageManager, subject to native unpackaged
  Tauri feasibility evidence.
- WinGet: not an authority; optional later enrichment only.
- WMI `Win32_Product`: prohibited because enumeration can trigger installer
  consistency checks.

Identity is tagged (`Msi`, `Msix`, `Arp`) and never based on display-name,
publisher, icon, or fuzzy path similarity. `NoRemove`, `SystemComponent`, hidden,
framework/resource/dependency/stub, unhealthy, conflicting, incomplete, and
DevSweep-self records are unselectable.

## Execution

- MSI: validated ProductCode/context reconstructed to a fixed Windows Installer
  strategy.
- MSIX: exact PackageFullName reconstructed to current-user PackageManager
  removal.
- ARP-only vendor entries: inventory and Windows Settings handoff only.
- `UninstallString`, `QuietUninstallString`, shell text, PATH lookup, and generic
  tokenization are prohibited.
- No heuristic Win32-leftover deletion. Related data is retained unless a future
  closed rule has exact installer identity evidence.

Every strategy is irreversible from DevSweep's perspective. The executor runs
one installer at a time, persists `started` before dispatch, re-queries installed
state after exit/cancel/timeout, and reports `unknown` when the result cannot be
proven. Raw uninstall strings, environment, username-bearing paths, and command
output are excluded from audit.

## Primary References

- [Alternate registry views](https://learn.microsoft.com/en-us/windows/win32/winprog64/accessing-an-alternate-registry-view)
- [MSI product inventory](https://learn.microsoft.com/en-us/windows/win32/msi/inventory-products-and-patches-)
- [MSI command options](https://learn.microsoft.com/en-us/windows/win32/msi/command-line-options)
- [Current-user MSIX enumeration](https://learn.microsoft.com/en-us/uwp/api/windows.management.deployment.packagemanager.findpackagesforuser)
- [MSIX removal](https://learn.microsoft.com/en-us/uwp/api/windows.management.deployment.packagemanager.removepackageasync)
- [Least-privilege guidance](https://learn.microsoft.com/en-us/windows/win32/secbp/running-with-administrator-privileges)

