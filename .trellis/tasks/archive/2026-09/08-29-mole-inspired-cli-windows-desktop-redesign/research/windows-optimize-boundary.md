# Windows Optimize Boundary

## Decision Summary

Optimize is a separate, closed Windows maintenance domain. It may reuse the
bounded process runner and digest/audit patterns, but not CleanupPlan or the
path-oriented cleanup authorizer.

## Non-Elevated Catalogue

| Operation | Disposition |
| --- | --- |
| DNS resolver cache refresh | Real execution: absolute System32 `ipconfig.exe`, fixed `/flushdns`, 10 s timeout |
| Storage recommendations | Allowlisted Windows Settings handoff |
| Search indexing | Allowlisted Windows Settings handoff |
| Energy recommendations | Windows 11 conditional Settings handoff |
| Drive optimization, DISM/SFC, CHKDSK, network reset | Read-only guidance; no execution |

Storage Sense configuration, search-index rebuild, Explorer/icon-cache rebuild,
service/registry/performance tweaks, security settings, Windows Update resets,
power/pagefile/hibernation changes, arbitrary commands, scripts, and remote
management are rejected.

Each operation has a catalog version/stable ID, exact preflight, dry-run/execute
equivalence, preview digest, privilege class, fixed action identity, durable
locked journal, no automatic retry, and terminal states including
`canceled_before_start` and `unknown_after_dispatch`.

## Primary References

- [ipconfig `/flushdns`](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/ipconfig)
- [Windows Settings URIs](https://learn.microsoft.com/en-us/windows/apps/develop/launch/launch-settings)
- [Storage Sense](https://learn.microsoft.com/en-us/windows/configuration/storage/storage-sense)
- [Drive optimization](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/defrag)
- [CHKDSK](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/chkdsk)
- [UAC and least privilege](https://learn.microsoft.com/en-us/windows/security/application-security/application-control/user-account-control/how-it-works)

