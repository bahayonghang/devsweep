# Independent C2 lifecycle and runtime review

2026-10-13. This follows the integrated C1 source/quality checkpoint in
`independent-runtime-check.md`. It does not replace C1's failed native minimize
or idle-thread observations, nor turn those failures into passes.

## Scope and findings

Reviewed `lifecycle.ts` and its tests, Planet and its lifecycle regressions,
Tauri commands/tray visibility paths, `async_runtime.rs`, native composition and
command/module inventories, the Cargo manifest/lock change, and the Planet
benchmark source hash. No additional confirmed product defect was found; the
checker made no product-source change in this C2 pass.

- `observeWindowPresentation` begins conservatively hidden in Tauri, subscribes
  before querying, rejects an initial response superseded by an event, accepts
  only literal boolean true, and releases a subscription that finishes after
  disposal. Late events/query results cannot call the disposed observer.
- Native resize/focus publishes a fresh `!is_minimized && is_visible` result;
  focus loss alone is not hidden. The query command is restricted by the existing
  main-window invoke gate. The visibility bridge has no operation-coordinator
  or scan cancellation dependency.
- Planet combines native and document visibility at start/tick/draw/resize and
  async texture completion. Minimize stops recurring callbacks while preserving
  the renderer and pose. Restore keeps the existing FPS and resets the elapsed
  baseline; async disposal and reduced-motion behavior remain intact.
- The configured Tokio runtime is built before installing the Tauri handle and
  remains a local owner across the complete `app.run`. Its async worker count
  is host parallelism capped at four, with fallback one. The separate blocking
  executor is unchanged. The regression proves dispatch still runs while a
  blocking job waits; it does not claim a measured native thread count.
- Cargo.lock only adds `tokio` to the desktop package's dependencies; no package
  entry/version is added. The accepted dependency already supplies Tauri's
  runtime. The command inventory is 49 and the adapter source inventory includes
  the new runtime module. The benchmark hash includes `lifecycle.ts`.
- New `backend/desktop-runtime.md` and current state-management guidance match
  these boundaries. The coordinator also synchronized the lifecycle adapter's
  type-safety table description to include native decorative visibility.

## Verification

Scoped `git diff --check` passed, with only Git line-ending notices. The runtime
owner first caught and repaired the missing adapter inventory entry; its failed
run remains preserved. Its subsequent desktop test/Clippy/fmt checkpoint passed.
The coordinator's full frontend gate used Node 22.23.2 / npm 10.9.8 and passed
438 Vitest tests in 58 files, five Node tests, lint, TypeScript, build and generated
type checking (`dist/frontend-c2-visibility-gate.txt`).

The independent checker runs the canonical final `just ci` with workspace-local
`dist` and the native Windows fixture token. Raw output is
`dist/just-ci-c2-independent-review.txt`; its result is recorded below.

Canonical `just ci` completed with exit code 0: format, workspace all-target
check/tests and Clippy `-D warnings` passed. Counts are CLI 191 unit plus 24
integration, core 436 unit plus one public API, and desktop 92: 744 passed total.
The existing three core and two desktop ignored entries remain ignored. Cargo
was released for C2 native build; no additional product-source change was made.

## Acceptance boundary

C2 must still prove zero recurring draw/RAF under actual native IsIconic
minimization with DOM visibility unchanged, restore/20-cycle behavior, continued
requested scan/Status progress, packaged startup and no-debug idle/process-tree
resources. It must also complete operation latency and equal-scope B3/C result
comparisons. This source review and unit tests are not native performance passes.
Keep immutable C1 evidence and the separate C2 identity; do not overwrite failed
samples or silently refresh B3 to improve a comparison.

## Read-only acceptance audit during final measurements

Reviewed the parent and all three child PRDs, the runtime design and measurement
protocol, current acceptance ledger, packaged Clean/Earth records, CLI console
and sink records, production component results, and the relevant harness/test
paths. This audit ran no tests, builds, applications or benchmarks and changed
no product source. Previously recorded source gates above remain the quality
checkpoint; planned measurements are not passes.

Two concrete omissions in the original slot plan were reported and assigned:

