# MSI removal and keep-awake contract research

Date: 2026-11-15. Baseline: `faf9dc6` on `dev`; shared Clean changes untouched.
Step 1 research, **not step-3 approval or native acceptance**. No host action,
installed-product probe, app launch, or Computer Use was performed.

**Verified**: inspected code or Microsoft contract. **Proposed**: unapproved
decision. **Unknown**: evidence missing. PRD/design are requirements.

## 1. Executive findings

1. Production MSI identity comes from Windows Installer APIs, not registry
   uninstall text. It already distinguishes current-user unmanaged, current-user
   managed, and machine contexts. It does **not** record the user SID or confirm
   installed versus advertised state in the persisted identity.
2. Existing `EligibleMsiQuiet` / `RemoveMsiProduct` support is a **fixture path**.
   Production inventory still makes all MSI rows manual; the real executor and
   CLI preview reacquire MSIX only. Its binding/exit-map gaps are detailed below.
3. Proposed fixed argv is `["/x", "{UPPERCASE-GUID}", "/qn", "/norestart"]`,
   with a system-API-resolved absolute `msiexec.exe` and no user-selectable UI mode.
   These options do not establish installation context or non-elevation.
4. Installer-success exit codes are exactly `0`, `1641`, and `3010` among the
   researched codes. None alone proves that this exact product is now absent.
   `1605` / `1614` must not create a successful removal by this operation.
5. The live leftover gate is already **`Removed` only**, stricter than the old
   quality-spec text mentioning `RebootRequired`. Keep that stronger gate.
6. Core keep-awake state is process-local, but CLI already implements an explicit
   loopback session protocol. Extracting it into core can support desktop/CLI
   cooperation while the owner lives. Query-after-exit in AC7 needs an additional
   owner-lifetime contract; a disconnected endpoint currently means unavailable.

## 2. MSI identity: actual origin and limits

**Verified source:** `crates/devsweep-core/src/software/msi.rs:38-100,126-197`
obtains the process token SID through `software/msix.rs::current_process_sid`,
enumerates each of the three contexts separately, and queries product metadata.
The FFI is `MsiEnumProductsExW` / `MsiGetProductInfoExW`
(`msi.rs:214-233,251-353`). User contexts pass that exact current SID; machine
passes null. The returned context is checked against the requested one. SID
output buffers are deliberately omitted; the requested SID is not stored.

