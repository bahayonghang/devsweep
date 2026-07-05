# 拆分 tui.rs 为 terminal/runtime/app/render 模块

## Goal

沿既有内部接缝将 3834 行的 `tui.rs` 拆为 `tui/{terminal, runtime, app, render}` 模块目录；对外接口保持 `run()` 一个符号不变；runtime 改为注入扫描依赖与 Executor，使目前零测试覆盖的线程化运行时获得测试面。

## 背景（评审证据）

`tui.rs` 接口只有一个符号（`run()`），实现却混装六个无模块墙分隔的层：

| 关注点 | 约行数 | 位置 |
|---|---|---|
| 终端生命周期 + 线程/worker 编排 | ~200 | 40–243（run、run_event_loop、dispatch_effects、run_scan_worker、run_clean_worker） |
| App 状态 + reducer | ~830 | 246–1094（App 14 个字段；update/handle_key 族 + 业务逻辑） |
| TUI 领域类型 + 自由函数 | ~385 | 1095–1480（UiEvent/Effect/WorkerEvent 等枚举、ScanSnapshot、sum_unique_target_bytes） |
| 渲染 | ~1245 | 1482–2726（render_app → 20+ 渲染/格式化辅助函数） |
| 测试 | ~1108 | 2727–3834（30 个测试） |

关键事实：reducer 已经是纯的（`update(&mut self, UiEvent) -> Vec<Effect>`），渲染已经是 `fn(&App, &mut Frame)` —— Elm 式接缝**已经存在**，只是没有被模块边界强制。runtime 直接构造 `ProjectScanner::new()` / `Executor::default()`，30 个测试全部只打 reducer 和 render，线程运行时、worker→WorkerEvent 翻译零覆盖。

## Requirements

- 拆分为四个子模块，接缝即现有分层：
  - `tui/terminal.rs` — raw-mode / alt-screen 生命周期
  - `tui/runtime.rs` — 事件循环、mpsc 通道、Effect 分发、scan/clean worker
  - `tui/app.rs` — App 状态 + 纯 reducer（含 TUI 领域类型，或按体量再分 `tui/types.rs`）
  - `tui/render.rs`（或 `tui/render/` 目录）— 全部渲染与格式化辅助
- 对外接口不变：`tui::run()` 仍是唯一公共符号；`main.rs` 调用点不动。
- runtime 接受依赖而非创建依赖：扫描入口（sweep 模块）与 Executor 通过参数/构造注入，测试可用假实现驱动 worker→WorkerEvent 翻译。
- `run_staged_scan` 不迁移进任何 tui 子模块——它随 07-05-sweep-pipeline-owner 整体迁出。
- 现有 30 个测试随其被测对象迁移到对应子模块，全部保持通过；为 runtime 新增至少覆盖 worker 事件翻译的测试。

## Acceptance Criteria

- [ ] `src/tui.rs` 单文件消失，替换为 `src/tui/` 目录（mod.rs 只做组装与 `run()` 导出）
- [ ] `tui` 模块对外仍只导出 `run()`；`cargo build` 无需改动 main.rs
- [ ] runtime 不再出现 `ProjectScanner::new()` / `GlobalProviderScanner::new()` / `Executor::default()` 的直接构造（由注入替代）
- [ ] 原有 30 个测试全部迁移并通过；runtime 新增测试 ≥ 1 组（worker 事件翻译）
- [ ] 拆分是移动而非重写：`git diff` 以整块搬迁为主，reducer/render 逻辑无行为变更
- [ ] `cargo test` 全量通过；`cargo clippy` 无新增告警

## 约束与顺序

- **依赖 07-05-sweep-pipeline-owner 先完成**：staged-scan 管线迁出后再拆分，避免搬两次。
- 纯重构：不改键位、不改渲染输出、不改进度展示。
- 复杂任务：`task.py start` 前需补齐 design.md（模块边界、pub(crate) 面、注入方式）与 implement.md（分块搬迁顺序与每步验证命令）。

## Notes

- 词汇约定沿用 /codebase-design：外部**接口**不变（run()），把已存在的内部**接缝**升级为模块墙；runtime 的注入点是新的可测**接缝**——两个 adapter（真实扫描器/假扫描器）证明它是真接缝。