1. **UI-AC6 navigation latency.** The existing Planet driver measured Scan/Cancel;
   its page navigation calls were setup, not timed user navigation. The
   coordinator added a separate planned navigation replay using production
   AppShell, ScanPage and OperationCoordinator, a registered destination sentinel,
   and 100 frame-aligned samples per renderer. Click-to-post-commit RAF must meet
   both p95 <=100 ms and <=10% relative regression with matched workload/order.
   Reset/remount stays outside the timed interval. Native cancel/join remains a
   separate runtime measurement. The earlier Cancel-only data cannot close this
   navigation clause.
2. **F-AC3 denied selected root.** Packaged C2 proves missing selected paths;
   `project/mod.rs::unreadable_nested_child_keeps_sibling_targets_and_is_partial`
   proves an ACL-denied nested child. The full-scan health regression separately
   combines injected partial project health with a global target. These do not
   establish the exact selected-root ACL case with surviving healthy/global
   observations in the GUI. The coordinator assigned a bounded native fixture
   case, with an owned denied directory, original-DACL restoration and no user
   cleanup. Keep the clause unverified until that real branch is observed.

The remaining plan covers the previously identified large native/performance
work, subject to its measured results. Final evidence still needs these explicit
distinctions and links:

- RT-AC2 compares repaired Go/configured/nested/partial facts with B3. The stock
  `measure-readonly.py` Clean workloads are project-only Node/Python/overlap
  workloads. A healthy Node timing pair alone cannot establish repaired provider
  parity. Map the repaired golden cases to actual B3/C comparisons, using a small
  read-only parity batch if no existing equal-input comparison covers them;
  five timing repetitions of every provider are not required.
- Keep CLI fresh-process first-use observations separate from discarded warmups
  and retained warm pairs. OS file-cache coldness is uncontrolled. The native
  consumer's complete report can also be compared offline with a same-input CLI
  report, complementing the existing complete two-root CLI/Tauri comparison.
- The design explicitly permits disposable fixtures/fake adapters for Clean
  preview/execute wait-only evidence. Existing TUI fake cleanup teardown, shared
  coordinator join tests, frontend result/rescan tests and the production native
  CoreRescanRunner test remain distinct from actual packaged preview/execute or
  reviewed-rescan observations. Do not claim a native execution measurement or
  perform user cleanup to fill this evidence boundary.
- `native-status-lifecycle.mjs --hud` includes simultaneous live Status with the
  twenty HUD visibility cycles. Do not add a duplicate combination workload.
  Packaged C2 Clean already proved offline primary/fallback assets; C2 Earth
  already recorded actual Windows scale and twenty native route/minimize cycles.
  The DPR-only diagnosis/corrected matrix is still needed, as separately planned.
- The actual Status unread/broken pipe, named file terminal and single-envelope
  create-new observations are supported by production-channel deterministic
  bounds. A slow regular-file reader is not slow filesystem-write backpressure;
  keep that limitation and do not infer an observed exact native queue size.
- Native TUI resize remains unavailable (Windows error 87), and its one help-key
  observation is not burst p95. Deterministic draw/mailbox tests support the
  approved idle/bounded-drain contracts. Do not invent an additional native TUI
  timing budget or convert the failed console scenario into a pass. Native Unix
  acceptance remains unverified on this Windows target; it is not a newly added
  cross-platform native-run requirement.

No additional confirmed product defect was found in this acceptance audit.
Existing C1 failures, failed/provisional DPR attempts and all pending numerical
gates remain in their original evidence records.

### Navigation benchmark follow-up

The later optional navigation harness and `renderer-c2.md`/summary were reviewed
independently without running tests, builds, applications or benchmarks. No
confirmed issue was found. The start timestamp immediately precedes the real
registered Software-tab click. The destination component schedules the endpoint
at the first RAF after its effect/commit; the recorded assertion requires its
sentinel to exist and Planet to be absent. This measures the documented
post-commit presentation opportunity, not actual display scanout or native
backend work. Returning to Clean, waiting for its first draw, starting replay
and the 150 ms replay preparation all occur outside the timed interval.

Read-only recomputation confirms five discarded warmups and 100 measured
navigation samples for each build, all 200 measured destination assertions true.
Nearest-rank navigation p95 is 7.0 ms for both original and C2 (ratio 1.0);
Cancel p95 is 7.7 ms original and 7.0 ms C2. Both satisfy <=100 ms and <=1.10x
the original. The retained order is accurately described: original first for
the full run, C2 first for the separate Cancel and navigation runs.

