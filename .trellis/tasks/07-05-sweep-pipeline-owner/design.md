# Design: sweep 模块 — 扫描管线唯一所有者

## 模块边界

新增 `src/sweep.rs`（`lib.rs` 加 `pub mod sweep;`）。sweep 是 scan→merge→rank 管线的唯一所有者，对外接口：

```rust
pub struct Sweeper<P = ProjectScanner, G = GlobalProviderScanner> {
    projects: P,
    global: G,
}

pub struct ScanOptions {
    pub include_projects: bool,
    pub include_global: bool,
    pub roots: Vec<PathBuf>,       // 项目扫描根；include_projects=false 时忽略
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanPhase { Projects, Global }   // 从 tui.rs 迁入（tui 删除本地定义，改用 sweep::ScanPhase）

pub struct ScanProgress {
    pub phase: ScanPhase,
    pub message: String,
    /// 到当前阶段为止、已合并且已排序的累计部分计划
    pub partial: Option<CleanupPlan>,
}

impl Sweeper {
    pub fn full_scan(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
    ) -> Result<CleanupPlan>;
}
```

- `Sweeper: Default`（真实扫描器），与 `Executor<C,T>` 同一注入惯例：泛型 + 默认实现。
- CLI 忽略进度：`full_scan(&options, &mut |_| {})`。

## 注入接缝

```rust
pub trait ProjectScan {
    fn scan_roots(&self, roots: &[PathBuf]) -> Result<CleanupPlan>;
}
pub trait GlobalScan {
    fn scan(&self) -> CleanupPlan;
}
```

`ProjectScanner` / `GlobalProviderScanner` 各自实现（在 sweep.rs 内 impl，不动 scanner/providers 的文件职责）。测试用假扫描器驱动——两个 adapter（真实/假）证明这是真接缝。

## 契约变更（关键）

1. **scanner 与 providers 停止内部调用 `rank_cleanup_plan`**：
   - `scanner.rs:39` 删除 rank 调用；`scan_roots` 返回**未排序**计划（dedupe 保留）。
   - `providers.rs:99`（`scan_with_probe`）删除 rank 调用，返回未排序计划。
   - 二者的接口契约从"返回已排序计划"收窄为"返回扫描结果"；排序职责整体上移至 sweep。
   - 各自现有测试若依赖排序顺序，改为断言集合内容或在测试内显式排序。

2. **ranking 的调用点收拢**：`rank_cleanup_plan` 生产代码仅 sweep 内 1 处调用点（一个私有辅助函数）。它在每个阶段完成后对**累计**目标集调用（freshness guard 幂等，evidence 去重，安全），并在最终返回前保证全集已排序。emitted partial 是排序后的克隆。

3. **不变量**：`full_scan` 返回的计划**已排序且 freshness guard 已应用**。调用方不得再 rank。

## 调用方迁移

### main.rs::run_scan（24–37）

```rust
let options = ScanOptions {
    include_projects: command.projects || !command.global,
    include_global: command.global || !command.projects,
    roots: command.roots.clone(),
};
let plan = Sweeper::default().full_scan(&options, &mut |_| {})?;
```

删除对 ProjectScanner/GlobalProviderScanner/rank_cleanup_plan 的直接使用与 import。

### tui.rs

- `run_staged_scan`（135–187）整体删除，`run_scan_worker` 改为：取 current_dir（仍是 TUI 的 scope 决策）→ 构造 ScanOptions（include 两者，roots=[current_dir]）→ `Sweeper::default().full_scan` + 进度闭包，把 `ScanProgress` 翻译为 `WorkerEvent::ScanProgress { job_id, phase, message, plan: partial }`。现有四条进度消息文案与粒度保持不变（"Scanning current directory" / "Project scan finished: N target(s)" / "Scanning global providers" + "Estimating global cache sizes" / "Global scan finished: N target(s)"）。
- **`ScanSnapshot` 简化**：partial 已是"累计 + 已排序"，snapshot 不再按 phase 分桶合并——字段改为 `latest_targets: Vec<CleanTarget>`，`set_phase_targets` 变为 `set_targets`，`targets()` 直接返回克隆，**删除 rank 调用（1220）**。stale-job 判断（`should_apply_scan_update`）不变。
- **`ScanFinished` 处理（568–569）删除 `rank_cleanup_plan(&mut plan)`**：计划由 sweep 保证已排序。
- tui 本地 `ScanPhase` 枚举删除，改用 `sweep::ScanPhase`。
- tui 对 `rank_cleanup_plan` 的 import 删除。

## 进度事件时序（契约）

```
Projects 阶段（include_projects 时）:
  ScanProgress { Projects, "Scanning current directory…"(由调用方传入的 root 无关), partial: None }
  ScanProgress { Projects, "Project scan finished: N target(s)", partial: Some(ranked cumulative) }
Global 阶段（include_global 时）:
  ScanProgress { Global, "Scanning global providers", partial: None }
  ScanProgress { Global, "Estimating global cache sizes", partial: None }
  ScanProgress { Global, "Global scan finished: N target(s)", partial: Some(ranked cumulative) }
返回: Ok(最终 ranked CleanupPlan) —— 与最后一次 partial 内容一致
```

消息文案由 sweep 产生（TUI 现有文案原样迁入），保证 CLI/TUI 共享同一管线时进度语义一致。

## 权衡记录

- **进度回调 vs 泛型 Sink trait**：选 `&mut dyn FnMut(ScanProgress)`——单一消费者、无需对象安全之外的能力，避免为回调再造 trait。若未来出现第二种消费形态再引 trait（一个 adapter 是假想接缝）。
- **partial 采用累计集而非按阶段增量**：把 merge 职责留在 sweep 内部（locality），TUI 的 ScanSnapshot 从"合并器"退化为"暂存器"，删除其 rank/merge 逻辑。代价是每次 partial 克隆全量目标集——目标数为几十到几百，可忽略。
- **`ScanPhase` 放 sweep 而非 model**：它是管线的进度词汇，不是计划数据的一部分；CleanupPlan serde 不受影响。
- **取消能力不在本任务范围**：现有 staged scan 亦不可中途取消（JobCanceled 只是 UI 标记），不为此扩接口。

## 兼容性

- `CleanupPlan` serde 格式与 `CLEANUP_PLAN_VERSION` 不变。
- `devsweep scan --json` 输出内容不变（同一批 target、同一排序规则）。
- TUI 进度展示、增量刷新、stale-scan 防护行为不变。

## 测试设计

sweep 模块内 `#[cfg(test)]`，用假扫描器（`FakeProjects`/`FakeGlobal` 返回固定未排序 targets）：

1. `full_scan_merges_and_ranks_once` — 合并两侧目标，最终顺序符合 ranking 规则（bytes 降序）。
2. `full_scan_applies_freshness_guard_exactly_once` — 新鲜目标被反选且 guard evidence 恰好 1 条。
3. `progress_events_follow_staged_sequence` — 记录事件序列，断言 phase/文案/partial 有无 按上述契约。
4. `scan_options_skip_phases` — 仅 projects / 仅 global 时另一阶段的事件与目标不出现。
5. `partial_plans_are_cumulative_and_ranked` — 第二个 partial 包含第一阶段目标且有序。

scanner/providers 现有测试：断言从"有序"调整为"集合相等"（或测试内排序后比较），不新增职责。
tui 现有测试：ScanSnapshot 相关测试随简化同步调整，行为断言（stale 防护、增量合并结果）保留。
