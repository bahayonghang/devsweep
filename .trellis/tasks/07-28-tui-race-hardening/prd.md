# TUI 竞态止血：确认冻结、single-flight 与精确去重

> 父任务：07-28-audit-remediation ｜ 审计条目：F-02、F-07、F-03（止血部分）、F-20 ｜ 对应审计 Phase 0

## Goal

关闭 TUI 清理路径上三个最危险的竞态：确认对象漂移、并发 mutation job、重复 footprint 重复执行；同时移除"伪取消"的错误承诺、保留用户显式选择。这是全部整改的先行止血任务，不引入新架构，只加约束。

## 背景与证据（当前 HEAD）

1. **确认对象漂移（F-02）**：`ConfirmState` 只保存 count/字符串摘要/命令预览（`src/tui/app.rs:1046-1062`）。用户按 Enter 时调用 `selected_cleanup_plan()` 从**当前可变状态**重建计划（`app.rs:274`）。确认框打开期间，`ScanProgress`/`ScanFinished` 仍会替换 `self.targets` 并重置 `selected_ids`（`app.rs:350-357`、`app.rs:493-499`），违反 "what you see is what will execute"。
2. **无 single-flight（F-07）**：clean 进行中 `handle_normal_key` 仍处理 `c`/`s`（`app.rs:112-198`）；`dispatch_effect` 对 `StartClean`/`StartScan` 无条件 `thread::spawn`（`src/tui/runtime.rs:124-135`）。两个 clean job 可并发。
3. **精确去重 bug（F-07）**：`dedupe_targets` 判 nested 用 `path != existing_path && path.starts_with(existing_path)`（`src/scanner.rs:304-309`），路径完全相等时不视为重复，重复 roots / parent+child roots 产生同一 footprint 的多个 target，executor 逐项执行。
4. **伪取消与终态复活（F-03 止血）**：`Effect::CancelJob` 仅向 UI channel 回发 `JobCanceled`（`runtime.rs:136-138`），worker 与已启动命令继续执行；`mark_job` 无合法状态迁移约束（`app.rs:608-613`），后续 `JobProgress`/`CleanProgress` 会把 `Cancelling` 改回 `Running`，`CleanFinished` 可把 `Canceled` 覆盖为 `Succeeded`/`Failed`（`app.rs:370-380`）。
5. **选择被重置（F-20）**：staged scan 每次 `rebuild_targets_from_scan_snapshot` 和 `ScanFinished` 都用 `default_selected_ids` 重置选择（`app.rs:354`、`app.rs:496-497`），用户的显式勾选/取消被丢弃。

## Requirements

1. `ConfirmState` 持有不可变 `ExecutionManifest`（选中 targets 的完整快照）；Enter 只消费该 manifest，不得重读可变 state。manifest 身份先以结构等价（同一 target 集合 + action identity）断言；canonical digest 契约由 07-28-plan-validation 拥有，落地后确认文案接入其前 8–12 位展示——本任务不得发明第二种 digest。
2. 确认期间到达的 scan 更新必须立即使 modal 失效并要求重新确认（决策 D5）：更新不得与已打开的确认窗口并存为可执行的最新状态；Enter 被拒绝并提示重新确认。不得延迟应用扫描更新。
3. mutation single-flight：application 层与 UI 层双重保证——clean 运行期间再次触发 `c`（以及会改写 targets 的 `s`）被拒绝并给出提示；`dispatch_effect` 层拒绝并发 `StartClean`。
4. 修复 exact duplicate：dedupe key 使用 canonical footprint + action identity；完全相等路径合并 evidence 而非保留双份；roots 先 canonicalize 再取最小覆盖集。**定位（二轮评审 SEC-002）**：scanner 端 dedupe 属展示层/扫描产物质量修复，不是安全边界——外部 plan 完全绕过 scanner；"每 fingerprint 恰好执行一次"的强制点在 07-28-plan-validation 的 ValidatedPlan 不变量与 executor once-ledger。
5. Job 状态机立约：`Running → Cancelling → Canceled / Succeeded / Failed`，终态不可被后续事件改写；迟到事件仅进 log。
6. 取消语义诚实化：在真实取消（07-28-true-cancellation）落地前，`x` 显示"请求在 action 边界停止；当前 action 不可中断"，不得显示未发生的 "Canceled"。
7. 保留用户选择：staged scan 合并新 targets 时，对已存在的 target 保留用户显式 select/deselect，仅对新增 target 应用默认选中。

## Acceptance Criteria

- [x] 回归测试：open confirm → `ScanProgress`/`ScanFinished` → Enter，Enter 被拒绝且提示重新确认，runner 调用次数为 0
- [x] 回归测试：clean 运行中按 `c`/`s`，`StartClean`/`StartScan` effect 不再产生第二个 mutation worker；请求被明确拒绝且有日志
- [x] 回归测试：重复 root、parent+child root、相同 path 不同 rule、连续两次 clean —— 每个 physical/action footprint 的 runner 调用次数恰好为 1（覆盖 scanner/TUI 路径；外部 plan 路径的等价保证由 plan-validation 的 fingerprint 套件负责）
- [x] 回归测试：`Cancelling`/`Canceled`/`Succeeded`/`Failed` 之后注入迟到 `JobProgress`/`CleanFinished`/`JobCanceled`，终态不变
- [x] 回归测试：用户显式取消勾选某 default-selected target 后触发 staged scan 更新，该 target 保持未选中
- [x] UI 不再出现与 worker 实际状态不符的 "Canceled" 文案
- [x] `cargo test` 全绿；`cargo clippy -D warnings` 无新增告警

## 约束与依赖

- 无前置依赖；建议最先执行（其余 P1 子任务的回归测试将依赖此处的状态机约定）。
- 不改 `CleanupPlan` serde 格式；manifest 冻结只发生在 TUI 内存中。
- 真实 cancellation token / 进程树终止不在本任务范围（runner/进程树归 07-28-process-runner-cancellation，token 贯穿与状态机接线归 07-28-true-cancellation）。