[MsiEnumProductsExW](https://learn.microsoft.com/en-us/windows/win32/api/msi/nf-msi-msienumproductsexw)
enumerates advertised **or** installed products. Product code is an API output;
context and SID select the registration instance. Machine requires null SID;
other-user/all-user enumeration requires administrative privileges. Enumeration
calls must stay on one thread. Existing enumeration satisfies the latter rule.

`SoftwareIdentity::Msi { product_code, context }` has no SID
(`software/model.rs:106-122`). `scope_for_identity` maps either user context to
`CurrentUser` (`software/mod.rs:294-311`). That is correct for this collector's
current scan, **not proof that a deserialized row originated under this SID**.
The stable ID hashes the identity (`mod.rs:369-381`); do not silently add SID to
that hash and invalidate saved selections without a compatibility decision.

The collector checks a 38-byte braced hexadecimal GUID
(`msi.rs:373-385`); a second reusable check already exists in
`software/launch.rs:25-40`. The official
[ProductCode property](https://learn.microsoft.com/en-us/windows/win32/msi/productcode)
requires uppercase GUID letters and distinguishes product releases, languages,
and 32/64-bit packages. **Proposed:** reuse one pure validator, normalize newly
collected MSI codes to uppercase before identity construction, reject extra
characters/NUL/whitespace/quotes/arguments, and test legacy lowercase handling
without silently rewriting a loaded inventory's fingerprint.

**Verified gaps:** current metadata queries are `ProductName`, `Publisher`,
`VersionString`, `EstimatedSize`; they omit `ProductState`, package identity,
and privilege evidence (`msi.rs:155-167`). MSI flags currently cover protected
name/publisher, source completeness, context conflict, and missing name
(`msi.rs:92-97`); `NoRemove`, system/update restrictions are not positively
collected for MSI. ARP's corresponding restrictions are separate observations,
not merged into MSI authority. Do not interpret these default-false flags as
proof that a newly executable MSI is unrestricted.

[MsiGetProductInfoExW](https://learn.microsoft.com/en-us/windows/win32/api/msi/nf-msi-msigetproductinfoexw)
documents exact SID/context queries, `ProductState` (`1` advertised, `5`
installed), `PackageCode`, `LocalPackage`, and `AuthorizedLUAApp`. The last is a
**patching** capability, not a general uninstall/no-UAC permission.

**Proposed minimum live authority:** exact product code + returned context +
current process SID + installed state, complete relevant context coverage,
current package/version evidence, restrictive eligibility evidence, and the
bound system executable. Keep display name/icon/date/size/install-directory
observations out of authorization. Do not derive MSI authority from a GUID-like
ARP subkey, `DisplayIcon`, `InstallLocation`, or vendor command text.

If the same code appears in multiple applicable contexts, reject it before
preview/dispatch. `/x ProductCode` exposes no explicit SID/context selector in
the documented command surface; this research did not establish a safe
selection precedence. Do not guess, add `ALLUSERS` to redirect uninstall, or
enumerate every user's products to make ambiguity disappear. Other-user
identities, inaccessible coverage, mismatched SID/context, advertised-only
products, and changed registrations remain unavailable/manual. Re-query the
same context and SID after execution; permission/API errors are not absence.

## 3. Existing plan, dispatch, history, and compatibility

| Layer | Verified current behavior | Minimum extension after review |
| --- | --- | --- |
| Eligibility | `software/mod.rs:327-367` returns `MsiExecutionNotSupportedV1` before the normal selectable result. `model.rs:149-150` describes `EligibleMsiQuiet` as fixture-only. | A production MSI eligibility branch with explicit context/permission/ambiguity refusal reasons; preserve all earlier safety refusals. Do not enable vendor/Scoop/ARP execution. |
| Saved plan | `software/plan.rs:78-98` saves only IDs, source fingerprint, observation time and a 15-minute expiry; `model.rs:293-302` is closed V1. | Preserve this shape and old plan readability. Saved fields never choose program/argv. |
| Preview | `plan.rs:146-175,238-296,374-413` reconstructs a strategy token. MSI currently requires in-memory `FixtureLaunch`; the fixture token binds path/args, not executable contents. | A production MSI binding and token that includes SID/context/state/package evidence, executable identity/hash, and fixed policy version. Recompute in preview and execution, then recheck immediately before dispatch. |
| Execute | `execution/mod.rs:194-198,399-430,455-572,746-813` obtains private live MSIX inventory under process permit + durable journal lock, writes `dispatch_started`, then calls MSIX or fixture adapter. | Acquire necessary MSI sources privately too; add an opaque typed MSI strategy/adapter instead of treating fixture launch as host authority. No public caller-injected live inventory or serialized executable action. |
| Requery/recovery | `execution/msix.rs:25-31` accepts only a package-full-name string; `execution/mod.rs:582-598,664-696` strips identity to a string, including an MSI code on recovery. Recovery never redispatches. | Requery by typed identity plus bound MSI SID/context; never pass a GUID into the MSIX query. Legacy MSI fixture records without sufficient context evidence cannot acquire new removal/requery authority. |
| CLI | `commands/software/inventory.rs:24-44` supports MSI inventory/ID plan; `commands/software/execution.rs:24-61` previews MSIX only and requires explicit confirmation/digest for uninstall. | Same command grammar; share core live MSI preview/execute, add typed refusal/result presentation in EN and zh-CN. Do not add shell/elevation flags. |
| History | `execution/audit.rs:96-147,694-837` has closed durable V1 records and already permits valid MSI identity. `history/mod.rs:522-550` decodes the shared audit type. | Add locale-neutral MSI result/context evidence where necessary, update replay validation and history together, and preserve old records. No command text, argv, output dump, or redispatch instructions. |

The fixture launcher is **not a ready host implementation**:
`execution/provider.rs:42-52` treats `3010 | 3011` as reboot success and all other
nonzero codes as failure; `:78-109` kills only its direct child at timeout;
the adapter ignores cancellation (`:28-37`). Production inventory never attaches
`FixtureLaunch` (`model.rs:277-279,348`, `mod.rs:278`), and it is `serde(skip)`.
Preserve the fixture boundary; do not expose a new host-program environment
override. Use a private fake adapter for tests.

**Compatibility proposal:** retain inventory/plan/preview V1 and existing IDs
where feasible. Add optional MSI evidence with `serde(default,
skip_serializing_if = "Option::is_none")` only if needed for persisted SID,
numeric exit code, and reboot-initiated distinction. No evidence means unknown,
never an executable default. Include live binding in an opaque strategy token,
not executable paths in saved plans. New audit evidence must be validated in
transition replay; raw numeric codes are data, not commands.

The parent `research/status-version-strategy.md` permits additive V1 changes
under synchronized readers; `backend/software-evidence.md:27-33` explicitly
distinguishes new-reader backward readability from old-reader forward support.
Software DTOs/audits have `deny_unknown_fields`, so old binaries may reject new
populated fields. Preserve missing-field legacy fingerprints byte-for-byte,
test missing/null/populated round trips and recovery, and do not claim downgrade
compatibility. Do not change MSIX terminal semantics just to fit MSI; a
MSI-specific classifier/evidence validator can preserve old MSIX records.

**AC3 source check performed:** scoped searches of core Software and CLI Software
handlers found exact `UninstallString` / `QuietUninstallString` names only in
tests (`execution/audit.rs:1707-1708`, `execution/tests.rs:968-969`, CLI
`inventory.rs:236-237`, `execution.rs:294,305`). Production ARP reads allowlisted
display/restriction fields. `launch.rs:108-131` has a generic uninstall-text
recognizer under `cfg(test)`, not a registry-reading host path. A whole-source
zero-match claim would therefore be false; the contract is **no runtime read or
execution as authority**. Retain hostile-field tests and source-boundary guards.

## 4. Fixed executable and argv proposal

```text
program = <Windows system directory from API>\msiexec.exe
argv    = ["/x", "{UPPERCASE-PRODUCT-CODE}", "/qn", "/norestart"]
```

[Microsoft Standard Installer options](https://learn.microsoft.com/en-us/windows/win32/msi/standard-installer-command-line-options)
define `/x` as product uninstall, `/qn` as the equivalent of `/quiet`, and
`/norestart` as `REBOOT=ReallySuppress`.
[REBOOT](https://learn.microsoft.com/en-us/windows/win32/msi/reboot) distinguishes
`ReallySuppress` from weaker `Suppress`: suppress both Installer restarts and
restart prompts, including `ForceReboot` handling. Keep one spelling; no need
for duplicate `REBOOT=` arguments, `/passive`, `/qb`, `/qn+`, or UI toggles.
One reviewed action at a time, not batch silent uninstall. Do not include
vendor properties, transforms, package paths, agreement flags, or log-file
paths in this minimal command.

**Proposed resolution/binding:** use
[GetSystemDirectoryW](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getsystemdirectoryw),
append the fixed filename, require a regular local executable and the expected
system-directory relationship, retain no-reparse file/ancestor proof, bind its
canonical identity and SHA-256, and recheck before spawn. Do not use PATH,
PATHEXT, cwd search, caller `SystemRoot`/`WINDIR`, or hard-code `C:\Windows`.
The generic `process/executable.rs:6-28` searches PATH and is inappropriate for
this closed system executable. Follow the live binding model in
`backend/provider-authority.md:27-45` and `execution/provider.rs:107-110,152-162`.
File hash agreement proves preview/dispatch identity, not a Microsoft signature
or arbitrary-package safety.

Use the existing bounded process infrastructure with an explicit MSI policy,
neutral trusted cwd, separate `Command::new(program).args(argv)`, null/bounded
streams, and hidden process creation. No `cmd`, PowerShell, ShellExecute,
`runas`, or GUI-spawned `devsweep.exe`. Windows internally uses a command line;
the application boundary must still keep program and argv distinct.
[CreateProcessW](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw)
documents the ambiguity of unspecified application names and inherited token.
`CREATE_NO_WINDOW` controls a console, not elevation or package UI.

**Unknown until implemented/tested:** WOW64 resolution must bind and execute
the same physical file. Microsoft documents
[file-system redirection](https://learn.microsoft.com/en-us/windows/win32/winprog64/file-system-redirector).
Do not hash native System32 then execute redirected SysWOW64, disable redirection
globally, or claim architecture coverage from a string-only unit test. Fail
closed for an unsupported resolution case; no extra platform matrix this round.

## 5. Elevation and no-UAC boundary

**Verified:** [Using Windows Installer with UAC](https://learn.microsoft.com/en-us/windows/win32/msi/using-windows-installer-with-uac)
describes managed operations running elevated, sometimes without prompting;
per-user-managed is not synonymous with a nonprivileged current-user action.
[Installation Context](https://learn.microsoft.com/en-us/windows/win32/msi/installation-context)
explains per-user versus machine registration; context is not an exhaustive
prediction of package behavior or the caller's token.

There is stronger documentation than just “quiet means fewer dialogs”:
[MsiSetInternalUI](https://learn.microsoft.com/en-us/windows/win32/api/msi/nf-msi-msisetinternalui)
says `INSTALLUILEVEL_NONE` suppresses the Installer elevation prompt even when
required; `INSTALLUILEVEL_UACONLY` re-enables it. The
[command-line options](https://learn.microsoft.com/en-us/windows/win32/msi/command-line-options)
set `/qn` to no UI. **Inference:** the fixed `/qn` requests this Installer
no-prompt mode. It does not grant permissions, prove an action runs unelevated,
or make a machine/managed package eligible. `/norestart` is unrelated to UAC.

Nor can this prove that every package has no external prompt:
[custom actions](https://learn.microsoft.com/en-us/windows/win32/msi/custom-actions)
can run EXEs, DLLs and scripts. Package-specific external UI/elevation behavior
is outside the evidence established here.
[Authoring without the UAC dialog](https://learn.microsoft.com/en-us/windows/win32/msi/authoring-packages-without-the-uac-dialog-box)
requires suitable per-user package authoring and Word Count Summary bit 3;
that authoring statement alone is not an audited uninstall behavior guarantee.

**Proposed conservative preflight, repeated immediately before dispatch:**

- Reject machine, user-managed, other-user, multi-context and inaccessible/
  unknown registrations before starting `msiexec`; use specific reason codes
  (`requires_administrator`, `managed_context_unsupported`,
  `ambiguous_msi_context`, `msi_authority_unavailable`, names provisional).
- A current-user-unmanaged instance is only a **candidate**. Require positive
  installed-state, SID and restrictive-policy proof. Do not guess permission
  from a user-writable directory, display fields, `AuthorizedLUAApp`, or the
  lack of a prompt. No fallback to interactive Installer UI or elevation.
- Preserve R11's hard boundary: while no sufficient package/policy evidence has
  been agreed for unelevated candidates, unknowns remain manual. Research has
  **not** established a generic preflight that proves arbitrary MSI custom
  actions can never request elevation. If R11 is intended as an absolute promise
  covering those actions, medium-integrity MSI dispatch must stay refused until
  a stronger reviewed policy exists. Do not claim AC8 from fixture success.
- A CLI command copied by the GUI must use the same saved-plan/live-preview/
  confirm flow. It is guidance, not automatic execution or an implicit offer
  to elevate. Already-elevated GUI execution must not silently broaden the
  approved machine/managed exclusion.

This is an open implementation-policy decision at step 2, not permission to
weaken R11. The most useful next evidence would be a reviewed disposable
current-user-unmanaged MSI package in a separately authorized environment.

## 6. Exit results, truthful history, and leftovers

The official [MSI error-code table](https://learn.microsoft.com/en-us/windows/win32/msi/error-codes)
identifies the following meanings. The last column is **proposed DevSweep
policy**, deliberately stricter than “a nonzero code always failed” or “absent
always means this operation succeeded.” Keep the numeric exit code in bounded,
typed result/audit evidence, not localized output parsing.

| Exit / event | Microsoft meaning | Proposed result and cleanup authority |
| --- | --- | --- |
| `0` | Action succeeded. | Adapter success; exact SID/context state absent -> `removed`; present -> `still_present`; unavailable/conflicting -> `unknown_after_dispatch`. Only the proven `removed` terminal can enable leftovers. |
| `3010` | Success; restart required to finish. | `reboot_required`, explicit restart-needed evidence; no automatic restart and no immediate leftover authority. |
| `1641` | Success; restart already initiated. | Success class with distinct `reboot_initiated` evidence; record an unexpected violation of the suppression expectation. Do not call it a mere future restart or enable leftovers. Do not discard the actual code because `/norestart` should have prevented this. |
| `1605` | Action valid only for currently installed products. | Operation failure/not-installed result; absence may mean stale registration or wrong context, not a successful removal by us. No success audit or leftovers. |
| `1614` | Product is uninstalled. | Already-uninstalled/not-performed result. Record the observed absence separately if verified, but do not attribute successful uninstall to this operation. No leftovers. |
| `1618` | Another installation is running. | Busy/failure with retry-after-review guidance; no automatic retry, kill, mutex bypass, or success record. See [_MSIExecute Mutex](https://learn.microsoft.com/en-us/windows/win32/msi/-msiexecute-mutex). |
| `1603` | Fatal installation failure. | `failed`; retain numeric evidence. Do not assert unchanged files or complete rollback. |
| `1602` | User canceled installation. | After dispatch, canceled installer outcome plus requery; never `canceled_before_start`. Does not prove rollback or removal. |
| Any other numeric code, including `3011` | Outside the reviewed MSI success set. | Non-success; retain code. Requery may show absence, but do not manufacture this operation's success/leftover authority. No special `3011` reboot mapping. |
| Cancel before durable dispatch | No removal started. | `canceled_before_start`, no process invocation. |
| Timeout, lost exit status, crash, cancel requested after dispatch | No definitive Installer terminal observed. | Unknown/incomplete execution plus exact requery; never “safely canceled” or “rolled back.” No new MSI success/leftover authority based solely on disappearance. |

An Installer return code and installation state are separate evidence. This
requires MSI-specific classification: current `classify_terminal`
(`execution/mod.rs:816-841`) promotes **any** adapter result to `Removed` when
the installed state is absent, and replay calls that classifier
(`execution/audit.rs:807-816`). Reusing it unchanged would contradict the
`1605`/`1614`/unknown rules above. Preserve historical MSIX interpretation;
validate new MSI terminals against the accumulated MSI evidence on replay.

Use bounded monitoring/requery and no automatic redispatch on recovery. The
existing MSIX timing (120 seconds, 0/2/10-second queries) can be the initial
monitoring budget, **not an assertion that MSI always completes in two minutes**.
MSI cancellation differs from WinRT cancellation: killing a client or its job
does not establish that an Installer transaction or package action was reversed.
Treat that as an unknown boundary and keep the UI truthful; do not expose a
cancel button promising rollback. A final audit write failure after dispatch
must also report uncertainty rather than hide a possible side effect.

**Verified leftover gate:** `software/leftovers.rs:749-759,1103,1302-1308` calls
`uninstall_removed`; `execution/audit.rs:614-627` requires the exact operation
and identity with terminal `Removed`. `RebootRequired`, `StillPresent`, failed,
unknown and pre-start cancellation are refused. Keep this behavior; the old
`quality-guidelines.md:466-469` description is stale. `leftovers.rs:539-546`
also supplies no MSI install location. Initially only the existing separately
reviewed name-based candidates can appear; do not invent a certain installation
directory by matching an ARP row. Preserve recycle-only, live digest, explicit
selection, protections and registry-backup rules.

## 7. Keep-awake: core table, real CLI protocol, and AC7

**Verified core:** `status/keep_awake.rs:250-289` stores leases in a per-service
`Mutex<Vec<Lease>>`, not shared memory or a global broker. `run` requires confirm,
owner and a positive finite duration at most 12 hours (`:301-335`); stop/cancel/
owner-exit release matching leases (`:344-409`), and `KeepAwakeOwner::drop`
releases its owner (`:412-431`). Multiple independent services do not share the
table. The Windows backend counts holds, calls `SetThreadExecutionState`
(`:166-213`) and clears on drop; the API return value is currently ignored.
Expiry is refreshed by service calls, not an independent core timer.

**Verified CLI, not just the core table:**
`application/cli.rs:915-951` requires:

```text
status keep-awake run --duration-ms <1..43200000> --owner <OWNER> --confirm
status keep-awake status --endpoint <LOOPBACK:PORT> --token <TOKEN> --owner <OWNER>
status keep-awake stop   --endpoint <LOOPBACK:PORT> --token <TOKEN> --owner <OWNER>
```

`commands/status.rs:357-469` creates the service and binds `127.0.0.1:0`, emits
endpoint plus lease, then keeps that process alive. It is not a detached daemon.
Queries use that endpoint rather than constructing a fresh service table
(`:382-394,547-595`). The protocol sends `STATUS|STOP|CANCEL TOKEN OWNER\n` and
returns one JSON line with `ok,status,code,message` (`:503-545,614-663`). The
listener checks loopback; token ownership is checked in core. Client connect/
read/write deadlines are two seconds; server per-client I/O is 400 ms, reads
are bounded. There is no installed-session discovery file or system-wide list.
Tokens are counter/time-based (`keep_awake.rs:488-495`), so do not describe this
loopback/tuple check as cryptographic authentication or a verified SID boundary.

The hold loop refreshes every approximately 20 ms, processes queries, and exits
on terminal lease or Ctrl+C (`status.rs:471-495`). A successful STOP replies
`stopped`, then ends the loop and drops the listener; the session drop releases
that owner (`:419-422`). A later STATUS against the closed endpoint returns
`network_unavailable`, **not off** (`:553-559`). GUI exit would have the same
communication limit. A connection error alone cannot distinguish dead owner,
busy listener, blocked connection, port reuse, or an inaccessible live owner.

**Proposed smallest reuse:** move this finite-session server/client transport
and error mapping boundary into a frontend-neutral core module. CLI remains a
thin parser/presenter; GPUI `Session` owns a joined worker/service plus owner
guard, sends start/stop/query to that one worker, and publishes the explicit
endpoint/token/owner for the equivalent CLI. Do not spawn `devsweep.exe`, create
a daemon, or add automatic discovery just to query a known endpoint.

The dedicated worker is essential:
[SetThreadExecutionState](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setthreadexecutionstate)
is a **thread** execution requirement and returns failure as null. Acquire,
expiry refresh, stop, and final clear must all happen on the same long-lived
thread. A GUI pool job acquiring on one thread and a drop/query clearing on
another is not a correct lifecycle. Return a capability error if acquisition
fails; the current backend's ignored return cannot prove the OS is kept awake.
Do not alter persistent power settings or promise to prevent explicit user
sleep/lid action. With separate owner threads, GUI and CLI requests may coexist;
stopping one owner's lease must leave the other's request intact. Two independent
services on the same thread could clear each other's thread flags, so avoid that
layout. Closing the GUI joins and clears only its worker/owner, including app
quit/error paths, not merely leaving the Status page.

**AC7 conclusion:** active GUI lease -> CLI STATUS can be supported by that
core extraction without persistence. Literal STATUS -> off after GUI shutdown
cannot be implemented by the existing endpoint-only protocol. To preserve AC7,
the recommended extension is a small current-user owner receipt (matching tuple,
owner PID + creation-time identity, finite deadline, last terminal observation),
created/updated atomically by the session owner. It is observational metadata,
not execution authority; it must never reactivate a lease. STATUS may report
`owner_exited/off` only after verifying that exact owner no longer lives;
otherwise a missing endpoint remains unknown. While GUI remains open, keep the
endpoint available for its stopped lease so STATUS can report `stopped/off`.
No permanent service is needed. This adds a bounded persistence/liveness contract
and tests, beyond merely adding `KeepAwakeService` to `Session`.

If that small extension is declined, AC7's post-exit wording needs an explicit
user decision; do not silently weaken it to “connection failed means off.” The
GUI should still render its own released state as off, never claim every other
process's keep-awake request is off. AC7 concerns the specified DevSweep lease.

## 8. Verification plan without Computer Use or host mutation

These are **future checks**, not tests run in this research step.

- Pure MSI fixtures: valid/invalid and lowercase GUIDs; installed versus
  advertised; current SID versus other SID; missing/permission/partial sources;
  duplicate code across contexts; protected/NoRemove/unknown privilege. Assert
  refused cases make zero adapter calls, not merely a failed exit.
- Inject a private system resolver, registration provider and fake MSI adapter.
  Assert exact separate program/argv, no PATH fallback, same-path binary/hash or
  SID/context/package/policy replacement invalidates digest, and pre-dispatch
  recheck catches late change. Never put a fake `msiexec` on host PATH.
- Exercise the complete plan -> live preview -> confirmation -> execution chain
  and every reviewed exit/requery pair. Include no digest/confirm, cancel before
  and after durable start, timeout, busy, crash recovery without redispatch,
  exclusive journal lock, started/terminal audit failures, and legacy records.
  Unknown/1605/1614/3010/1641 must not expose a leftover execution entry.
- CLI parser/help and handler contracts cover existing grammar and EN/zh-CN
  output with locale-neutral JSON. `DEVSWEEP_SOFTWARE_INVENTORY` is a GPUI
  read fixture (`software_review.rs:27`), **not** a production CLI live-authority
  override. Use test dependency injection for CLI execution, not an env bypass.
- GPUI tests use its `UninstallExecutor` closure
  (`software_review.rs:32-34,773-890`), retained inventory fixtures, and existing
  update/startup fixtures. Cover first-click review, explicit confirmation,
  refusal, stale selections and obsolete background delivery. Do not infer
  real-host acceptance from these simulated executors.
- Keep-awake fake clock + recording/failing backend prove finite duration,
  same-thread acquire/clear, expiry, independent owners, no persistent settings,
  GUI close/drop and startup failure. Transport tests use loopback in isolated
  test processes with the recording backend; test wrong owner/token, stopped
  query, owner exit, PID reuse, stale/malformed receipt and unavailable owner.
- After implementation run the task's focused core Software/Status and CLI
  contracts, then `just ci` and `just gpui`. No repeated heavy gates for this
  document-only research. This step checks documentation and `git diff --check`.

**Missing native evidence:** no actual MSI exit behavior, privilege/UAC display,
reboot suppression, package custom actions, OS keep-awake effectiveness, or
English/Chinese review/result screenshots were exercised. User has prohibited
Computer Use. AC10's screenshot clause remains unfulfilled; source/fixture
evidence must be reported under its actual scope. A future real-package probe
requires fresh explicit authorization and a disposable target/environment.

## 9. Concrete decisions for the step-2 user review

1. **Fixed command:** approve system-resolved and identity-bound `msiexec.exe`
   with exactly `["/x", "{GUID}", "/qn", "/norestart"]`; no UI choice, shell,
   vendor command, automatic elevation, or batch mode.
2. **Result rules:** approve section 6: only `0/1641/3010` are reviewed Installer
   success codes; `1641` retains restart-already-initiated evidence; only proven
   `0 + exact absence` creates new MSI `removed`/leftover authority. Known
   failures, already-absent codes, unknown code, timeout and canceled monitoring
   do not become this operation's successful uninstall merely from absence.
3. **Context/UAC boundary:** approve refusing machine, managed, other-user,
   multi-context and unknown privilege cases, even when silent execution might
   otherwise succeed. Resolve whether R11 covers arbitrary package custom
   actions absolutely. Recommended: retain the hard unknown -> refusal boundary;
   do not mark all current-user-unmanaged MSI rows executable until a sufficient
   positive package/policy proof is specified. This can leave live MSI dispatch
   unavailable for some/all unelevated candidates initially; it is a real scope
   constraint, not a completed MSI feature.
4. **Keep-awake AC7:** approve core extraction of the existing explicit protocol,
   one owning worker thread and a bounded current-user lifetime receipt to
   distinguish stopped/exited from unknown after endpoint closure. Cost: a small
   additional persistence/liveness contract and tests; no daemon or automatic
   discovery required. If declined, discuss AC7 explicitly before step 6.
5. **Compatibility:** approve additive optional MSI evidence under the synchronized
   V1 reader policy, preserving old saved formats/IDs and MSIX audit semantics;
   no assertion that older closed binaries read newly populated evidence.

Implementation must synchronize core, CLI, GPUI, audit/history readers and specs.
No product, checklist, commit, push, or archive change in this research step.

Validation: diff, UTF-8 and whitespace checks passed. Product gates were not
run for this document-only step.
