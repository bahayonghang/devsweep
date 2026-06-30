# Implement - size/age ranking and freshness guard

前置:本文件 review 后再执行 `python ./.trellis/scripts/task.py start 06-30-size-age-ranking`。当前阶段不要先改业务代码。

## Assumptions

- `06-30-perf-parallel-sizing` 已完成并归档,`src/fs_size.rs` 是 `estimated_bytes` / `last_modified` 的共享来源。
- 本子任务不新增 CLI 参数;7 天 freshness floor 是常量。
- 排序和默认选择应在 plan 生成层统一完成,TUI 不维护独立排序规则。

## Steps

1. **Add ranking module**
   - Create `src/ranking.rs` with `rank_cleanup_plan`, score/age helpers, and deterministic sort.
   - Add `pub mod ranking;` to `src/lib.rs`.
   - Verify: `cargo test ranking`.

2. **Freshness guard**
   - In `rank_cleanup_plan`, apply 7-day guard before sorting.
   - Only change true -> false; append `Evidence::RuleMatched { rule_id: "ranking.freshness_guard.7d" }` once.
   - Verify: tests for recent/stale/missing/future `last_modified`.

3. **Project and global scan assembly**
   - Call ranking after `dedupe_targets` in `ProjectScanner::scan_roots`.
   - Call ranking before returning from provider scan.
   - Verify: focused scanner/provider tests with known sizes and mtimes.

4. **CLI and TUI merged-plan assembly**
   - Call ranking after `run_scan` combines project/global targets.
   - Call ranking after `run_staged_scan` combines project/global targets.
   - Ensure `ScanSnapshot::targets()` or `rebuild_targets_from_scan_snapshot` also ranks the partial merged list, so progress UI does not regress to project-then-global order.
   - Verify: TUI test where a larger global target appears before a smaller project target after merge.

5. **Details/explainability check**
   - Confirm TUI Details already renders the freshness evidence line through `target_details_lines`.
   - Add or adjust a render/unit test only if needed to prove the evidence is visible.
   - Verify: existing TUI render tests pass.

6. **Contract and safety review**
   - Confirm no new fields or enum variants were added to `CleanupPlan` / `CleanTarget`.
   - Confirm scanner/provider/ranking code does not execute commands or mutate filesystem.
   - Verify: `rg "std::process::Command|trash::|remove_dir|remove_file|remove_dir_all" src/ranking.rs src/scanner.rs src/providers.rs`.

7. **Final validation**
   - Run `cargo fmt --all -- --check`.
   - Run `cargo test --all-targets`.
   - Run `just ci`.
   - If `just ci` fails, fix and rerun until green.

## Review Gate

- [ ] `design.md` and `implement.md` reviewed.
- [ ] `task.py start 06-30-size-age-ranking` has been run before code edits.
- [ ] New tests fail before implementation where practical, then pass after.
- [ ] `just ci` passes before reporting implementation complete.

## Rollback

- Ranking integration points are additive. If a late issue appears, remove the calls to `rank_cleanup_plan` first while keeping the module/tests for diagnosis.
- If freshness guard proves too aggressive, disable only the guard call and keep size sorting; do not change executor behavior.
