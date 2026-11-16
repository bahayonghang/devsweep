# Reviewed process termination

## 1. Scope / Trigger

Apply to Status process termination, its CLI/GPUI adapters and Status history.
Core owns live process identity, protected classes and dispatch. Sampling remains
read-only; a sampled PID/name is not termination authority. No elevation, shell
command, automatic retry or software-removal capability is added by this action.

## 2. Signatures

Public functions under `devsweep_core::status` retain the existing V1 documents:

```text
bind_process_termination_plan(pid, creation_time_filetime, name)
preview_bound_process(&plan, &dyn ProcessProbe)
terminate_bound_process(&plan, digest, confirm, probe, killer)
terminate_bound_process_audited(&plan, digest, confirm, probe, killer, audit)
terminate_bound_process_audited_cancellable(&plan, digest, confirm,
                                           probe, killer, audit, cancel)
protected_process_class(name) -> Option<ProtectedClass>

ProcessActionAudit::append(StatusActionAuditRecordV1)
FileProcessActionAudit::production()
```

The audited wrappers return `Result<ProcessTerminationReportV1,
ProcessActionError>`. CLI uses the compatible audited facade; GUI passes its job
observer to the cancellable entry. The low-level helper
remains compatible; calling it alone does not fulfill action-history requirements.
CLI grammar stays `status process plan / preview / terminate`; terminate requires
the saved plan, live `--preview-digest sha256:...` and `--confirm`.

## 3. Contracts

- Saved plan fields remain `version`, `pid`, `creation_time_filetime` (canonical decimal
  Windows FILETIME string), `name`. Preview also contains the canonical data-loss
  warning and digest; success report retains the existing `terminated` outcome.
  Human English/Chinese messages localize; machine fields and digests do not.
  PID and FILETIME are nonzero; audit FILETIME is canonical decimal, and process
  names reject control characters. Native `TerminateProcess` success preserves
  V1 `outcome=terminated`; human UI says the termination was requested rather
  than treating the API return as proof that the process is no longer running.
- GUI resolves a fresh identity on its background worker before binding and
  reviewing the plan. `ProcessV1` sampling does not acquire a creation-time field
  or executable permission. Show the actual reviewed name/PID and unsaved-data
  warning, then require a separate confirmation.
- Core validates protected classes and identity at preview and execution. The
  native lookup reads name and creation time from the same opened process handle;
  do not combine a Toolhelp name with a later PID lookup's creation time.
  The native killer compares creation time on the actual process handle before
  termination. A name or PID alone cannot authorize a replacement process.
- UI protection derives from core's classification, with a visible reason.
  Selection/navigation/search or relevant row changes invalidate review. Normal
  CPU/memory changes do not alter a retained plan. Core still rejects same-name
  PID reuse at execution. Late preview/result delivery cannot transfer authority
  to another row; single-flight remains held until the actual worker terminal.
- Process confirmation belongs to the explicit Confirm control (mouse, focused
  keyboard or accessibility activation). Global Enter from the root or search
  input cannot dispatch a process action; activating the row's Review entry
  always prepares review, even when another process preview is Ready. Preserve
  the existing keyboard behavior of unrelated actions.
- CLI checks a named create-new output destination before any termination.
  An already-existing output file must cause zero killer calls. A later sink
  failure cannot undo a dispatched action; keep its real durable history and
  report the output error rather than claim rollback.
- Unbacked Hive fixtures use a private Hive-only executor, with no native
  process controller/audit or awake owner. That executor accepts only Hive
  startup intents at its boundary; environment/display fixtures cannot install
  a production process executor. Fake controllers remain explicit test injection.
- The audited wrapper validates confirmation, plan, live identity and digest
  before writing Started. A durable Started then precedes the final core/handle
  check. Refusal/failure/success gets a terminal record; never retry termination
  from a journal or a delivery failure.
- GUI cancellation is checked before dispatch, including after durable Started
  and final live checks. `canceled_before_start` records a refusal when a Started
  already exists and must produce zero actual killer calls. After native commit,
  cancellation cannot undo termination or replace its real outcome with rollback.
