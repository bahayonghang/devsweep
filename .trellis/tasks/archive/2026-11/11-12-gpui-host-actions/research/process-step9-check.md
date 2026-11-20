# Independent check: step 9 process actions

Date: 2026-11-16. Scope: the user-approved `step-9-review.md` only, with the
existing uncommitted Clean and Host 3–8 work preserved. No step 10+ action was
enabled; no commit, push, archive, Computer Use or GPUI window was performed.

## Findings (fixed)

- **Output refusal happened after the CLI action.** The process termination
  adapter called the audited killer before discovering that a named output
  already existed. It now uses the existing output-absence precheck before the
  core action. The isolated CLI regression verifies output_exists with zero
  killer calls and zero journal appends.
- **Human results overstated the native observation.** The stable V1 machine
  outcome remains terminated, but the Windows API's successful request is not
  proof of observed exit. English/Chinese human results now say Termination
  requested / 已请求结束进程, matching the GUI and specification. Existing
  bilingual assertions were updated; machine output equality is unchanged.
- **Plan and journal identity validation disagreed.** A saved plan containing
  a leading-zero FILETIME could pass preview but fail production audit as
  audit_unavailable. Plan validation now rejects noncanonical decimal FILETIME
  alongside zero/sign/whitespace cases. Control characters cannot be hidden by
  basename/trim normalization. Tests assert InvalidPlan before any journal or
  killer call. Generated V1 shape and normal old plans are unchanged.
- **Global Enter could become an implicit process confirmation.** Focused
  clickable controls already propagate to their own GPUI keyboard click, so
  Review's mouse/UIA handlers prepare only. The shared root/search Enter path
  nevertheless confirmed any Ready host action. As explicitly agreed by the
  main session, process Ready now requires the labelled Confirm ending process
  control. Its mouse, keyboard click and UIA paths remain enabled. Root/search
  Enter and repeated Review produce zero killer calls; explicit Confirm then
  executes the fake once. Prior Software/Clean keyboard behavior is preserved.
- **The unbacked Hive fallback constructed unused native process authority.**
  The existing all-Hive intent gate made that authority unreachable, so no
  unauthorized host execution was observed. Step 9's default executor had,
  however, installed HostProcessControl and production audit in the temporary
  fallback. A small Hive-only factory now has no native process owner or power
  permission, and prepare/execute both refuse non-Hive intents. The existing
  isolated startup workflow remains covered.
- **Critical audit/cancellation assertions were missing.** Expanded final-live
  refusal cases cover a failed terminal append after zero killer calls, yielding
  audit_unavailable with only Started retained. A new test cancels during the
  final probe and asserts Refused/zero killer, then cancels inside the committed
  fake killer and proves its genuine result is retained. No assertion, timeout
  or coverage was weakened.
- Corrected the implementation receipt's blanket no-native-test wording to
  distinguish fake controllers from the already permitted task-owned process
  fixtures. This check does not claim no native termination occurred in tests.

## Findings (not fixed)

- No remaining product finding within this review's approved scope.
- Shared specs, task state and code map are main-session ownership. The exact
  identity, keyboard, output and Hive boundaries above were reported for sync.
- Native keyboard/accessibility/rendering and EN/zh screenshots remain
  unverified. State tests and source guards establish routing contracts, not
  native UI acceptance.
- The Python tools suite remains non-green: both full runs had the two known
  gitignore subtest failures and a skill-distribution temporary-directory
  WinError 5. Its isolated focused rerun passed, but the complete rerun reproduced
  the error. These unrelated tool/ignore issues were not modified; exact results
  and the minimal error site are in the final gate receipt.

## Verified code paths

- Production CLI and GUI use the audited core wrappers; the low-level compatible
  helper alone is not their production action route. The GUI captures a fresh
  worker-side identity before binding and live preview. Sampling ProcessV1 did
  not gain execution fields or authority, and CLI grammar stayed unchanged.
- Core plan, preview and final probe check PID, nonzero creation FILETIME and
  name. Native lookup obtains image basename and creation from one query handle,
  avoiding the previous Toolhelp-name/later-time mixture. After the final core
  name/time check, the actual terminate handle independently checks FILETIME
  before TerminateProcess. This report does not claim that the terminate handle
  queries the name again.
- Protected UI classes derive from core's closed list; the core independently
  refuses protected, missing, changed/reused and unavailable identities. No
  elevation, shell or child devsweep executable was introduced.
- Durable Started precedes the final checks/killer. Initial append failure
  blocks the killer. Final refusal is recorded as Refused; terminal persistence
  failure with no killer call remains audit_unavailable. An attempted killer
  plus failed terminal persistence becomes unknown_after_dispatch. Cancellation
  is checked immediately before the irreversible callback; no post-commit
  rollback is claimed.
- Process events use the shared fixed Status journal, short lock, flush/sync,
  real bounded read and append limit. Closed optional process details, zero
  process duration and mandatory valid digest are separate from finite power
  records without process details. Existing power serialization/readability is
  preserved. History is inspect-only; incomplete/Started-only operations stay
  unknown and never redispatch.
- Selection, search, sort, navigation and missing/renamed samples invalidate
  pending review. Ordinary telemetry preserves immutable review. Late deliveries
  cannot transfer authority or release the single-flight lock early. Finished
  results survive the next sample lacking the terminated row. Unbacked and Clean
  display fixtures cannot schedule native process actions. Software removal and
  leftovers remain refused before their later gates.

## Verification and actual coverage

- Latest focused core process_action tests: **11 passed**.
- Latest focused CLI Status tests: **12 passed**.
- The first reviewer CLI run reported **11 passed, 1 failed** because the prior
  human-result assertion still expected terminated after the wording correction.
  The assertion was corrected to requested/已请求, and the focused rerun passed.
  Full failure log:
  `C:\Users\lyh\AppData\Local\rtk\tee\1794807974_cargo_test.log`.
- Combined canonical gates, lint/type-check, GPUI counts, Python failures,
  context validation, logs and release artifact are recorded separately in
  [final-gates-step9.md](final-gates-step9.md).

Both core and CLI have an existing
fixture_owned_process_is_the_only_native_termination_target test. Each creates
and owns its ping.exe child and cleanup. The CLI fixture additionally checks a
wrong FILETIME against the actual termination handle, verifies its child stays
alive, then follows its reviewed audited CLI path using a test-only memory audit
and observes exit. The core fixture uses its existing bound helper. Focused and
canonical runs include these allowed owned-child tests; other new controllers
are fake and journals are isolated. No existing user process, installed software,
startup state or power request was changed. This is native helper coverage,
distinct from production-store end-to-end or native GUI acceptance.
