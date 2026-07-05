# Design: 拆分 tui.rs 为 terminal/runtime/app/render 模块

## 前置

07-05-sweep-pipeline-owner 已完成：`run_staged_scan` 已删除，scan worker 走 `sweep::full_scan`，`ScanSnapshot` 已是暂存器。本任务纯搬迁 + 一处注入接缝，不改任何用户可见行为。

## 目标布局

`src/tui.rs`（3762 行）→ `src/tui/` 目录：

| 新文件 | 内容来源（现 tui.rs 行区间） | 职责 |
|---|---|---|
| `tui/mod.rs` | 37–52（`run`）+ 模块声明与 `pub use` | 对外唯一接口 `run()`；组装 terminal + runtime |
| `tui/terminal.rs` | 41–56（raw-mode/alt-screen 进入与 `restore_terminal`） | 终端生命周期 |
| `tui/runtime.rs` | 58–188（`run_event_loop`、`drain_worker_events`、`dispatch_effects/effect`、`run_scan_worker`、`run_clean_worker`） | 事件循环、通道、worker 线程；**注入接缝在此** |
| `tui/app.rs` | 190–1036（`App` + impl）+ 1038–1402（TUI 领域类型与自由函数：`UiEvent`/`Effect`/`WorkerEvent`/`ScanSnapshot`/`ActiveTab`/`Overlay`/`ConfirmState`/`CommandPreview`/`CleanupProgress*`/`JobKind`/`JobStatus`/`JobRecord`/`LogEntry`/`AppLog*`/`FooterTone`/`FooterAction`/`ScopeKind`/`BodyLayoutKind`/`sum_unique_target_bytes`/`default_selected_ids`/`cleanup_progress_for_plan` 等） | 状态 + 纯 reducer + 领域类型 |
| `tui/render.rs` | 1404–2646（`render_app` 起的全部渲染、样式、格式化辅助） | 纯视图：`fn(&App, &mut Frame)` |

测试（2648–3762，30 个）按被测接口就近拆入各子模块的 `#[cfg(test)] mod tests`：reducer/状态/快照测试 → app.rs；渲染快照与格式化测试 → render.rs；新增 runtime 测试 → runtime.rs。

类型归属原则：被 reducer 与渲染共同使用的类型（App、ConfirmState、CleanupProgress 等）一律放 app.rs，render.rs 只读取；`FooterTone`/`FooterAction`/`BodyLayoutKind`/`ScopeKind` 若仅渲染使用则放 render.rs（实现时以实际引用为准，原则：**类型跟随写它的模块，读它的模块 import**）。

## 可见性纪律

- `tui/mod.rs` 对 crate 外只 `pub use` … 实际上 `run()` 是 crate 内 public（main.rs 调用），保持 `pub fn run()`；其余子模块项一律 `pub(super)` 或私有。
- 子模块之间经 `super::` 引用，不新增跨模块的公开面。

## 注入接缝（runtime）

现状：`run_scan_worker` 内 `Sweeper::default()`、`run_clean_worker` 内 `Executor::default()` —— 直接创建依赖，不可测。

设计：沿用 `Executor<C,T>` 的泛型 + 默认实现惯例，引入两个窄 trait（放 runtime.rs）：

```rust
pub(super) trait ScanService: Send + Clone + 'static {
    fn full_scan(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
    ) -> Result<CleanupPlan>;
}

pub(super) trait CleanService: Send + Clone + 'static {
    fn run_plan(
        &self,
        plan: &CleanupPlan,
        request: ExecutionRequest,
        on_progress: &mut dyn FnMut(ExecutionProgress),
    ) -> Result<ExecutionReport>;
}
```

- 真实 adapter：`SweepScanService`（内部 `Sweeper::default().full_scan(...)`）与 `ExecutorCleanService`（内部 `Executor::default().run_plan_with_progress(...)`）。`Send + Clone + 'static` 因 worker 每次 spawn 线程按值携带服务克隆。
- `dispatch_effect` 与两个 worker 函数改为携带服务参数；`run_event_loop` 泛型化或以一个 `Runtime<S, C>` 小结构持有 `(scan, clean, worker_tx)`——实现时取更少样板的一种，遵循"类型签名最小"。
- `run()` 构造默认服务。**收口后 runtime 内不再出现 `Sweeper::default()`/`Executor::default()` 之外的构造，且二者只出现在真实 adapter 内。**
- 两个 adapter（真实 / 测试用 fake）证明接缝为真。

## runtime 新增测试（此前零覆盖的部分）

用 fake ScanService / CleanService（记录调用、返回固定结果或错误）直接调用 worker 函数，断言 WorkerEvent 翻译：

1. `scan_worker_emits_started_progress_finished` — 成功路径事件序列与 partial 透传。
2. `scan_worker_reports_failure_as_job_failed` — 扫描 Err → `JobFailed { message }`。
3. `clean_worker_translates_execution_progress` — ExecutionProgress → CleanProgress 字段映射（含 `compact_target_id` 格式）。
4. `clean_worker_reports_finish_and_failure` — 成功 → CleanFinished；Err → JobFailed。

事件循环本身（terminal 交互）不强测——终端 I/O 属 terminal.rs 的薄壳。

## 搬迁纪律

- **移动优先于改写**：除注入接缝与 `use` 路径外，函数体逐字搬迁；`git diff` 应呈现整块移动。
- 渲染输出、键位、进度文案零变更；现有 30 个测试除 `use` 路径与模块前缀外不改断言。
- 搬迁后 `src/tui.rs` 删除。

## 兼容性

- `lib.rs` 的 `pub mod tui;` 不变（指向目录模块）；`main.rs` 的 `devsweep::tui::run()` 调用点零改动。
- 不动 model/sweep/executor 的任何接口。

## 权衡记录

- **CleanService 用 `&mut dyn FnMut` 而非泛型回调**：executor 的 `run_plan_with_progress` 是泛型 `F: FnMut`，adapter 内闭包转接即可；trait 侧保持 dyn 友好、签名最小。
- **不拆 render 为多文件**：1200 行渲染函数已按 `render_*` 前缀自然分区，单文件不阻碍导航；再拆目录属 speculative（本任务不做，留给真实痛点出现时）。
- **App 与领域类型同文件**：reducer 与其操作的类型高内聚；再分 `types.rs` 只会把一次修改摊到两个文件（损失 locality）。
