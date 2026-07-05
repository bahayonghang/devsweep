# Implement: 拆分 tui.rs 为 terminal/runtime/app/render 模块

按 design.md 执行。移动优先于改写；每步验证后再前进。

## 步骤

1. **建骨架**：`src/tui/mod.rs`（含 `run()` 与子模块声明）+ 空的 `terminal.rs`/`runtime.rs`/`app.rs`/`render.rs`；旧 `src/tui.rs` 暂时保留改名为过渡不可取——直接一次性搬迁见步骤 2-5，本步只确认 `mod.rs` 布局可编译（可临时 `include!` 不采用，直接进入搬迁）。
   - 实际操作：直接创建目录结构并开始搬迁，以下每步搬一个模块并保持可编译。
2. **搬 terminal + mod**：`run()` 进 `mod.rs`（暂时仍引用旧文件内容者不留——见搬迁顺序说明），terminal 生命周期进 `terminal.rs`。
   - 推荐顺序：先把整个旧 tui.rs 内容移入 `tui/mod.rs`（一次移动，git 可识别），编译通过后再逐步"从 mod.rs 往子模块搬"，每搬一块编译一次。
   - 验证：`cargo build`
3. **搬 app.rs**（App + impl + 领域类型 + 自由函数 + 对应测试），调整 `use` 与可见性（`pub(super)`）。
   - 验证：`cargo test`
4. **搬 render.rs**（全部渲染/样式/格式化 + 对应测试）。
   - 验证：`cargo test`
5. **搬 runtime.rs 并引入注入接缝**：`ScanService`/`CleanService` trait + 真实 adapter；`run_scan_worker`/`run_clean_worker`/`dispatch_effect` 改为携带服务；`run()`（mod.rs）构造默认服务。
   - 验证：`cargo build && cargo test`
6. **runtime 新增 4 组测试**（design.md 所列，fake 服务驱动 worker 函数）。
   - 验证：`cargo test runtime`
7. **收口检查**
   - `src/tui.rs` 已删除；`mod.rs` 内除 `run()`、模块声明、默认服务构造外无逻辑
   - `grep -n "Sweeper::default\|Executor::default" src/tui/` 仅真实 adapter 两处
   - 验证：`cargo fmt --all -- --check && cargo test && cargo clippy --all-targets -- -D warnings`
8. **人工冒烟**：`cargo run`（TUI 可启动、扫描进度、选择、Rules/Jobs 标签、q 退出）；无 TTY 环境则以全量测试 + `cargo run -- scan` 替代并在报告中注明。

## 回滚点

- 步骤 2 的"整体移入 mod.rs"是独立可回滚的中间态；其后每个子模块搬迁独立成块。
- 全部完成后一次性提交。

## 评审门

- 步骤 7 通过后对照 prd.md 验收清单，再进入 Phase 3（spec 更新 → 提交 → 归档）。
