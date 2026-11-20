# Step-1 MSI and keep-awake research review

Date: 2026-11-15. Baseline: `faf9dc6` (`dev`). Reviewed the task's
`check.jsonl` references, PRD, design, implementation plan, full on-disk
`msi.md`, owning core/CLI/GPUI code, and Microsoft primary documentation.
The injected quality guide was truncated; the Software execution/support
sections were read directly from the actual file.

**Disposition:** the command and result proposals are ready for the step-2
user review. The research does not yet supply an implementable, approved
answer to the absolute no-UAC interpretation or to AC7 after app exit.
Do not start step 3 until the user reviews the MSI contract. Steps before
process termination and before MSIX removal retain their separate gates.
No product behavior or acceptance requirement was changed by this review.

## Findings (fixed)

- File: `research/msi.md`. Repeated introduction, scope and validation text
  pushed the document past the 32,768-byte injection limit, which could hide
  the final decisions. Condensed those repetitions to below 32,000 UTF-8
  bytes, retaining evidence and all five unapproved decisions. Clarified
  "no UI-selection flag" to "no user-selectable UI mode": `/qn` itself is
  a UI-mode option. No substantive Microsoft/source claim needed correction.

## Findings (not fixed)

### 1. R11 needs a product guarantee that permits a real MSI feature

Research sections 5 and 9.3 correctly admit that no generic positive proof
was found that arbitrary MSI custom actions cannot display external UI or
request elevation. Their proposed "unknown privilege -> manual" rule must
not be described as a complete first MSI implementation: without a concrete
eligibility predicate it can refuse every current-user-unmanaged product.
The unresolved issue is the promised behavior, not another GUID validator.

