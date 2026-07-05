# 新建 sweep 模块：扫描管线唯一所有者

## Goal

新建深模块 `sweep`，接口为 `full_scan(scope, progress) -> CleanupPlan`，收拢 scan→merge→rank 管线，使"返回的计划已排序且只排序一次"成为模块内部持有的不变量。消除 `main.rs` 与 `tui.rs` 的重复拼装及 5 处 `rank_cleanup_plan` 防御性调用。

## 背景（评审证据）

- `main.rs::run_scan`（24–37）与 `tui.rs::run_staged_scan`（135–187）各自独立完成：选 scope → 跑 `ProjectScanner::scan_roots` + `GlobalProviderScanner::scan` → `extend` 合并 targets → `rank_cleanup_plan`。管线形状是复制出来的，没有共享的所有者。
- `rank_cleanup_plan` 现有 5 处调用：scanner.rs:39、providers.rs:99、tui.rs:185（staged-scan）、tui.rs:569（ScanFinished）、tui.rs:1220（ScanSnapshot::targets）。freshness guard 专门写成幂等并配有幂等性测试——这是作者知道它被重复调用的防御性证据。
- scanner 与 providers 在内部调用 ranking，导致"扫描器无法返回未排序的计划"，而调用方又不信任这一点、再排一次。"计划何时被排序"没有单一所有者。
- TUI 的 `run_staged_scan` / `run_scan_worker` 直接构造 `ProjectScanner::new()`、`GlobalProviderScanner::new()`、`std::env::current_dir()`，无注入点，分阶段合并逻辑完全无测试覆盖。

## Requirements

- 新建 `sweep` 模块（`src/sweep.rs`），拥有完整的 scan→merge→rank 管线；接口保持最小：一个入口函数 + 进度回调（供 TUI 分阶段展示复用，CLI 可忽略进度）。
- `scanner::scan_roots` 与 `providers::scan` 不再各自调用 `rank_cleanup_plan`，回归纯扫描职责（返回未排序结果）。
- `main.rs::run_scan` 与 TUI 的 staged-scan 均改为调用 `sweep`，删除各自的手工拼装。
- TUI 侧 ScanFinished / ScanSnapshot 中的重复 rank 调用一并移除；最终全库生产代码只剩 sweep 内部 1 处 rank 调用点。
- 进度回调设计需覆盖现有 TUI 分阶段扫描的事件粒度（project 扫描阶段、provider 扫描阶段、增量更新），不丢失现有用户可见的进度体验。
- sweep 的依赖（两个扫描器）可注入，使管线编排本身可在测试中用假扫描器驱动。

## Acceptance Criteria

- [ ] `grep -rn "rank_cleanup_plan(" src/` 生产代码仅 1 处调用（sweep 内部；ranking.rs 自身定义与测试除外）
- [ ] `main.rs` 与 tui 中不再出现 `ProjectScanner` + `GlobalProviderScanner` 的并列拼装
- [ ] sweep 模块有自己的测试：用注入的假扫描器验证合并顺序、rank 恰好一次、进度事件序列
- [ ] TUI 分阶段扫描的对外行为（进度展示、增量刷新）无回归
- [ ] `cargo test` 全量通过；ranking 的幂等性测试可保留但不再是正确性的依赖
- [ ] CleanupPlan serde 格式不变（CLEANUP_PLAN_VERSION 不变）

## 约束与顺序

- 本任务是父任务 07-05-arch-deepening 下其余子任务的先行基础；07-05-tui-module-split 依赖本任务完成（run_staged_scan 将整体迁出 tui.rs）。
- 复杂任务：`task.py start` 前需补齐 design.md（接口形状、进度回调契约、注入方式的权衡）与 implement.md。

## Notes

- 词汇约定沿用 /codebase-design：本任务的核心是把管线的**不变量**（rank 恰好一次）移入**实现**，让两个入口共享一个**接口**——leverage 归调用方，locality 归维护者。
