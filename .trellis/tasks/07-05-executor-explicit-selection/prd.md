# Executor 显式选择接口

## Goal

在 `ExecutionRequest` 中加入显式的已选目标列表，让"执行哪些 target"作为数据穿过接缝，终止 TUI 克隆 target 并强写 `selected_by_default` 的旗标走私；自清理守卫收拢为 executor 内的单一强制点。

## 背景（评审证据）

- Executor 的接口契约是"执行所有 `selected_by_default == true` 的 target"（executor.rs:171–175 过滤）。
- TUI 的 `selected_cleanup_plan`（tui.rs:969–983）为了驱动执行器，克隆 targets 并把用户勾选的项强写 `selected_by_default = true` —— 把一个**扫描期排序提示字段**偷运成**执行期选择信号**，跨接缝耦合走的是共享可变旗标而非显式数据。
- 自清理不变量（不删除运行中可执行文件所在目录）被强制两次、机制不同：
  - `tui::default_selected_ids`（1473–1480）在选择期过滤 `!target_contains_current_exe(...)`
  - `executor::execute_target`（257–261）在执行期再跳过一次
  - 两处各有一个测试（`scan_finished_default_selection_skips...` 与 `target_containing_running_executable_is_skipped_before_command_runs`），但对纯函数的单测恰恰测不出"同一不变量两个强制点"这种耦合问题。

## Requirements

- `ExecutionRequest` 增加显式选择字段（如 `selected: Vec<TargetId>` 或等价形状，具体在 design.md 定夺），executor 以它为准决定执行集合。
- 删除 TUI 的克隆-强写路径：`selected_cleanup_plan` 的 clone-and-mutate 消失，TUI 只维护自己的 UI 选择状态并在提交时传出 id 列表。
- `selected_by_default` 回归其名义语义：ranking 产出的扫描期默认勾选提示，rank 之后只读。
- 自清理守卫只保留 executor 一个强制点；TUI 的 `default_selected_ids` 退化为纯 UI 默认值计算（可以继续参考 `selected_by_default`，但不再承担安全职责）。
- CLI 路径（`main.rs::run_clean`）同步适配新的 ExecutionRequest 形状：默认执行集合 = 所有 `selected_by_default` 的 target id，行为不变。

## Acceptance Criteria

- [ ] `ExecutionRequest` 携带显式选择；executor 不再以 `selected_by_default` 作为执行过滤条件
- [ ] tui 中不再有对 `selected_by_default` 的写操作（`grep -n "selected_by_default = " src/tui*` 为空）
- [ ] `target_contains_current_exe` 在生产代码中只有 executor 一个调用点
- [ ] executor 测试直接覆盖选择语义：空选择、全选、含自身目录的选择被跳过
- [ ] CLI `devsweep clean` 与 TUI 清理的对外行为无回归
- [ ] `cargo test` 全量通过；CleanupPlan serde 格式不变

## 约束与顺序

- 无硬依赖；建议在 07-05-tui-module-split 之后执行，改动落点集中在 `tui/app.rs` 与 executor，diff 更干净。
- executor 是全库测试最好的深模块，改动其接口须保持 `CommandRunner`/`TrashRunner` 注入接缝不变。
- 复杂度中等：`task.py start` 前至少补 design.md（选择字段的形状与向后兼容权衡）；implement.md 视 design 结论决定。

## Notes

- 词汇约定沿用 /codebase-design：让**接口**陈述契约（选择即数据），删除旗标走私；一个不变量一个强制点是 locality 的直接收益。
