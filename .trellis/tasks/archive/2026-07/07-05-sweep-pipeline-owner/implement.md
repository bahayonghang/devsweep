# Implement: sweep 模块 — 扫描管线唯一所有者

按 design.md 执行。每步末尾运行验证命令，失败即修复后再进入下一步。

## 步骤

1. **新建 `src/sweep.rs` 骨架 + lib.rs 注册**
   - `ProjectScan` / `GlobalScan` trait；为 `ProjectScanner` / `GlobalProviderScanner` 实现
   - `Sweeper<P,G>` + `Default`、`ScanOptions`、`ScanPhase`、`ScanProgress`
   - `full_scan` 实现：阶段编排、累计合并、每阶段后经**唯一**私有辅助函数调用 `rank_cleanup_plan`、按契约发进度事件
   - 此步 scanner/providers 仍在内部 rank（幂等，不破坏行为），保证可编译的中间态
   - 验证：`cargo build && cargo test sweep`

2. **sweep 模块测试**（design.md 测试设计 1–5，假扫描器驱动）
   - 验证：`cargo test sweep` 全绿

3. **scanner/providers 摘除内部 rank**
   - 删 `scanner.rs:39` 与 `providers.rs`（scan_with_probe 内）的 `rank_cleanup_plan` 调用及 import
   - 调整二者依赖排序顺序的既有测试（集合断言或测试内排序）
   - 验证：`cargo test scanner providers sweep`

4. **迁移 main.rs::run_scan**
   - 改用 `Sweeper::default().full_scan(&options, &mut |_| {})`；删除 ProjectScanner/GlobalProviderScanner/rank_cleanup_plan 的 import
   - 验证：`cargo build`；手动 `cargo run -- scan --json` 输出结构不变、排序合理

5. **迁移 tui.rs**
   - `run_scan_worker` 改为调用 sweep（current_dir 决策留在 worker）；删除 `run_staged_scan`、`send_scan_progress`
   - `ScanSnapshot` 简化为暂存器（`latest_targets`），删除其 rank 调用
   - `ScanFinished` 处理删除 rank 调用；删除本地 `ScanPhase`，改用 `sweep::ScanPhase`；清理 import
   - 调整受影响的 tui 测试（快照/合并断言）
   - 验证：`cargo test`

6. **收口检查**
   - `grep -n "rank_cleanup_plan(" src/*.rs`：生产调用仅 sweep 内 1 处（ranking.rs 定义与各测试模块除外）
   - `grep -n "ProjectScanner::new\|GlobalProviderScanner::new" src/`：仅 sweep 的 Default 实现出现
   - 验证：`cargo test` 全量 + `cargo clippy -- -D warnings`（不引入新告警）

7. **人工冒烟**
   - `cargo run -- scan`（文本输出）、`cargo run -- scan --json`
   - `cargo run`（TUI）：确认启动扫描的分阶段进度消息、目标列表、增量刷新如常

## 回滚点

- 每步一个逻辑单元；步骤 3 之前 sweep 与旧管线并存，可随时放弃。
- 全部完成后一次性提交（本任务单独成 commit）。

## 评审门

- 步骤 6 通过后，对照 prd.md 验收标准逐条勾验，再进入 Phase 3。
