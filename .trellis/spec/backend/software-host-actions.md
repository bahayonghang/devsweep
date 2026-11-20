# Software host actions

## 1. Scope / Trigger

Apply when changing production MSI removal, its saved-plan/live authority,
winget update execution, Software support audit, or GPUI host review. Core owns
execution; CLI and GPUI consume it in process. Vendor uninstall text, display
metadata, fixture eligibility and a serialized executable are never authority.

The approved first MSI scope is current-user unmanaged registration with complete
positive evidence. Machine, managed, other-user, ambiguous, known-elevated and
incompletely observed candidates remain manual. DevSweep never requests elevation
or falls back to interactive Installer UI. This is not a sandbox guarantee about
arbitrary third-party custom actions.

## 2. Signatures and payloads

The existing Software command surface remains:

```text
software plan
software preview --plan PLAN
software uninstall --plan PLAN --preview-digest sha256:... --confirm
software updates plan / preview / execute
software startup plan / preview / execute
```

The existing `execute_software_update(program, plan, digest, confirmed)` facade
keeps its signature and delegates with a no-op cancel observer. The additive
`execute_software_update_cancellable` entry receives the GUI job observer for
pre-target checks; it does not change CLI grammar or persisted DTOs.

`SoftwareExecutionRequest` retains `plan`, `expected_preview_digest`, `confirmed`
and `cancel`; it does not accept a caller-supplied live inventory. New optional
V1 data is:

```text
SoftwareEntryV1.msi_registration:
  user_sid, package_code, registration_digest, tool_digest
SoftwareAuditRecordV1.msi / SoftwareActionOutcomeV1.msi:
  user_sid, process?: { exit_code?: u32, interruption?:
    timed_out | canceled | status_unavailable }
```

Missing optional evidence preserves legacy decoding and fingerprints. Exact
identity/selection IDs and old saved-plan shapes remain unchanged. Live authority
must independently reacquire the evidence; a forged optional object cannot confer
execution permission.

## 3. Authority and execution contracts

- MSI enumeration must complete the relevant contexts, yield one applicable
  strict braced GUID, and bind the exact current token SID, `UserUnmanaged`
  context and installed state. The native property constant
  `INSTALLPROPERTY_PRODUCTSTATE` has the string value `State`; an unknown
  property is not an absent product. Only a typed authoritative product-not-found
  result may become absence.
- Require `PackageCode`, version and read-only cached `LocalPackage` evidence.
  Its summary package code must match and Word Count bit 3 must declare that
  elevated privileges are unnecessary. Retain no-reparse file/ancestor guards
  and a bounded content hash. This is authoring evidence, not a proof about all
  custom actions.
- Release the cached MSI's guard immediately after the final proof/digest check
  and before Installer dispatch: holding a deny-delete handle for the whole
  uninstall can prevent Installer maintaining its own cache. Keep the separate
  system executable guard alive across execution. No claim is made that the
  Installer-owned package cache remains locked throughout the transaction.
- Read only the exact MSI GUID registration's typed restrictions. Require
  `WindowsInstaller=1` and successful observations of NoRemove, SystemComponent,
  ParentKeyName and ReleaseType. A successfully absent optional value differs
  from access/type/query failure. Reject restrictive entries and enabled or
  unreadable AlwaysInstallElevated policy. Never query vendor uninstall fields.
- Resolve `msiexec.exe` through the system-directory API, not PATH or WINDIR.
  Hash and execute the same canonical physical file, retaining read-only guards
  until dispatch completes. Fixed separate argv is
  `["/x", "{GUID}", "/qn", "/norestart"]`. Do not globally disable WOW64
  redirection, add a shell, auto-retry or change UI/elevation mode.
- Reacquire registration/restriction/package/tool evidence for preview and
  immediately before dispatch. Changes invalidate the live digest. Current
  GUID/SID/context queries cannot be replaced by the MSIX string-only query.
- MSI has its own terminal classifier and journal validation. Preserve legacy
  MSIX interpretation; do not reuse its absent-registration shortcut for new MSI
  evidence. Recovery can query identity but never repeat removal.
- New MSI leftover authority requires the same operation/identity, `Removed`,
  numeric exit 0 with no interruption, valid SID evidence and authoritative
  absence. Legacy MSI records remain readable but do not gain this new authority.
  Reboot, failed and unknown results cannot enable immediate leftover removal.
- The desktop reports a directory as moved only when the strict report's single
  outcome names the selected candidate and records a successful move. A
  completed operation can contain a skipped directory. Show its live refusal
  reason. A terminal audit failure keeps the result unknown.
- The desktop result for one removed app can open a read-only directory picker.
  There is no confirmation for the picker itself. Selecting one candidate
  requests a strict preview, and the labelled move control confirms exactly
  one directory. Core independently checks the durable operation, identity,
  protections and digest before moving the directory to the Recycle Bin.
  ARP has a fixed Windows Settings handoff. An administrator-only row can
  copy its CLI plan command. Neither route applies a host change.
- Winget plan/preview are read-only: top-level version/help/source inspection
  and target-free upgrade listing, followed by exact in-memory row selection.
  Never use target `--id`/`--name`/`--version` or `--all` on an upgrade probe.
  A target-bearing upgrade occurs only after confirmation, live rebinding and
  durable dispatch-start. Bind actual native tool identity/content, supported
  capabilities, source identity, current-user scope and the selected row.
