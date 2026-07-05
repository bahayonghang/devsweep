# Implement: Executor 显式选择接口

按 design.md 执行；每步验证后前进。

## 步骤

1. **model.rs**：`CleanupPlan::default_selected_ids()` 只读辅助 + 单测（按序、只含 selected_by_default）。
   - 验证：`cargo test model`
2. **executor.rs**：`ExecutionRequest.selected` 字段；交集过滤替换 flag 过滤；新增/迁移测试（design.md 测试设计 1–5）。
   - 验证：`cargo test executor`
3. **main.rs::run_clean**：传 `selected: plan.default_selected_ids()`。
   - 验证：`cargo build`；`cargo run -- clean`（无 plan 报错不变）、构造小 plan 冒烟 dry-run 计数不变
4. **tui**：`Effect::StartClean` 增加 selected；`selected_cleanup_plan` 去 mutation 并产出 id 列表；`default_selected_ids` 去 exe 过滤（删 path_safety import）；runtime worker 透传；相关测试调整（含删除/改写 startup 默认选择跳过 exe 的测试）。
   - 验证：`cargo test`
5. **收口**
   - Grep 无 `selected_by_default = `（tui 生产代码）；`target_contains_current_exe` 生产调用仅 executor
   - 验证：`cargo fmt --all -- --check && cargo test && cargo clippy --all-targets -- -D warnings`

## 回滚点

- 步骤 1–2 与 3–4 分别独立；全部完成后一次性提交。

## 评审门

- 步骤 5 后对照 prd.md 验收清单，再进入 Phase 3。
