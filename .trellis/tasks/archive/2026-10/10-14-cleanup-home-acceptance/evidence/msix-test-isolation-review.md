# Existing MSIX worker-count test race

Date: 2026-10-14. Reviewer: `backend_contract_check` (`trellis-check`).

## Cause and before evidence

The parent post-Node-fix canonical gate (`dist/cleanup-home-final/gates/just-ci-node-fixed.log`) failed only `software::msix::overlap_tests::enumeration_panic_keeps_the_existing_joined_mta_failure`: its own `run_joined_mta` returned the expected `mta_worker_panicked`, but the subsequent process-wide `active_mta_workers() == 0` assertion observed 1. The core result was 479 passed / 1 failed / 3 ignored.

`ACTIVE_MTA_WORKERS` counts every MTA worker in the test process. `run_joined_mta` correctly joins its own worker before returning; it does not join unrelated concurrent test workers. Several independently parallel tests asserted that the entire process counter was zero without isolating the observation. The Node dispatch repair did not change MSIX production or test code before this investigation.

Before this repair, a focused eight-thread run of just the four MSIX overlap tests reproduced the problem without native inventory: **2 passed / 2 failed**. `streamed_panic_is_resumed_after_join_even_when_canceled` observed 2 active workers and `late_dependency_and_partial_source_are_finalized_after_streamed_measurement` observed 1. Raw output: `dist/cleanup-home-final/gates/msix-overlap-before.log`.

## Bounded test-only repair

Added `mta_test_guard()` under `cfg(all(test, windows))`, with one shared mutex held through each test's post-join assertions. All real MTA callers in this core test binary participate: three overlap worker tests, the current-process-SID/native-inventory test, and both ignored native diagnostics. The cancellation-only overlap case creates no MTA worker and does not inspect the counter. Execution tests use `SoftwareExecutor::for_test` with fake adapters and inventory; they do not start native MTA workers. Separate integration-test binaries do not share this static counter.

No production MTA worker, counter, WinRT lifecycle, cancellation, joining or error propagation changed. The exact zero-worker and panic assertions remain. Global test threading is not reduced; only tests observing this shared counter are isolated. Benchmark timing starts after acquiring the test guard.

Changes are confined to test-only code in `software/msix.rs`, one test guard in `software/mod.rs`, and this evidence. A final source freeze must include these test edits, but release production behavior is unchanged.

## Verification

After the parent's release build finished, ran:

- `cargo test --locked --offline --target-dir dist -p devsweep-core --lib -- --test-threads=8 software::msix::overlap_tests software::windows::current_process_sid_is_exact_and_mta_worker_joins`: **5 passed**, on the normal Windows host. This retains eight libtest threads and includes the real read-only native inventory/SID test alongside all four overlap tests. Raw output: `dist/cleanup-home-final/gates/msix-overlap-green.log`.
- `cargo clippy --locked --offline --target-dir dist -p devsweep-core --all-targets -- -D warnings`: **PASS**, including test compilation. Raw output: `msix-isolation-clippy.log` in the same gates directory. Existing incremental hard-link fallback warnings did not change the successful exit status.
- `cargo fmt --all -- --check`: **PASS**.
- `git diff --check`: **PASS**.

The two opt-in native diagnostics remain ignored; this repair does not run or claim their performance evidence. The parent owns the final canonical gate and source-manifest rerun. Reviewer Cargo work has finished.
