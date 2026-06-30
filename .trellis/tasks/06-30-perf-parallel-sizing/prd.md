# 性能与构建优化:rayon 并行 estimate_tree + 共享 fs 模块 + release profile

父任务:`06-30-putzen-inspired-optimization`(R1)

## Goal

降低扫描/大小估算耗时,并消除目录估算与链接安全判断的重复实现;顺带补齐 release 构建配置。

## Problem

- `estimate_tree`(递归求 size + 最新 mtime)单线程,且在两处**逐字重复**:
  - [src/scanner.rs:341](../../../src/scanner.rs)
  - [src/providers.rs:376](../../../src/providers.rs)
- `is_unsafe_link` / `has_windows_reparse_point` 同样在 scanner 与 providers 重复。
- 大目录(node_modules、target、`~/.cargo`)的估算是扫描阶段主要耗时;putzen 用 rayon `par_bridge` 并行([ref/repo/putzen-rs/src/cleaner.rs:12](../../../ref/repo/putzen-rs/src/cleaner.rs))。
- [Cargo.toml](../../../Cargo.toml) 无 `[profile.release]`,二进制偏大、release 未优化。

## Requirements

- 抽取共享模块(暂名 `fs_size` 或 `fsutil`),统一:
  - `estimate_tree(path) -> (u64 bytes, Option<SystemTime> latest_mtime)`
  - `is_unsafe_link` / `has_windows_reparse_point`(保持现有 Windows reparse 语义)
- 用 rayon 并行化大小估算,在保持结果**完全一致**(同样的字节合计与最新 mtime、同样跳过 symlink/reparse)的前提下提速。
- scanner.rs 与 providers.rs 改为调用共享模块,删除各自副本(仅删因本次改动而产生的冗余,符合 CLAUDE.md)。
- Cargo.toml 增加 release profile(对齐 putzen:`lto=true`、`codegen-units=1`、`strip=true`;`panic="abort"` 视测试影响决定,见 design)。

## Acceptance Criteria

- [ ] `estimate_tree` 仅存在一处实现;scanner/providers 复用之。
- [ ] 并行版与原串行版在构造目录树上字节数与最新 mtime 结果一致(新增对照测试)。
- [ ] symlink 与 Windows reparse point 仍被跳过(现有相关测试不回退)。
- [ ] `cargo test` 全绿;`cargo build --release` 成功并应用 profile。
- [ ] 在一棵大目录树(如本机含 node_modules/target 的工程)上手测,估算耗时较优化前下降(implement.md 给出测量步骤)。

## Notes / Risks

- rayon 是新依赖,需加入 [Cargo.toml](../../../Cargo.toml)。
- `panic = "abort"` 可能影响 `#[should_panic]` 或测试可见性;若有冲突仅在 release profile 保守处理或省略该项,不影响其余优化。
- 不改变 `CleanupPlan` JSON 形状,本子任务对外契约零变更。
