# Implement — 性能与构建优化

前置:`task.py start 06-30-perf-parallel-sizing`(状态置 in_progress)后再动代码。

## 基线(改动前)

1. `cargo test` → 记录全绿基线。
2. 选一棵大目录树(本机含 `node_modules`/`target` 的工程)记录 TUI 启动扫描的体感耗时,或临时 `tracing` 计时 `estimate_tree`,作为后续对比。

## 步骤

1. **加依赖**:[Cargo.toml](../../../Cargo.toml) `[dependencies]` 增 `rayon = "1"`。
   - verify:`cargo build`。
2. **建共享模块** `src/fs_size.rs`:实现 design 中的串行版 `estimate_tree` + `is_unsafe_link` + `has_windows_reparse_point`(先照搬现有逻辑,保证语义)。在 [src/lib.rs](../../../src/lib.rs) 加 `pub mod fs_size;`。
   - verify:`cargo build`。
3. **接入 scanner**:[src/scanner.rs](../../../src/scanner.rs) 删除本地 `estimate_tree`/`is_unsafe_link`/`has_windows_reparse_point`,改 `use crate::fs_size::...`。
   - verify:`cargo test scanner`(scanner 相关用例全绿)。
4. **接入 providers**:[src/providers.rs](../../../src/providers.rs) 同样删除副本并改用共享模块,`SystemProviderProbe::estimate_tree` 转调。
   - verify:`cargo test providers`。
5. **并行化**:把 `fs_size::estimate_tree` 子项遍历改为 `par_iter().map(...).reduce(combine)`(design 第 2 节)。
   - verify:`cargo test`(含新增等价性/精确性用例)。
6. **加测试**:在 `fs_size.rs` 增字节精确、最新 mtime、并行=串行等价、symlink/reparse 跳过用例。
   - verify:`cargo test fs_size`。
7. **release profile**:[Cargo.toml](../../../Cargo.toml) 加 `[profile.release]`(lto/codegen-units=1/strip)。
   - verify:`cargo build --release` 成功;`./target/release/devsweep --help` 可运行。
8. **性能复测**:用步骤 0 同一棵树复测,确认估算耗时下降并记录到本文件结尾。

## Review Gate

- [x] `cargo test` 全绿、`cargo build --release` 成功。
- [x] `estimate_tree` 全仓仅一处实现(`rg "^fn estimate_tree|^pub fn estimate_tree" src/` 仅命中 fs_size.rs 的实现;providers 中仅保留 trait/mock 方法)。
- [x] symlink/reparse 用例未回退。
- [x] 记录优化前后耗时对比。
- [x] 运行 `trellis-check`(或 /trellis:finish-work 前置检查)。

## Rollback

- 各步独立提交;并行化(步骤 5)若引入不一致或回退,可单独还原为串行版而保留模块抽取与 profile 成果。
- profile(步骤 7)与代码改动解耦,可独立增删。

## 测量记录(实现后填写)

- 树规模:D:\Documents\Code\Rust\Exp\devsweep 当前工作区,含 target/ 约 8971 个条目。
- 测量方式:从 HEAD 临时 git clone 构建改动前 release 二进制,与当前 release 二进制分别执行 `devsweep scan <repo-root> --json | Out-Null`。
- 改动前:44080.11 ms;改动后:19783.89 ms;约 2.23x。