[MsiSetInternalUI](https://learn.microsoft.com/en-us/windows/win32/api/msi/nf-msi-msisetinternalui)
documents suppression of the Installer elevation prompt at `INSTALLUILEVEL_NONE`;
[command-line options](https://learn.microsoft.com/en-us/windows/win32/msi/command-line-options)
map `/qn` to no UI. Applying that UI-level contract to `/qn` is a reasonable
documented inference, not native acceptance. It does not establish that an
entire transaction runs without privileged service work:
[managed Installer operations](https://learn.microsoft.com/en-us/windows/win32/msi/using-windows-installer-with-uac)
can run elevated without prompting. Packages can also run executable/script/DLL
[custom actions](https://learn.microsoft.com/en-us/windows/win32/msi/custom-actions).
Quiet flags are not a sandbox around those actions.

**Recommended decision for the user, not approval:** define R11 as DevSweep
using no `runas`, elevation request/fallback or interactive Installer mode,
with Installer's own UI disabled by the fixed `/qn`; explicitly exclude a
universal guarantee about arbitrary third-party custom actions. Continue to
refuse known administrator-required, machine, managed, other-user and ambiguous
registrations. Unknown or incomplete *identity/registration/restriction*
evidence remains a refusal. Distinguish that from demanding an unavailable
proof about every possible package action.

After that decision, specify the positive current-user-unmanaged predicate
before coding: exact current SID and context, installed state, complete
relevant enumeration, unique applicable product code, stable package/version
evidence, protected/restriction checks with named evidence sources, and live
system-tool binding. `NoRemove` and system/update flags currently default false
for MSI; neither that default nor `AuthorizedLUAApp` proves eligibility.
Do not silently invent an allowlist or use display metadata as authority.
If the user retains the absolute guarantee, record the MSI dispatch scope as
deferred/unavailable; R2/AC2 are not delivered by a refusal-only branch.

This changes a public product contract and therefore was left for the user;
the review did not rewrite the PRD or approve either alternative.

### 2. AC7's owner receipt is an unreviewed design, not proven off-state evidence

The existing CLI already owns the finite-session loopback protocol in
`crates/devsweep-cli/src/application/commands/status.rs:357-663`. Its STOP
response can show `stopped`, then the server exits; a later connection failure
is `network_unavailable`. A direct second `KeepAwakeService` cannot query the
first service's process-local lease table. Research sections 7 and 9.4 are
correct about that gap, but a small receipt does not resolve it by itself.

If persistence is chosen, a separate design must define at least:

- How the existing endpoint/token/owner CLI tuple locates the matching receipt;
  versioning, retention and atomic creation before publishing an active lease.
- Current-user storage and read/write permissions, exact session/owner binding,
  bounded parsing, stale/forged records and endpoint/port reuse. Current tokens
  are counter/time based (`keep_awake.rs:488-495`); persisting a STOP-capable
  credential must not be called cryptographic authentication.
- PID reuse and creation-time matching, an exited process object still having
  a handle, reboot and abnormal exit, and separate treatment of access-denied
  or failed liveness queries. A PID alone is not stable owner identity
  ([process identifiers](https://learn.microsoft.com/en-us/windows/win32/procthread/process-handles-and-identifiers));
  [OpenProcess](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocess)
  can fail for access reasons, and
  [GetProcessTimes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocesstimes)
  itself requires query rights and can fail.
- Acquisition/clear/receipt ordering: a stale terminal or finite deadline is
  not evidence that a blocked, still-live owner actually cleared its request.
  Missing, mismatched or unverifiable evidence must remain unknown. A receipt
  must never recreate a lease or authorize an unrelated action.

**Recommended smaller decision for the user:** do not introduce receipts for
this task. Extract the existing transport into core, use one dedicated owning
thread, and keep the GUI session endpoint alive for stopped leases while the
app remains open. Revise AC7 explicitly to require: CLI reports that specified
live GUI lease as active/stopped; GUI exit releases its own OS request; queries
to the old endpoint after exit report unavailable/unknown. This is a proposed
AC7 change, not fulfillment of its current literal "reports off after exit".
If that wording must stay, design and review the receipt/liveness contract
separately before step 6. A daemon is not required merely to satisfy a slogan.

Neither outcome says the whole computer has no other keep-awake request.
Independent CLI and GUI owners may coexist; stopping one leaves the other.
Core extraction must also address `AwakeBackend::acquire` currently returning
no result and ignoring OS failure (`status/keep_awake.rs:58-61,166-213`).
The [Windows thread-state API](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setthreadexecutionstate)
has thread scope and a failure return. Acquire, expiry, stop and clear belong
to the same joined worker, not separate pool jobs. Final clear precedes a
successful off response; failed or uncertain OS calls cannot prove off.

Receipt/API/lifecycle choices affect public interfaces and persistence, so
they were not implemented or silently substituted into the research proposal.

### 3. Keep the MSI extension separate from the fixture/MSIX shortcuts

The research's core facts are confirmed:

- `software/msi.rs:38-100,126-197,251-353` uses Windows Installer enumeration
  with current process SID for user contexts and null for machine context.
  It does not capture returned SID or `ProductState` in persisted identity.
  API enumeration includes advertised products and is not a permission proof
  ([enumeration](https://learn.microsoft.com/en-us/windows/win32/api/msi/nf-msi-msienumproductsexw),
  [exact-context product query](https://learn.microsoft.com/en-us/windows/win32/api/msi/nf-msi-msigetproductinfoexw)).
- `software/mod.rs:327-367` makes production MSI manual. Live executor inventory
  (`software/execution/mod.rs:194-198`) and CLI preview
  (`commands/software/execution.rs:24-40`) acquire MSIX only. Fixture launch
  data is not serialized authority; no fixture environment override should
  become a production MSI executor.
- The `/x ProductCode` surface has no SID/context parameter. Refuse duplicate
  applicable contexts and incomplete coverage rather than guessing which
  registration is removed. Typed requery/recovery must retain SID/context;
  existing string-only MSIX requery cannot simply accept an MSI GUID.
- System-API resolution plus canonical file identity/hash is an appropriate
  proposal, with no PATH, caller `WINDIR` or shell fallback. It proves tool
  identity, not package safety. Hash and execution must address the same
  physical file under WOW64; that remains an implementation/native evidence
  limit ([system-directory API](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getsystemdirectoryw),
  [redirection](https://learn.microsoft.com/en-us/windows/win32/winprog64/file-system-redirector)).

These are step-3 implementation requirements after approval, not defects to
fix in product code during step-1 review. Source searches confirmed the exact
vendor uninstall field names occur only in scoped hostile-input tests; no
production read/execute authority was found. Do not claim the test tree has
zero matches.

### 4. Exit, absence and journal semantics need one explicit MSI matrix

The fixed argv proposal is supported by Microsoft's
[standard options](https://learn.microsoft.com/en-us/windows/win32/msi/standard-installer-command-line-options)
and [REBOOT contract](https://learn.microsoft.com/en-us/windows/win32/msi/reboot):
`["/x", "{GUID}", "/qn", "/norestart"]`, with no extra vendor properties.
The [Installer exit-code table](https://learn.microsoft.com/en-us/windows/win32/msi/error-codes)
supports the research's `0/1641/3010` success set and distinction between a
restart already initiated and one still required. `3011` is not an MSI success
exception. The proposed attribution/leftover policy is stricter application
policy and requires user approval.

Recommended reviewable matrix:

| Evidence | Proposed MSI terminal/authority |
| --- | --- |
| `0` + authoritative exact absence after bounded requery | `removed`; only this new MSI case grants leftover authority |
| `0` + still registered | `still_present`; wait/requery within the reviewed budget, no early leftover access |
| `0` + unavailable/conflicting requery | `unknown_after_dispatch` |
| `3010` | `reboot_required`; no immediate leftovers |
| `1641` | Reboot outcome with explicit **already initiated** evidence; unexpected under suppression, no leftovers |
| `1605`, `1614`, `1618`, `1603`, `1602`, `3011`, other numeric failure | Non-success with numeric code and separate observed state; absence cannot turn it into this operation's success |
| Timeout/lost status/post-dispatch cancellation/crash | Unknown execution; exact requery does not manufacture a success code or prove rollback |

`software/execution/mod.rs:816-841` currently maps absent registration to
`Removed` even for failed/unfinished adapters. Audit replay uses that same
classifier (`execution/audit.rs:807-816`). Both need MSI-specific evidence
validation while retaining historical MSIX interpretation. Fixing just the
adapter exit map is insufficient. Recovery never redispatches.

The existing leftover gate is verified as exact operation/identity **Removed
only** (`execution/audit.rs:614-627`, `leftovers.rs:749-759,1302-1308`). Preserve
it; the quality-guide reference to `RebootRequired` is stale. Do not clear
leftovers while registration remains or require a real uninstall for tests.

Before implementation, pin how `1641` is represented without conflating
"initiated" with "required". An optional numeric/subtype evidence extension
with an existing reboot terminal is a possible approach, not an approved DTO
change. Journal readers are closed V1. New readers must preserve old records,
IDs/fingerprints and MSIX semantics; older binaries need not understand new
populated fields. Legacy missing MSI evidence must never acquire fresh host
authority. Update transition validation, recovery, history and compatibility
tests together; do not rely only on the Status additive-V1 precedent.

The public schema and classification are intentionally left unmodified.

## Verification

- Documentation/source checks: scoped vendor-field search, direct call-chain
  inspection, Microsoft references, UTF-8/whitespace/size and diff checks.
- Lint: not run; documentation-only ownership, no product changes by this agent.
- TypeCheck: not run for the same reason.
- Tests: not run. The research's fixture/test-executor plan is suitable for
  future plan -> preview -> confirm -> execution authority tests, including
  zero calls on refusal, late binding changes, full exit/requery/recovery
  matrix, journal failures and fake-clock/same-thread lease lifecycle.
- Such tests can prove application authorization/state transitions, not actual
  Installer elevation/custom-action/reboot behavior, effective OS power holds,
  or EN/zh native review/result rendering. Those native items remain
  **UNVERIFIED**. No Computer Use, host mutation or heavy gate was performed.
- Do not silently waive AC10 screenshots or the pre-step-9/pre-step-10 review
  gates because Computer Use is prohibited. Present actual fixture evidence
  and its limits at those gates; any acceptance change belongs to the user.

## Suggested step-2 decision request

Ask the user to review the exact command and result matrix above, and explicitly
choose the narrower R11 guarantee and revised AC7 semantics before treating
them as requirements. Recommend those two narrower guarantees so a real,
bounded current-user MSI path and keep-awake lifecycle can be implemented.
If the user declines, retain the original requirements and report the affected
capability as unresolved; do not disguise refusal-only code as delivery.

Follow-up cross-check (2026-11-15): read `step-2-review.md`. Its three-item
proposal accurately preserves the command/result findings and the recommended
narrower R11 and revised AC7 boundaries, with no receipt added. It explicitly
retains user approval, positive-eligibility/schema work, native-evidence limits
and both later review gates as pending; no substantive correction was needed.
This cross-check is not user approval or permission to start implementation.