All three raw-record SHA-256 values and teardown records match the tracked
summary. Each teardown reports natural exit 0 without forced termination.
The navigation build manifest has the documented SHA-256
`d9cb835f6bfd0f87228a88bba7df0f454858ffd0333f688e42fdc63f58751ce7`;
all 144 recorded input hashes and 14 preserved asset hashes match. Scoped
whitespace checks pass. The coordinator reports TypeScript, both Vite builds and
final scoped lint after the small `home()` readiness adjustment passed.
The benchmark-only follow-up changes no C2
packaged product source or binary and closes the component navigation omission,
not the separate native backend/performance gates.

### ACL and repaired-provider evidence follow-up

This later static review read `repaired-provider-golden-parity.md`,
`native-clean-acceptance.md`, `native-clean-acl-c2.json`, their exact raw reports
and scoped driver paths. No build, test, browser, application or scan ran.
Read-only parsing confirmed all 15 ACL evidence hashes, exact original/restored
DACL equality, unchanged sentinel hashes, and semantic equality of every field
in the native project-only report and the matching CLI report (array order
retained). Both reports include the denied selected root and healthy Node target;
the CLI returns partial exit 5. The native request explicitly has
`include_projects: true, include_global: false`.

| Acceptance part | Supported conclusion | Remaining boundary |
| --- | --- | --- |
| F-AC3 unrelated CWD, two drives, explicit roots, dedupe/nested-selection semantics, recovery | Earlier packaged C2 cases remain valid. | The ACL extension does not need to repeat the independent two-drive case. |
| F-AC3 denied selected root plus healthy project | Actual native ACL failure, surviving review row and complete matching project-only CLI report pass. | This is no longer merely a nested-child or missing-path test. |
| F-AC3 denied root plus healthy/global survivors in core/CLI | The separate isolated all-scope CLI report has the healthy Node target and task-owned Maven target, with the denied root still failed and exit 5. | The GUI run did not enable global discovery. |
| F-AC3 combined GUI denied-root/healthy-project/global-survivor case | Not verified. | Separate runs with different effective scope/provider environments cannot be composed into a passing native all-scope observation. |
| P-AC2 permission-error adapter parity | Entire native/CLI project-only reports agree under the same effective project inputs. | The all-scope CLI report has no matching native all-scope report in this new evidence. |
| RT-AC2 repaired semantics before/after runtime optimization | Six B3/C2 release CLI cases pass complete-envelope/output-byte/exit equality and independent semantic expectations: Go, configured known roots, npm, nested Python, partial roots and Yarn delegation refusal. | These are correctness cases, not provider performance or actual installed-tool configuration verification. |
| P-AC2 remaining golden categories and adapter contracts | Existing real shared-core/rule tests, deterministic adapter tests, generated wire fixtures and earlier native parity retain their stated evidence levels. New provider cases add golden coverage. | A B3-to-C2 CLI pair alone is not a CLI-to-Tauri comparison, but P-AC2 does not mandate a packaged-native run of every provider/category. Do not invent that additional matrix or declare the whole parent unacceptable from this one native gap. |

The relevant production paths remain shared: CLI Clean invokes `Sweeper` with
terminal-only observation; Tauri `CoreScanRunner` invokes the same service with
interactive observation and returns its `ScanReport`. The policy-equality,
native adapter, generated Rust/TypeScript wire and decoder/bridge tests prove
their actual deterministic contracts. They do not show that a GUI process
whose native bootstrap retained the real USERPROFILE used the CLI fixture home.
Generated wire parity establishes representation/decoding, not environmental
provider discovery. These evidence axes must remain distinct in P-AC2's final
mapping; no new product defect is inferred from their different scopes.

At this review's initial reading the tracked ACL JSON placed a CLI `global`
Maven coverage object inside `native_project_only`. The coordinator subsequently
split those CLI fields into `separate_isolated_cli_all_scope`, retaining all
original raw hashes and the explicit false
`native_combined_global_case_verified`. The separate CLI result must not be
counted as native provider coverage. Likewise, the native note's
overall Clean-pass heading means its listed cases, not a waiver of the remaining
F-AC3 clause. The parent ledger now records that clause explicitly.