- Production winget resolves the current user's Store-signed
  `Microsoft.DesktopAppInstaller_8wekyb3d8bbwe` package to its physical native
  executable; do not hash an app-execution alias reparse point as the tool.
  Accept only stable 1.x with successful, untruncated version/help capability
  evidence. Plan/preview/dispatch use the same user scope and exact row/source.
- Source name alone is insufficient. Bind the read-only export's supported
  official `winget` identity (default CDN, PreIndexed.Package type, Microsoft
  source identifier and trusted Store origin). Reject a same-name replacement,
  incomplete/malformed export or unsupported capabilities. Hash only the relevant
  canonical fields; never store source URLs/credentials in audit. Do not accept
  agreements, arbitrary sources or caller overrides to work around refusal.
- Update cancellation reaches version/help/source/row preflight and every late
  probe after durable-start, with a final check before target commit. Cancellation
  before that commit produces zero targeted upgrades; an already-written start
  gets a Refused/canceled_before_start terminal. After commit the installer and
  observation use no-op user cancellation so remaining-item cancellation cannot
  kill an installer or erase its actual result; no rollback is promised.
- Post-update observation must reject truncated/error output, conflicting source
  or duplicate matching IDs. An otherwise successful process plus unverifiable
  observation is Unknown with its real reason, not Updated.
- Startup execution retains the existing current-user StartupApproved funnel.
  Restoring a toggle creates a fresh inverse plan and live preview; do not write
  the old value directly from a result button. Update/startup results use the
  Software support journal and are visible through History. A support event is
  not an uninstall receipt and never grants leftover authority.
- GPUI currently enables only the actions approved at its task gate. Shared
  review code must not automatically activate later removal/leftover
  actions. Same-domain execution remains locked until the real worker terminal;
  stale UI delivery must not replace a newer selection or pretend rollback.
- Current-user MSIX removal consumes one saved Software plan and a live digest
  through `SoftwareExecutor`. Bind the exact package full name, ID and removal
  class. The confirmation control is the sole dispatch entry. Preserve core
  terminal, reboot evidence and exact-package requery in the result panel.

## 4. Validation & Error Matrix

| Evidence | Required behavior |
| --- | --- |
| Malformed GUID, incomplete restrictions/context, protected or unsupported entry | Refuse before removal adapter |
| Missing confirmation, stale digest or changed physical tool/package/registration | Zero destructive dispatch |
| `0` + exact authoritative absence | MSI `Removed`; eligible for later leftover review |
| `0` + present / unavailable / conflicting | StillPresent / UnknownAfterDispatch; no leftovers |
| `3010` / `1641` | Preserve restart-required / already-initiated numeric evidence; no immediate leftovers |
| Other numeric code, including 1605/1614/1618/1603/1602/3011 | Non-success even if a later query reports absence |
| Timeout/lost exit/post-dispatch cancel/crash | Unknown execution; no fabricated success or rollback |
| Initial audit failure | No destructive dispatch |
| Cancel during execution preflight/late probes | No target process; record canceled-before-start refusal when started is durable |
| Terminal audit failure after dispatch | Explicit uncertainty, not a clean pre-dispatch refusal |
| Old MSI record lacks new proof | Readable history, no new removal/leftover authority |
| Historical legacy MSI journal already has a leftover support row | Preserve historical readability; new append/authority still requires new MSI proof |

## 5. Good / Base / Bad Cases

- Good: an eligible per-user MSI goes through saved plan, fresh binding, live
  digest, confirm and a fake adapter; only exit 0 plus exact absence is Removed.
- Base: existing MSIX plans and journals still decode and follow their original
  interpretation; optional MSI evidence is absent.
- Bad: enabling the fixture strategy for arbitrary inventory JSON, treating
  unknown-property as absent, or changing an MSI failure into success solely
  because the product subsequently disappeared.

## 6. Tests Required

Use the real plan/preview/confirmation/executor/journal chain with injected
registration/tool/process boundaries. Cover a positive candidate, strict GUID,
every qualification refusal, same-path replacement, changed policy/package,
late drift, complete exit/requery matrix, interrupted monitoring, durable-start
and terminal failures, recovery without redispatch, legacy decoding and legacy
MSI leftover refusal. Assert refusal call counts and actual separate argv.

CLI tests cover mandatory digest/confirm, closed optional evidence, unchanged
plan grammar and both human locales. Update fixtures assert that no targeted
upgrade appears during plan/preview or refused execution. GPUI tests cover
success/failure/stale/no-confirm, inverse startup review, single-flight and
generation delivery, including polls captured before a new keep-awake action.
Ready row controls must say Confirm when a second activation can dispatch;
calling a control Review must never execute. No test may uninstall/update the host or mutate real
startup state. Fixture evidence is not native Installer/UI acceptance.

## 7. Wrong vs Correct

```text
Wrong: a GUID-like registry key or EligibleMsiQuiet fixture tag authorizes MSI.
Correct: private current-user registration, package/policy and tool proof does.

Wrong: Failed + Absent => MSI Removed; reboot result opens leftovers.
Correct: only numeric 0 + same-identity authoritative absence grants that proof.

Wrong: upgrade --id ID --exact is a harmless preview query.
Correct: list without a target, select the exact row in memory, then dispatch
         the targeted command only behind confirmation and durable start.
```
