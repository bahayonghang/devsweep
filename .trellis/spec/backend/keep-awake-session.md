# Keep-awake request, session and history

## 1. Scope / Trigger

Apply to core keep-awake, its CLI loopback transport, GUI review and Status action
history. The specified DevSweep lease is the scope; do not claim the whole machine
has no other power request. No daemon, automatic discovery, persistent owner
receipt or child CLI process is added.

## 2. Signatures

Public types/functions live under `devsweep_core::status`. Fallible operations
return `Result<_, KeepAwakeError>`:

```text
service.plan_start(owner, duration) -> KeepAwakePlan
service.preview(&plan) -> KeepAwakePreviewV1
KeepAwakeSession::start(service, &plan, expected_digest, confirmed) -> session
session.plan_start(duration) / plan_stop() -> KeepAwakePlan
session.preview(&plan) -> KeepAwakePreviewV1
session.execute_start(&plan, expected_digest, confirmed) -> KeepAwakeStatusV1
session.execute_stop(&plan, expected_digest, confirmed) -> KeepAwakeStatusV1
session.status() / shutdown() -> KeepAwakeStatusV1
query_keep_awake(endpoint, verb, token, owner) -> KeepAwakeStatusV1
```

Existing CLI syntax remains `status keep-awake run --owner ... --duration-ms ...
--confirm` and `status|stop --endpoint ... --token ... --owner ...`. History also
accepts `--domain status`. The CLI-compatible convenience open/run and tuple-
authorized STOP paths use the same core plan/preview/dispatch internally. GUI
must use explicit reviewed-digest execution, not those convenience shortcuts.

## 3. Contracts

- `KeepAwakePlan` is opaque, immutable and retained in memory. Duration is
  explicit whole milliseconds from 1 ms to 12 hours. No new disk plan format.
  A GUI-local hash plus a core bool is not the shared authority chain.
- Preview binds service identity, relevant owner generation, action, owner,
  duration and target token/state. Refresh timestamps, remaining milliseconds,
  unrelated owners and CPU/Status samples are excluded. True lease/state change,
  expiry or failure invalidates the old digest.
- One joined long-lived worker owns all native acquire/refresh/clear calls and
  the backend's destruction. A separate bounded loopback worker prevents a slow
  client from delaying expiry. Service commands also reconcile expiry so a busy
  command queue cannot starve finite duration.
- Backend acquire/release return errors. Failed clear retains an error/active
  internal state and requires explicit retry or shutdown; never report off from
  local bookkeeping. Do not retry/log indefinitely every polling interval.
- A stopped/expired GUI lease keeps its endpoint while the app lives. Reopening
  the same session reuses token/endpoint without accumulating workers. Separate
  services and owners must not clear each other's requests.
- Explicit shutdown and Drop clear on the owner thread and join workers even
  when cleanup fails. Stop reports off only after successful clear. Old endpoint
  connection failure after owner exit means unavailable/unknown, never off.
- Existing endpoint/token/owner checks are a bounded explicit local-session
  protocol, not cryptographic authentication, SID proof or a discovery service.
  Do not persist STOP-capable credentials or introduce a network START verb.
- Production action history is closed V1 at
  `%LOCALAPPDATA%/DevSweep/audit/v1/status.jsonl`, with short per-append
  `status.lock`, flush/sync and bounded reads. It records action, stable outcome,
  finite duration, operation/time and optional preview digest; no token, endpoint,
  owner label, argv, raw path, environment or localized error text.
  Reviewed [Process Actions](./process-actions.md) share this store through a
  distinct action and optional process details; power records require those
  details absent and retain their original finite-duration contract.
- New Status and Software-support history reads, and Status write-side validation,
  have a real 16 MiB bound using a limit-plus-one read, not only an earlier file
  metadata check. Status validates finite durations and SHA-256 syntax. An
  unclosed final Status line cannot become a successful terminal merely because
  its JSON happens to parse; retain started as unknown and report corruption.
- Explicit action initial audit failure blocks its OS call. Terminal audit
  failure reports unknown-after-dispatch. Expiry/exit cleanup still attempts to
  release safely if audit is unavailable and reports uncertainty. Action history
  is not a liveness receipt and cannot prove an unreachable owner's off state.
- History consumes Status actions and the existing Software support lifecycle
  store. Started-only operations remain unknown, not success. In-memory/recording
  journals and failing backends are explicit test dependencies; production uses
  the fixed durable store and Windows backend.
- Server request reads have a 400 ms absolute deadline and client response reads
  a two-second absolute deadline. Partial bytes cannot renew the budget and hold
  shutdown joins indefinitely. A late CLI hold/cleanup error preserves the one
  already-emitted JSON document, emits diagnostics to stderr and returns the real
  failure exit; it must not append a second JSON or overwrite a named output.
- Keep human errors bilingual without changing machine message/code/shape by
  locale. Report History I/O against its known domain, or History when unknown,
  rather than labeling every failure Clean.

## 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Unconfirmed, invalid owner/duration, wrong service or stale generation | No requested backend action |
| Different duration/action/token after preview | Refuse reviewed digest |
| Wrong tuple or non-loopback endpoint | Refuse; no unrelated owner control |
| Native acquire failure | Error, not active |
| Native clear failure | Error/uncertain state, not stopped/off |
| Initial audit unavailable | No explicit acquire/stop call |
| Terminal audit unavailable | unknown_after_dispatch |
| Audit unavailable on expiry/exit | Attempt clear for safety, retain uncertainty |
| Closed/unreachable endpoint | unavailable/unknown; no synthesized off |
| Oversized/corrupt/future audit | Bounded refusal/unsupported history, not silent empty success |

## 5. Good / Base / Bad Cases

- Good: GUI retains a core start plan and digest, confirms, and a known CLI tuple
  queries that lease. Stopping keeps the endpoint observable while GUI lives.
- Base: CLI run remains a finite confirmed session and can exit after terminal
  state. Its old endpoint then reports unavailable.
- Bad: a pool job acquires on one thread and clears on another, a failed API is
  ignored, or a stale journal/connection failure is used as proof of off.

## 6. Tests Required

Fake clock plus recording/failing backend assert same-thread acquire/clear/drop,
finite expiry under busy queries, independent owners, no-confirm/stale refusal,
stable hashes across telemetry refresh, repeated toggles without new workers,
retained stopped endpoints and shutdown/join. Isolated recording/failing audit
tests cover durable start, terminal failure, redaction, malformed/oversized input
and safety cleanup despite audit failure. CLI tests query/stop fake sessions and
read isolated History; they must not start production power requests. Native OS
effectiveness and screenshots remain unverified unless separately observed.

## 7. Wrong vs Correct

```text
Wrong: GUI signs owner/duration and calls a core bool toggle.
Correct: retain a core plan, live-preview it, then pass that reviewed digest.

Wrong: acquire on a worker, clear from GUI Drop; network failure means off.
Correct: same owner worker acquires/clears/joins; unreachable stays unknown.

Wrong: persist the control tuple and call it a lifecycle receipt.
Correct: keep credentials in the live session; record only redacted action history.
```
