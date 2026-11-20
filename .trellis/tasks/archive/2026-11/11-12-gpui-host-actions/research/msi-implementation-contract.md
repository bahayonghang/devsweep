# MSI implementation contract (steps 3–4)

Approved scope is `step-2-review.md`; the original absolute no-UAC and keep-awake
receipt proposals are not requirements. This implementation changes only the
core Software removal path and CLI Software adapter. GPUI MSI activation remains
step 11. Existing MSIX and fixture strategies keep their historical semantics.

## Positive eligibility and live proof

A production MSI selection requires all three current-user/machine enumeration
contexts to complete, exactly one registration for its strict braced GUID, and
that registration to be current-token-SID `UserUnmanaged`, installed (`ProductState
5`). `PackageCode` and `VersionString` must be available. Its exact current-user
GUID uninstall registry key must be readable in the applicable registry view,
identify an MSI registration (`WindowsInstaller=1`), and have successfully queried
NoRemove/SystemComponent/ParentKeyName/ReleaseType restrictions. An absent
optional restriction is an observed absence, unlike an API/type/access failure.
NoRemove, hidden/system/update/parent entries are refused. Applicable Installer
AlwaysInstallElevated policy is also inspected; enabled or unreadable policy is
refused. Vendor uninstall fields are never queried.

The API-provided `LocalPackage` is opened read-only with no-reparse proof and a
bounded hash. Its package-code summary must match the registered PackageCode,
and Word Count Summary bit 3 must positively declare that elevated privileges
are not required. Missing/unknown declaration remains manual. This is package
authoring evidence, not a sandbox or a promise about arbitrary custom actions.
Sources: [Product info](https://learn.microsoft.com/en-us/windows/win32/api/msi/nf-msi-msigetproductinfoexw),
[Word Count Summary](https://learn.microsoft.com/en-us/windows/win32/msi/word-count-summary),
[summary API](https://learn.microsoft.com/en-us/windows/win32/api/msiquery/nf-msiquery-msigetsummaryinformationw),
[uninstall registration](https://learn.microsoft.com/en-us/windows/win32/msi/uninstall-registry-key).

New optional inventory evidence retains SID/package code and opaque registration
and system-tool digests. It does not change the exact identity/selection ID.
Old inventories omit it and keep their fingerprints. Deserialized evidence is
never live authority: execution independently re-acquires it, then rebinds it
immediately before the process boundary. Changed registration/restrictions,
package, SID or executable invalidate the preview. Missing new evidence cannot
become a production MSI strategy through existing fixture eligibility.

The SDK's `INSTALLPROPERTY_PRODUCTSTATE` string is **`State`**, not
`ProductState` (`C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\um\Msi.h:408`).
Exact-state querying treats only `ERROR_UNKNOWN_PRODUCT` as absence;
`ERROR_UNKNOWN_PROPERTY`, corrupt state and permission failures are unavailable.

## Process and result evidence

Resolve `msiexec.exe` through the Windows system-directory API; never PATH,
WINDIR, registry text or caller argv. Canonicalize the physical file used by
both hashing and process creation, retain read-only file/ancestor handles during
dispatch, and compare identity/content with the live preview. Do not toggle
WOW64 redirection globally. Fixed separate argv is `/x`, GUID, `/qn`, `/norestart`.
Use bounded ProcessRunner monitoring and cancellation; no shell/elevation/UI
fallback. Refused live bindings produce zero process calls.

Retain the system program's immutable leaf/ancestor bindings through execution.
The cached package's binding is held only through the final live proof and
comparison, then released immediately before dispatch so Installer can maintain
its own LocalPackage. Registration plus an external Installer operation is not
an atomic transaction; authority is rechecked immediately before dispatch.

Optional V1 MSI execution evidence contains the exact SID and typed numeric
process result/interruption. Numeric `1641` distinguishes already-initiated
restart from `3010` restart-required without changing the legacy reboot enum.
Only `0 + authoritative same-SID/context absence` becomes Removed. `0 + present`
is StillPresent; unavailable/conflicting state is UnknownAfterDispatch. `3010`
and `1641` are RebootRequired, without leftover authority. All other numeric
codes are Failed even if absent; interruption/lost status is UnknownAfterDispatch.
Recovery never redispatches. The journal validates the same MSI evidence matrix
and monotonic accumulated evidence, while missing-MSI-evidence legacy records
retain their old interpretation. New MSI requery is typed identity/SID/context;
no GUID goes into the MSIX string-only query.

Legacy MSI records remain readable but **do not gain new leftover authority**.
That gate additionally requires valid MSI SID, exit code zero, no interruption,
and exact observed absence in the accumulated operation state.

CLI grammar remains plan/preview/uninstall with mandatory digest and confirm.
Human presentation is bilingual; JSON and numeric evidence are locale-neutral.
Core tests use private injected registration/tool/process boundaries through the
real plan/live-preview/confirmation/executor/journal chain. CLI tests inject only
handler dependencies, never a production environment-variable authority bypass.
Native Installer behavior remains unverified; no host mutation or Computer Use.
