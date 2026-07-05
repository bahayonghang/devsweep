# Design: Executor 显式选择接口

## 接口变更

### ExecutionRequest（executor.rs）

```rust
pub struct ExecutionRequest {
    pub execute: bool,
    pub allow_permanent_delete: bool,
    pub audit_log: Option<PathBuf>,
    pub selected: Vec<TargetId>,   // 新增：要执行的目标 id，显式穿过接缝
}
```

- `run_plan_with_progress` 的目标集合改为 `plan.targets` 与 `request.selected` 的交集（以 `HashSet<&TargetId>` 查找），**保持 plan 的既有排序**；selected 中不存在于 plan 的 id 被忽略；重复 id 经集合自然去重。
- executor 不再读取 `selected_by_default`。dry-run 的 `report.selected` = 交集大小。
- `execute_target` 内的自清理守卫（`target_contains_current_exe` → Skipped）保持原样——收拢后它是**唯一**强制点。

### model.rs 新增只读辅助

```rust
impl CleanupPlan {
    pub fn default_selected_ids(&self) -> Vec<TargetId> {
        // 所有 selected_by_default 目标的 id，按 plan 顺序
    }
}
```

`selected_by_default` 回归名义语义：ranking（freshness guard）写、其余各处只读的扫描期默认勾选提示。serde 格式不变。

## 调用方迁移

### main.rs::run_clean

```rust
let report = Executor::default().run_plan(&plan, ExecutionRequest {
    execute: command.execute,
    allow_permanent_delete: command.allow_permanent_delete,
    audit_log: command.audit_log,
    selected: plan.default_selected_ids(),
});
```

CLI 行为不变（默认执行集合 = selected_by_default 集合）。

### tui/app.rs

- `selected_cleanup_plan`（742–756）的 clone-and-mutate 消失：改为 clone-filter（不再强写 `selected_by_default = true`），并同时产出 `selected: Vec<TargetId>`（即该子计划全部 id，按序）。
- `Effect::StartClean` 载荷增加 `selected: Vec<TargetId>`（与 plan 子集一起穿过 Effect → runtime → CleanService）。
- `default_selected_ids`（1188–1194）删除 `!target_contains_current_exe(...)` 过滤，退化为"读 `selected_by_default`"的纯 UI 默认值；app.rs 对 `path_safety` 的 import 移除。
  - **行为变化（PRD 已批准）**：包含运行中可执行文件的目标现在会被默认勾选；执行时由 executor 跳过并以 skipped 状态出现在进度与报告中。原测试 `startup_default_selection_skips_target_containing_running_executable` 相应删除/改写。

### tui/runtime.rs

- `CleanService::run_plan` 签名不变（ExecutionRequest 已携带 selected）；`run_clean_worker` 从 Effect 载荷透传 selected 进 ExecutionRequest。
- runtime 测试中 recorded `ExecutionRequest` 的断言补上 selected 字段。

## 收口不变量

- `grep "selected_by_default = " src/tui/` 无写操作（test_support 构造器除外——那是造数据，不是走私）。
- `target_contains_current_exe` 生产调用点仅 executor.rs 一处（path_safety 定义除外）。

## 测试设计

executor.rs 新增/调整（沿用 Recording runner）：

1. `explicit_selection_drives_execution` — 只执行 selected 中的 id，无视 `selected_by_default`。
2. `empty_selection_executes_nothing` — selected 为空 → attempted 0，dry-run selected 0。
3. `unknown_ids_in_selection_are_ignored` — 交集语义。
4. 既有 `target_containing_running_executable_is_skipped_before_command_runs` 保留（守卫唯一强制点）。
5. 既有各测试从"设 flag"迁移为"传 selected"（构造语义等价）。

tui：确认流/进度流测试随 Effect 载荷调整；新增断言 clean worker 透传 selected。

## 权衡记录

- **`Vec<TargetId>` 必填而非 `Option`（None=默认）**：Option 会让"默认选择"语义留在 executor 内，接口重新变得隐式；显式列表 + `CleanupPlan::default_selected_ids()` 辅助让契约完全在类型面上。
- **辅助放 model 而非 executor**：它是对计划数据的只读投影，与执行无关；CLI 与 TUI 都能用。
