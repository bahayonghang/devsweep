# C03 Implementation and verification

Status: 2026-10-19 checks passed on rustc 1.99.0. This is not an MSRV 1.88.0 pass. C03-AC1 through C03-AC4 are checked. A real cloud-files reparse tag stays NOT_RUN inside C03-AC1 and is not PASS. Dependencies C01 and C02 are archived. The 2026-10-17 section below is historical. Its "AC3 stays open" and "Recycle Bin was not used" sentences are superseded by `evidence/c03-recycle-round3.md`.

## Sequence

1. Verify approval of the parent final-review.md; platform scope is already confirmed.
2. Verify listed dependency acceptance and load curated manifests. Complete this
   child's technical decisions before task.py start or code dispatch.
3. Assign exact file ownership; implement only the requirements in this PRD.
4. Add meaningful successful and rejected identity/partial/failure/lifecycle cases
   appropriate to the boundary; do not add tests that only repeat implementation.
5. Run targeted checks below and required broader gates; repair relevant failures.
6. Record each C03-AC result with source/binary identity and evidence level.
7. Dispatch independent Trellis check review, update implemented-contract specs,
   and hand accepted contracts to dependents. Commit/archive need separate approval.

## Checks

- `cargo test --locked -p devsweep-core`
- `just ci`
- Isolated cache-family golden reports and fixture-preservation hashes

Use [the parent protocol](../10-15-gpui-desktop-rewrite/research/performance-protocol.md)
for applicable native measurements. Freeze approved budgets and exact scopes before
candidate work. A failed result never authorizes a relaxed threshold.

## Evidence and stop conditions

- Map every requirement to the acceptance checklist in prd.md.
- Use isolated roots, test registry state, disposable apps and owned processes.
- No real user-cache cleanup, unrelated app uninstall or host-wide repair for tests.
- Keep failures and NOT_RUN/UNVERIFIED platform/hardware results explicit.
- Failed required dependency evidence blocks downstream integration.
- Preserve current rollback behavior until C14 accepts migration.

## Recorded checks (2026-10-17)

Not accepted. No commit.

| Command | Exit | Log |
| --- | --- | --- |
| `cargo test --locked -p devsweep-core --target-dir dist/ci-target` | 0 | `C:\Users\lyh\AppData\Local\Temp\grok-goal-8f19d0d75913\implementer\c03-core-test.txt` |
| `just ci` | 1 | `C:\Users\lyh\AppData\Local\Temp\grok-goal-8f19d0d75913\implementer\c03-just-ci.txt` (first run, before the separator; clippy `needless_lifetimes` in `windows_cache_tests.rs`) |
| `just ci` | 0 | `C:\Users\lyh\AppData\Local\Temp\grok-goal-8f19d0d75913\implementer\c03-just-ci.txt` (rerun appended; ends `ci complete`) |
| `cargo test --locked -p devsweep-core --lib execution::tests::windows_cache_pipeline::verified_freed_bytes_after_isolated_rename -- --test-threads=1` | 0 | `C:\Users\lyh\AppData\Local\Temp\devsweep-c03-freed.txt` |
| `cargo test --locked -p devsweep-core --lib rules::windows_caches::tests::user_temp_foreign_sid_is_ownership_unknown -- --test-threads=1` | 0 | `C:\Users\lyh\AppData\Local\Temp\devsweep-c03-owner.txt` |

The core log shows `windows_cache_pipeline::real_scan_saved_plan_preview_and_recorded_execution` and `catalogue_omissions_links_and_distinct_totals` passed. Lib result in that log: 495 passed, 0 failed, 3 ignored.

AC2 is checked because that log shows a global scan and a project-only scan in the omission case, and the unsafe-root scenario keeps a healthy sibling while health is not complete.

AC3 stays open. The distinct case still keeps estimated bytes, scan `verified_bytes`, and history `known_bytes` as separate JSON fields, and the recording runner still leaves fixtures in place. `verified_freed_bytes_after_isolated_rename` exited 0: production `run_plan` renamed the validated path into `isolated-trash` under the fixture root, the source was gone, and the renamed length was 17 bytes. The report action is `move_to_trash` and the JSON has no `delete_permanently`. History `known_bytes` for that move is also 17, so the measured length is not a third stored field. The Recycle Bin was not used: NOT_RUN.

AC4 is checked because `coverage-manifest.md` lists the same six trash ids and eight omission ids the passing omission case asserts. It does not claim a Mole rule count.

AC1 is checked. Positive, adjacent personal data, busy, reparse, hardlink, and stale-preview cases passed for all six trash ids. Unknown owner passed for the five application families in scenario `unknown`. `user_temp_foreign_sid_is_ownership_unknown` exited 0: `discover` for `UserTemp` with an expected LocalSystem SID left the stale file out of candidates and recorded `ScanCoverageStatus::OwnershipUnknown` with `temporary entry is not proven owned by the current user`; dropping the override still allowed that owned file as a candidate. Passing `OwnerState::Unknown` is not this proof. A real cloud-files reparse tag was not created; the junction and hardlink cases cover the required reparse/hardlink alternative. The Recycle Bin stays NOT_RUN.

## Recorded checks (2026-10-19)

`cargo test --locked -p devsweep-core --target-dir dist/ci-target` exited 0. Lib result: 564 passed, 0 failed, 3 ignored. Public API 2 passed. Doc-tests 4 passed. The four named tests `verified_freed_bytes_after_isolated_rename`, `user_temp_foreign_sid_is_ownership_unknown`, `real_scan_saved_plan_preview_and_recorded_execution`, and `catalogue_omissions_links_and_distinct_totals` are ok. rustc 1.99.0. This is not an MSRV 1.88.0 pass.

`just ci` was not rerun. It exited 0 on 2026-10-19 on this crate tree, recorded before commit `3b20b49`. `git diff --stat 3b20b49 HEAD -- crates` is empty.

The real Recycle Bin measurement remains `evidence/c03-recycle-round3.md` (scan, plan, preview, and execute each exited 0; shell count 201 to 202). A real cloud-files reparse tag stays NOT_RUN.