- Track whether the actual killer was called. A failed terminal append after
  final refusal with zero killer calls remains `audit_unavailable`; Started-only
  history is still unknown. Only an attempted killer plus terminal persistence
  failure becomes `unknown_after_dispatch`.
- Process actions share `%LOCALAPPDATA%/DevSweep/audit/v1/status.jsonl` and
  `status.lock` with power actions. Preserve bounded reads, per-append locking,
  flush/sync and incomplete-line handling. Optional V1 `process` is:
  `{pid, creation_time_filetime, name, error_code?}`. Its name is a reviewed
  basename, never a command line or full executable path.
- `action=process_terminate` requires process details, `duration_ms=0` and a
  valid preview digest. Power actions retain their finite duration contract and
  require `process=None`; do not reinterpret them as process events. Existing
  records without the optional field remain readable. `KeepAwakeAuditAction`
  remains a compatibility alias for `StatusActionAuditAction`.
- Status history is evidence of an attempted action, not current process
  liveness. Started without a valid terminal remains unknown. An incomplete
  final line cannot prove success even if its JSON parses. No endpoint/token,
  owner label, environment, argv or localized error text belongs in this journal.

## 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Missing confirmation, invalid plan, stale digest | Refuse; zero killer calls |
| Zero PID/FILETIME, noncanonical FILETIME or control-character process name | Invalid plan; zero killer calls |
| Existing named CLI output | Refuse before dispatch; zero killer calls |
| Protected name/class, missing process, changed name/creation time or PID reuse | Refuse; zero termination |
| Query/access/platform failure | Typed refusal/failure; no elevation fallback |
| Started append fails | `audit_unavailable`; zero killer calls |
| Identity changes after durable Started | Final check refuses; record refusal |
| Cancel before killer commit | `canceled_before_start`; zero killer calls, Refused terminal if Started was durable |
| Killer fails | Persist real failure; do not display terminated |
| Terminal append fails after final refusal, before any killer call | `audit_unavailable`; zero killer calls, history remains unknown |
| Terminal append fails after actual killer call | `unknown_after_dispatch`; no fabricated success or automatic retry |
| Crash/unterminated history operation | Unknown; history never redispatches |
| Process/power field combination or digest is malformed | Reject record, retain bounded corruption reporting |

## 5. Good / Base / Bad Cases

- Good: a fake process is bound, live-previewed, explicitly confirmed, audited
  before the recording killer runs, then recorded as succeeded.
- Base: old V1 plans and old finite-power history continue to decode unchanged.
- Bad: reuse a sampled PID as authority, dispatch directly from a row called
  Review, or display success despite an undurable terminal.

## 6. Tests Required

Use fake probes/killers and isolated journals. Assert call counts for unconfirmed,
stale, protected, vanished and reused identities, including the final identity
check. Cover durable-start/terminal failures, killer errors, redacted records,
pre-killer cancellation during Started/final live checks,
legacy/new history, malformed field combinations and incomplete final lines.
CLI tests preserve grammar, digest/confirm and bilingual human vs stable machine
output, including pre-dispatch output refusal. GUI tests cover separate review/confirm,
zero dispatch from root/search Enter or row Review, protected reasons, search/row
refresh invalidation, late delivery, single-flight and fixture isolation. Run
`just ci`, `just gpui` and the tools tests. Fake results and text projections do
not establish native Windows termination or screenshot acceptance. A test-owned
process may exercise the existing native helper in isolation; report that actual
coverage separately from fake Session tests and native UI acceptance.

## 7. Wrong vs Correct

```text
Wrong: sampled PID -> native termination -> best-effort history -> success UI.
Correct: fresh identity -> saved plan -> live preview -> explicit confirmation
         -> durable Started -> final identity/handle check -> terminal history.

Wrong: old Started or absent row proves the requested process was terminated.
Correct: incomplete action history stays unknown; no automatic retry.
```