No user-cache scan is authorized or needed by this review. The prior unintended
global scan remains a preserved scope failure and grants no authorization to
repeat it. Bounded investigation of an isolated native launch remains within
the implementation's existing fixture scope. If an isolated native all-scope
case cannot be established, keep the clause unverified. Accepting delivery with
that exception is an explicit user acceptance decision; provisioning a new
account/VM or changing machine settings would require a concrete separately
authorized scope. Do not substitute a broad current-user cache scan or silently
weaken the original criterion to avoid that decision.

### Rejected metadata-by-name optimization candidate

After Software's C2 native five-sample p95 missed the unchanged 5-second gate
(coordinator-reported 6.266 seconds), this reviewer statically evaluated replacing
ordinary entry inspection with `GetFileInformationByName`/
`FileStatBasicByNameInfo`. No application, benchmark, build or test ran and no
product code changed. The coordinator rejected this candidate; research stopped.

The present `filesystem/reparse.rs::inspect_windows_entry` opens each path with
`FILE_READ_ATTRIBUTES`, `OPEN_REPARSE_POINT`, backup semantics and the existing
share flags, then obtains metadata and the reparse tag. Its failure returns to
the existing metadata-plus-probe path, which preserves errors and Unverified
safety. Accepting a by-name result as Safe solely because its reparse attribute
is clear and tag is zero does not establish that the old handle access/tag query
would have succeeded. Falling back only when the new API fails cannot catch a
successful metadata query where the old access or probe would refuse.

Microsoft documents [GetFileInformationByName](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getfileinformationbyname)
as querying by path without returning an opened handle. Its
[FILE_STAT_BASIC_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-file_stat_basic_information)
page states that no specific access rights are required for that information.
That does not supply equivalence with the current requested-access handle path.
The potential ACL/share/probe classification difference is a static rejection
reason, not a claim that a native reproduction was run or that the current
product has this defect. Ordinary-file success alone would not prove preservation
of permission, partial-size and fail-closed semantics.

