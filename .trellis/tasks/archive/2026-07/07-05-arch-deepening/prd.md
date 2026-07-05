# 架构深化：收拢顶层扫描管线与接缝

## Goal

按 2026-07-05 `/improve-codebase-architecture` 评审结论，对 devsweep 顶层架构做深化重构。评审发现：底层模块群（executor、providers、scanner、ranking、fs_size、path_safety）是接口小、行为多、I/O 处有可注入 adapter 的深模块，测试充分；所有架构摩擦集中在顶层边缘。本父任务持有需求集与任务地图，负责跨子任务验收和最终集成评审，自身不承担实现工作。

## 需求来源（评审发现的四类摩擦）

1. **扫描管线无所有者**：scan→merge→rank 管线在 `main.rs::run_scan`（24–37）与 `tui.rs::run_staged_scan`（135–187）各手工拼装一遍；`rank_cleanup_plan` 在 scanner、providers、staged-scan、ScanFinished、ScanSnapshot::targets 共 5 处被防御性调用，甚至有专门的幂等性测试自保。
2. **tui.rs 单文件 3834 行**：接口只有 `run()` 一个符号，内部却混装六种关注点（终端生命周期、线程运行时、reducer、TUI 领域类型、渲染、测试）；线程化 runtime 直接构造 `ProjectScanner::new()` / `Executor::default()`，30 个测试全部绕开它。
3. **选择信号走私过接缝**：TUI 克隆 target 并强写 `selected_by_default`（扫描期字段）来驱动执行器；"不删除运行中可执行文件所在目录"这一不变量在 `tui::default_selected_ids` 与 `executor::execute_target` 两处用两种机制各实现一遍。
4. **规则身份无局部性**：`rule_catalogue()`（rules.rs:314–417）手工复述 scanner.rs / providers.rs 中过程式规则的 id、风险级、摘要，靠 `project_dir_rules_match_legacy_scanner_ids` 测试防漂移；id/scope/risk 行格式化在 `main.rs::run_rules` 与 `tui::render_rules` 重复。

## 任务地图与执行顺序

| 子任务 | 优先级 | 依赖 |
|---|---|---|
| 07-05-sweep-pipeline-owner | P1 | 无 — **先行**，其余子任务的基础 |
| 07-05-tui-module-split | P1 | 依赖 sweep 落地（run_staged_scan 整体迁出） |
| 07-05-executor-explicit-selection | P2 | 无硬依赖；建议在 tui 拆分后做，改动落在 tui/app 更干净 |
| 07-05-rules-identity-locality | P2 | 独立，可并行 |

## 跨子任务验收标准

- [x] `rank_cleanup_plan` 在整个代码库只有 1 处生产调用点（sweep 模块内部）
- [x] `main.rs` 与 `tui` 不再各自拼装 scan→merge→rank 管线
- [x] `tui.rs` 单文件消失，替换为 `tui/` 模块目录，对外接口仍为 `run()`
- [x] 自清理守卫（target_contains_current_exe）只有 executor 一个强制点
- [x] 每条过程式规则的 RuleDoc 与其实现同处声明，无第二份手写副本
- [x] 全量 `cargo test` 通过；`cargo clippy` 无新增告警
- [x] CLI（scan/clean/rules）与 TUI 行为对外无回归

## 约束

- 这是纯重构：不新增用户可见功能，不改变 CleanupPlan 的 serde 格式（CLEANUP_PLAN_VERSION 不变）
- 每个子任务独立可验证、独立可提交；不跨子任务合并大 diff
- 底层深模块（executor/providers/scanner/ranking/fs_size/path_safety）的公共接口非必要不动

## Notes

- 评审报告：`C:\Users\lyh\AppData\Local\Temp\architecture-review-20260705.html`（临时文件，需求已完整收录于本 PRD 及各子任务 PRD）
- 词汇约定沿用 /codebase-design：module / interface / depth / seam / adapter / leverage / locality
