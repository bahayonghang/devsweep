# Step 9 implementation boundary

User approval: 2026-11-16, `step-9-review.md`. This stage ends before step 10.

## Gap and ownership

The existing process helper already binds PID/creation FILETIME and checks the
handle before termination, but the CLI has no durable action journal and GPUI
only explains sampled rows. Core owns identity, authorization and audit;
GPUI must retain its plan/digest and request a separate confirmation.

Expected changes: core `status/process_action.rs`, shared Status audit and
History; CLI Status adapter/presentation and focused bilingual contracts;
GPUI host review, Session and Status row detail/actions with fake tests.
Shared specs, task lifecycle and `code_map.md` remain main-session ownership.

## Contract choice before coding

- Preserve existing V1 saved process plan/preview/report and CLI grammar.
- Resolve a fresh process identity on the background worker before binding a
  plan; sampled `ProcessV1` has no creation FILETIME and is not authority.
- Reuse `status.jsonl` and its short lock/bounded validation. Add a process
  action and optional closed process details to V1 Status audit records;
  old keep-awake records keep their exact serialization and meaning.
- Provide an audited core termination wrapper shared by CLI/GPUI, retaining
  the existing bound helper. Durable Started precedes the final identity check
  and killer; terminal persistence failure reports unknown after dispatch.
- Protected UI state derives from core's closed classification. Selection,
  search, navigation and missing/renamed sampled rows invalidate review;
  fresh core rechecks also catch same-name PID reuse. Telemetry changes alone
  do not authorize or silently alter the immutable reviewed plan.

No new CLI flags, process sampling schema, elevation, shell command,
uninstall/removal/leftover activation or unrelated refactor. Controller tests
use fake probes/killers and isolated journals. The existing task-owned native
process fixture is separately described below; no existing user process was a
target. No Computer Use, commit, push or archive. Outcomes follow below.

## Implemented contracts

- Added `terminate_bound_process_audited(plan, digest, confirm, probe, killer,
  audit)` and its additive `_cancellable(..., cancel)` entry. CLI uses the former;
  GPUI passes its job observer to the latter. The original low-level helper and
  V1 saved plan/preview/report shape remain compatible.
- `ProcessActionAudit`/`FileProcessActionAudit::production()` share the fixed
  Status store and its locking, bounded read/append and flush/sync implementation.
  `StatusActionAuditAction::ProcessTerminate` (`process_terminate`) requires
  optional closed `process: {pid, creation_time_filetime, name, error_code?}`,
  duration zero and a SHA-256 preview digest. Power actions require no process
  object and retain finite duration. The original action type remains an alias.
- Confirmation, plan, live identity and digest validate before Started. The
  final core and native-handle checks still run after durable Started. Canceled
  pre-dispatch actions record Refused when Started exists. Terminal persistence
  failure after an actual killer call reports unknown; if final checks prevented
  any killer call, an undurable refusal reports audit unavailable. History with
  only Started remains unknown in either case; nothing redispatches it.
- Native lookup now reads image basename and creation time from one query-only
  handle, removing the old Toolhelp-name/later-PID-time mismatch. The killer
  separately checks the actual opened terminate handle's FILETIME. Zero FILETIME
  and control characters in identity names are invalid.
- GPUI's selected-row action prepares only. Shared fixed confirmation displays
  the actual bound name/PID/creation time and bilingual unsaved-data warning.
  Core classification disables protected actions with a reason. Search, sorting,
  selection, navigation and missing/replaced sample rows invalidate review;
  unchanged identity telemetry preserves it. Stale deliveries cannot transfer
  authority or unlock execution early. Finished results are historical and are
  not erased merely because the next sample no longer lists the process.
- CLI human preview includes the reviewed name and digest, localizes the loss
  warning and error explanation, and retains locale-neutral JSON. Human History
  shows the process identity and stable failure reason. Grammar is unchanged.

## Verification performed by implementer

All Cargo operations ran serially. Final focused results:

| Check | Actual result |
| --- | --- |
| `cargo test --locked -p devsweep-core process_action --lib` | 9 passed, 644 filtered, 0.03 s; four root test threads |
| `cargo test --locked -p devsweep-cli application::commands::status --lib` | 12 passed, 203 filtered, 2.14 s; four threads |
| GPUI `cargo test --locked ... --target-dir desktop-gpui/dist/goal-gpui-test process_` | 8 passed, 230 filtered, 0.19 s; one thread, canonical `/Brepro` flags |
| Core+CLI all-target Clippy, `-D warnings` | Passed |
| GPUI all-target Clippy, canonical `goal-gpui-check`, `-D warnings` | Passed |
| Root and GPUI `cargo fmt ... -- --check`; `git diff --check` | Passed |

The first root Clippy attempt found an unused import in the new test/production
adapter split. It was removed; the later all-target run passed without allowances.

Coverage includes unconfirmed/missing or wrong digest, UI and core protection,
vanished/reused/renamed identity, late drift after Started, backend failure,
Started and terminal journal failure, cancellation during Started persistence,
legacy power/new process history, malformed action fields and partial final
lines, bilingual warning/results, stable machine output, fixture isolation,
single-flight and stale delivery. Shared Status appends now also refuse a write
that would exceed the existing 16 MiB bounded store.

### Actual native helper coverage and limits

The existing CLI test `fixture_owned_process_is_the_only_native_termination_target`
creates its own hidden-output `ping.exe` child and owns cleanup. This run added
and passed a direct wrong-FILETIME refusal against that child's actual terminate
handle, checked it remained alive, then completed its existing reviewed CLI
termination and observed exit. It uses a test-only memory audit. This is an owned
process fixture, not an operation on an existing user process; the other new
controller tests use fake probes/killers and isolated journals.

No user process was ended, no Computer Use or GPUI window was started, and no
uninstall/update/startup/power action ran. This does not establish native GPUI
keyboard/accessibility/rendering or screenshot acceptance. Full `just ci`,
`just gpui`, Python tools tests and independent Trellis review belong to the next
checker stage. Step 10 remains unopened; no commit, push or archive was made.
