# Independent Check Report

## Reopen verification - 2026-08-31

Overall verdict: **PASS** for all four parent-acceptance gaps.

### Per-gap verdicts

| Gap | Verdict | Independent evidence |
| --- | --- | --- |
| 1. ACL-denied no-follow classification | **PASS** | `NativeFs::metadata` maps a permission-denied handle probe paired with `PathSafety::Unverified` to `FsFail::AccessDenied`. `mark_fail` produces incomplete evidence and the access-denied warning with zero bytes; the failed entry never reaches `schedule_children`. Other unverified probe failures remain conservative reparse leaves and are not traversed. The focused debug/release suites and the native ACL branch passed. |
| 2. Sparse-file coverage | **PASS** | The fake adapter covers an 8 GiB logical sparse file without allocating content. The Windows native test creates an 8 GiB sparse file through `FSCTL_SET_SPARSE` and verifies complete logical bytes. Both tests passed in debug and release runs. |
| 3. Native live churn and refreshed 50k gate | **PASS** | The Windows churn test queues 6,000 stable files before deletion/growth observations, distinguishes deleted entries as unknown churn and growing entries as incomplete churn lower bounds, verifies an actual ACL-denied zero-byte branch, and reconciles every represented directory from child lower bounds. It passed three additional independent repetitions. The release gate passed with 51,007 nodes, 9,095,340 accounted bytes, two warnings, two workers, and 2 ms independent p95 cancel-to-join; the implementer evidence records the same node/accounting contract and a 3 ms p95. |
| 4. CLI/TUI/desktop identical-fixture parity | **PASS** | `reopen-parity-cli.log` exactly matches the checked-in CLI JSON fixture. The TUI test deserializes that same file through the core DTO and applies the real reducer. The desktop test uses the real decoder and selectors. All agree on 7 nodes, 8,388,616 lower-bound bytes, 5 complete nodes, 2 incomplete nodes, one access-denied warning, and snapshot SHA-256 `d14a6c33214dbb4a19527b6184411b68b02b90bec639fb9d8c94e08f7a3e1c36`. |

### Contract and safety audit

- No cleanup authority, action conversion, deletion, trash, or process execution entered the changed production Analyze paths.
- No DTO/model, cap, CLI grammar/module-root, generated type, dependency, or package-lock change is part of the reopened implementation. The frozen limits remain 250,000 nodes, 10,000 warnings, 268,435,456 accounted bytes, and two workers.
- Denied entries are represented conservatively and never traversed. Non-denied unverifiable reparse probes remain non-traversed leaves.
- Protected dirty files `.trellis/.gitignore`, `README.md`, and `justfile` were not edited by this reviewer.

## Findings (fixed)

No implementation defect required a production-code change. This review added only the requested independent logs, this report, and `check.jsonl` journal entries.

## Findings (not fixed)

No functional blocker remains for the four gaps.

Two evidence-hygiene notes are non-blocking:

- The generator is deterministic for logical structure, regular-file bytes, sparse logical length, ACL payload, and collision refusal. The frozen snapshot hash is intentionally a hash of the recorded CLI DTO; absolute root identity and filesystem mtimes make that raw DTO hash host/run specific.
- The recorded release binary hash fields agree with each other, but the independent release rebuild replaced the binary at the same `target/release` path. The current path therefore no longer hashes to the older evidence value. The recorded log/status assertions remain internally consistent, and the independent release suite passed from the current source.

## Verification

| Command or audit | Exit | Result / log |
| --- | ---: | --- |
| `cargo test -p devsweep-core analysis` | 0 | 15 passed; `reopen-independent-core-analysis.log` |
| `cargo test -p devsweep-core analysis --release -- --nocapture` | 0 | 16 passed; native 50k gate included; `reopen-independent-core-analysis-release.log` |
| `cargo test -p devsweep-cli analyze` | 0 | 12 passed, including shared CLI fixture through the TUI reducer; `reopen-independent-cli-analyze.log` |
| `cargo fmt --all -- --check` | 0 | pass; `reopen-independent-fmt-check.log` |
| `git diff --check` | 0 | pass; line-ending warnings only; `reopen-independent-git-diff-check.log` |
| `just ci` | 0 | format, check, workspace tests, and clippy passed; `reopen-independent-just-ci.log` |
| `npm --prefix desktop run typecheck` | 0 | pass; `reopen-independent-desktop-typecheck.log` |
| `npm --prefix desktop run test -- src/modes/analyze/parity.test.ts` | 1 | sandbox `spawn EPERM`; `reopen-independent-desktop-parity-test.log` |
| Main-session desktop Vitest evidence audit | 0 | 19 files / 122 tests passed, including `parity.test.ts`; `reopen-desktop-test-main-session.log` and `reopen-independent-parity-audit.log` |
| Node fallback using real desktop decoder/selectors | 0 | parity totals and hash passed; `reopen-independent-parity-desktop.log` |
| Native churn test, three repetitions | 0 | all three passed; `reopen-independent-native-churn-repeat.log` |
| Native release evidence/status audit | 0 | recorded samples/status aligned; `reopen-independent-native-evidence-audit.log` |
| Scope/authority/cap audit | 0 | no forbidden contract/dependency/authority change; `reopen-independent-scope-audit.log` |

### Fixes applied

- Production code: none.
- Evidence/reporting: added independent raw logs, parity/native/scope audit logs, this reopen-verification report, and reading-journal entries.

### Blockers

None. The reviewer-local Vitest launch is sandbox-blocked, but the required checked-in parity test is independently covered by the audited main-session 19-file/122-test pass and by a no-spawn run through the same decoder and selectors.