The pinned [CPython v3.13.0 stat implementation](https://github.com/python/cpython/blob/v3.13.0/Modules/posixmodule.c#L2026-L2054)
does use this API and conditionally falls back for reparse points and errors.
Its [dynamic loader helper](https://github.com/python/cpython/blob/v3.13.0/Include/internal/pycore_fileutils_windows.h#L38-L69)
handles missing exports. These are useful compatibility precedents for Python's
stat contract, not proof of DevSweep's stronger existing refusal contract.
Old-OS fallback or restricting acceptance to ordinary non-reparse entries would
not independently resolve the successful-query access-equivalence issue.

No new Win32 API, loader dependency or change to live execution authorization is
accepted by this review. The 6.266-second result remains a failed performance
criterion; it is not traded for weaker inspection semantics. The separately
owned scheduling investigation remains outside this candidate's conclusion.

### C3 sizing dispatch review

This follow-up is a static review of the narrow C3 scheduling candidate. No
build, test, application or benchmark ran in this review slot. Comparison with
the preserved C2 source archive found only two product changes in `sizing.rs`:
`with_max_len(1)` on the existing indexed `paths.par_iter()` and
`children.par_iter()` dispatches. The other changes add the bounded scheduling
regression. The initially reviewed source hash was
`d3d5233fab718a91422c72f3bb3bc92dd064ba328a2f2f3a628945fab34f24cb`.

No confirmed product defect was found. The existing dedicated two-worker pool,
serial fallback, no-follow entry inspection, live authority, root batching,
per-child budgets, maximum depth and cancellation checks are unchanged. The
outer indexed collection preserves caller root order. Static inspection of the
locked Rayon 1.12.0 producer/reducer shows that the new maximum grain splits
existing indexed ranges while retaining left/right order; the existing estimate
merge remains byte addition, completeness conjunction, latest-time selection
and ordered warning concatenation. Scheduling can change how much work finishes
before a concurrent cancellation, as before; this is not a promise of identical
partial prefixes under races.

The retained before log reproduces the concrete blocked-fold problem: only 48
of 63 unblocked root siblings were visited before the explicit gate release.
The initial after log reports all 24 sizing tests passing, including worker
ceiling, result ordering, budgets, cancellation, denied/cloud/reparse handling
and the new sibling fixture. These are inspected implementer results, not tests
rerun by this reviewer and not a native Software performance result.

Review found one proof gap in the new test: its 10-second emergency gate timeout
was initially ignored, so a sufficiently delayed observer could mistake visits
after automatic timeout for progress before explicit release. The implementer
now records `gate_timed_out` and asserts it is false only after setting release,
notifying, dropping the lock and joining the worker. This preserves bounded
failure cleanup while rejecting that false pass. The reviewed corrected source
hash is
`964f3fca1ff40b38ac32ddd4bb6fb862c5bc32ec60b419080061987e7e9017d3`.
The earlier 24-test pass predates this test-only correction; the later full
canonical gate below verifies the final guarded test.

The new controlled fixture directly exercises `estimate_root_children`, not the
outer multiple-root dispatch. Existing multi-root coverage checks results and
order, and both changes use the same reviewed indexed-dispatch contract. The
coordinator explicitly chose not to add a second blocked multiple-root fixture
in this narrow iteration; this coverage boundary is retained rather than
described as directly reproduced. No new Win32 API or permission shortcut is
accepted. All C2 measurements and the failed C2 Software gate remain preserved;
C3 source review does not establish that the unchanged native performance gate
passes.

### C3 canonical gate and fixture isolation

The coordinator subsequently assigned an exclusive Cargo slot for the latest
canonical `just ci`, using workspace-local `dist`, locked dependencies and the
existing Windows native-fixture execution context. Its first run failed only
`scanner_finds_marker_backed_project_targets` with `missing rust.target`.
That full failure is retained in
`dist/just-ci-c3-independent-review-before-fixture-isolation.txt`.

This was diagnosed rather than dismissed as a flaky scheduler test. The same
test binary and exact test failed with `CARGO_TARGET_DIR=dist` (0.14 seconds),
then passed when only that test process omitted the variable (0.13 seconds).
The fixture creates `rust-app/target`, while the real inherited Cargo metadata
query resolves `rust-app/dist`; the scanner correctly declines that nonexistent
directory. The existing quality guidelines already explain this distinction and
prefer Cargo's `--target-dir` argument for general validation isolation. C2's
earlier successful gate did not prove this fixture robust to every inherited
Cargo environment.

With the coordinator's explicit approval, the reviewer changed only this
`cfg(test)` function in `scan/project/mod.rs`, following the neighboring Rust
target test's existing self-subprocess pattern. The fixture child removes
`CARGO_TARGET_DIR`; the outer process and production provider environment remain
unchanged. Every original marker assertion remains, and a new canonical-path
assertion requires the Rust target to equal the fixture's own `rust-app/target`.
The exact regression now passes under the original relative override. Raw
diagnostics are `dist/c3-marker-target-with-override.txt`,
`dist/c3-marker-target-without-override.txt` and
`dist/c3-marker-target-isolated-guard.txt`. This is test isolation, not a change
to scanner authority or a scheduler product fix.

The subsequent complete `just ci` passed with exit 0 in the same assigned
environment: format, locked workspace all-target check/tests and Clippy with
`-D warnings`. Counts are CLI 191 unit plus 24 integration, core 437 unit plus
one public API, and desktop 92: **745 passed, zero failed, five existing ignored**.
The guarded sibling scheduling test is included. Scoped source whitespace checks
also pass. No dependency or production-source change was made by this gate fix.

Final checked identities:

- `sizing.rs`: `964f3fca1ff40b38ac32ddd4bb6fb862c5bc32ec60b419080061987e7e9017d3`.
- `scan/project/mod.rs`: `90b1ed3130aba66d2cee1f2a1ad147fffe260067cb1fb2702285acc630195484`.
- `dist/just-ci-c3-independent-review.txt`: `2ab48f1a141e17020e13ce078fd886cb4d0850704e4a698763cf8ac3333a39ce`.

Cargo was released to the runtime owner for the C3-final source archive and
native build. The native Software performance gate and final C3 parity remain
separate acceptance work; this successful source gate does not convert C2's
failed measurement into a pass.
