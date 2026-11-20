# Shared keep-awake implementation

## Core API and bounded lifecycle

The existing CLI already provides explicit loopback endpoint/token/owner
`STATUS`, `STOP`, and `CANCEL` commands. They now share core transport and
retain the V1 lease payload. No discovery, persistent owner receipt, daemon
or child CLI process is added. Unreachable endpoints mean unavailable/unknown.

A joined service worker exclusively acquires, expires and clears the
thread-scoped request. Backend acquire/release return errors; failed clear
never becomes stopped/off. Session owns that worker plus a small transport
worker, so a slow client cannot postpone expiry. Different services own
independent requests; token-specific accounting also preserves other owners
inside one service.

All fallible public operations return `Result<_, KeepAwakeError>`:

```rust,ignore
service.plan_start(owner: &str, duration: Duration) -> KeepAwakePlan;
service.preview(plan: &KeepAwakePlan) -> KeepAwakePreviewV1;
KeepAwakeSession::start(service: KeepAwakeService, plan: &KeepAwakePlan,
    expected_digest: &str, confirmed: bool) -> KeepAwakeSession;
session.plan_stop() -> KeepAwakePlan;
session.plan_start(duration: Duration) -> KeepAwakePlan;
session.preview(plan: &KeepAwakePlan) -> KeepAwakePreviewV1;
session.execute_stop(plan: &KeepAwakePlan, expected_digest: &str,
    confirmed: bool) -> KeepAwakeStatusV1;
session.execute_start(plan: &KeepAwakePlan, expected_digest: &str,
    confirmed: bool) -> KeepAwakeStatusV1;
session.endpoint() / token() / owner() -> &str;
session.status() / stop() / cancel() -> KeepAwakeStatusV1;
session.shutdown(&mut self) -> KeepAwakeStatusV1;
query_keep_awake(endpoint: &str, verb: KeepAwakeVerb, token: &str,
    owner: &str) -> KeepAwakeStatusV1;
```

`KeepAwakePlan` is opaque, immutable and cloneable. Preview returns action,
owner, duration_ms, current_state and a sha256 digest. It binds service identity,
the relevant owner's state generation, action, duration and target token/state.
It excludes refresh timestamps, remaining milliseconds and unrelated Status
samples. Different service/owner/duration/action or changed/expired lease
rejects stale authority. Duration must be explicit whole milliseconds within
1 ms through 12 hours. Unconfirmed actions make zero backend/audit calls.

Stop/expiry retains the endpoint while the app lives. Restart uses the same
session token and endpoint, so repeated GUI toggles do not accumulate leases
or workers. Explicit shutdown closes transport, clears on the owner thread,
and joins both workers even on failure. Drop uses the same cleanup and logs
otherwise unreturned failures. Session is non-cloneable, Send + Sync.

## Confirmation compatibility

The shipped `status keep-awake run --owner ... --duration-ms ... --confirm`
has no plan/preview subcommands or digest argument. Preserve this grammar:
`KeepAwakeSession::open` and `KeepAwakeService::run` are CLI-compatible
convenience paths that internally build the same plan, live preview and
confirmed dispatch. Existing tuple-authorized network STOP also uses the
same core stop chain internally. No network START verb is introduced.

Desktop explicitly retains the core plan/preview, shows the common review,
then passes its reviewed digest to `Session::start` or the session's checked
execute methods on second confirmation. It never signs a GUI-local digest
or substitutes convenience open/stop calls for this review.

## Action history

Production flushes/syncs closed V1 actions to
`%LOCALAPPDATA%/DevSweep/audit/v1/status.jsonl`, serialized per append by the
short-lived `status.lock`. This is action history, never an owner receipt or
off-state proof. Start, explicit stop, cancellation, expiry and owner exit
record separate operation IDs with stable started/succeeded/backend-failed
codes. Fields are schema/domain, operation ID, timestamp, action, status code,
finite duration and optional preview digest. Token, endpoint, owner label,
argv, path and error text are absent. History gains Status and CLI
`--domain status`. Software's fixed `software-support.jsonl` is also read
under the existing Software domain, including Update lifecycle records.

Initial audit failure blocks acquisition/explicit stop before dispatch.
Terminal audit failure returns unknown_after_dispatch despite a successful
native call; started-only history summarizes as unknown. Expiry/exit/cancel
still clear for safety if their audit becomes unavailable and return an
unconfirmed result. Failed OS clear remains an error; background failure is
retained for explicit retry/shutdown instead of an unbounded retry log.

`KeepAwakeService::new` injects a backend with in-memory audit;
`new_with_audit` injects a recording/failing journal. Production always uses
the fixed durable store and Windows backend.

Ownership includes core keep-awake/transport/audit/export, shared History
reader, CLI Status/History adaptation, one HistoryDomain enum addition and
focused tests. No Software implementation, GPUI, shared specs/map or task
state changes by this agent.

## Verification

Owned-file rustfmt and `git diff --check` passed. Coordinated focused tests:

- `cargo test --locked -p devsweep-core --lib keep_awake`: 14 passed.
- `cargo test --locked -p devsweep-core --lib history`: 12 passed.
- `cargo test --locked -p devsweep-cli --lib keep_awake`: 2 passed.
- `cargo test --locked -p devsweep-cli --test keep_awake_contract`: 2 passed.

These 30 test executions passed against the concurrently updated MSI/Software
source; the file-history case is included in both core filters.
The complete `just ci` / `just gpui` gates remain delegated to final checking;
this receipt does not claim they passed. Added fixtures cover
confirmation/staleness, retained endpoints,
repeated toggles, same-thread calls, independent owners, fake-clock expiry,
stop/cancel/exit, backend failures and started/terminal journal failures.
CLI cross-process tests only query/stop fake core sessions and read isolated
history; they never run production acquisition. These tests do not prove an
effective native Windows power hold or visible native review/results. No
native power request, Computer Use, host mutation, commit, push or archive
was performed.
