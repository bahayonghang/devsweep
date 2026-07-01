# 借鉴 putzen-rs 优化 devsweep:性能/排序/广度

## Goal

把参考项目 `ref/repo/putzen-rs` 的三项长处嫁接进 devsweep——**rayon 并行**、**size×age 排序**、**数据驱动的缓存目录表**——同时**保留 devsweep 自身更强的安全/可测试设计**(回收站而非永久删除、官方命令优先、审计日志、依赖注入测试、symlink/reparse 防护)。

## Background:两个项目的真实差距

devsweep 当前架构(已读全量源码确认)其实**比 putzen 更安全、更可测**:

- Elm 风格异步 TUI(`thread::spawn` + `mpsc`,分阶段流式扫描)— [src/tui.rs](../../../src/tui.rs)
- 安全第一:`trash` 回收站、`DeletePermanently` 硬禁用([src/executor.rs:284](../../../src/executor.rs))、保护正在运行的 exe 目录([src/path_safety.rs](../../../src/path_safety.rs))、跳过 symlink / Windows reparse point
- 官方命令优先:`cargo clean` / `npm cache clean` / `pip cache purge` / `pnpm store prune`,而非盲删([src/providers.rs](../../../src/providers.rs))
- 审计日志(JSONL)、`ProviderProbe`/`CommandRunner`/`TrashRunner` 依赖注入、59 处测试

putzen 领先、而 devsweep **确实缺失**的点(本任务范围):

1. **并行**:putzen 用 rayon `par_bridge` 算目录大小([ref/repo/putzen-rs/src/cleaner.rs:12](../../../ref/repo/putzen-rs/src/cleaner.rs));devsweep 的 `estimate_tree` 单线程递归,且在 [src/scanner.rs:341](../../../src/scanner.rs) 与 [src/providers.rs:376](../../../src/providers.rs) **重复实现**。
2. **构建**:[Cargo.toml](../../../Cargo.toml) 无 `[profile.release]`;putzen 有 `lto/codegen-units=1/strip/panic=abort`。
3. **排序**:devsweep 采集了 `last_modified` 却未用于排序或默认勾选;putzen 用 `size_MiB × age_days` 评分并以 `--floor` 把新缓存标 ACTIVE。
4. **广度/可维护性**:devsweep 全局仅覆盖 npm/pip/pnpm/yarn/cargo 5 项,规则硬编码为命令式 Rust;putzen 用 `roots!` 宏声明 ~50 个缓存目录。`Ecosystem::Docker` 枚举存在但无 provider 产出;`Rules` 命令/标签页仍是占位([src/main.rs:98](../../../src/main.rs))。

## Requirements

按可独立验证拆为 3 个子任务:

- R1 — **性能与构建** → 子任务 `06-30-perf-parallel-sizing`
  - 用 rayon 并行化目录大小估算
  - 把重复的 `estimate_tree` 与 `is_unsafe_link`/`has_windows_reparse_point` 抽到共享模块
  - Cargo.toml 增加 release profile
- R2 — **排序与智能默认** → 子任务 `06-30-size-age-ranking`
  - 目标按 size(及可选 size×age 评分)排序,大件浮顶
  - 近期修改过的缓存默认不勾选(借鉴 `--floor`),阈值可配置
- R3 — **广度与声明式规则** → 子任务 `06-30-declarative-rules-catalogue`
  - 把硬编码规则抽成数据驱动的规则表(putzen `roots!` 思路)
  - 扩展全局缓存目录(gradle/maven/go/huggingface/JetBrains 等),保持"命令优先、否则 inspect/trash"的安全分级
  - 让 `Rules` 标签页列出规则表

## Constraints(必须遵守,不可回退)

- 不得放弃"命令优先 + trash 回收站 + 永久删除禁用"的安全策略。
- 不得移除 symlink / Windows reparse point 防护与自清理保护。
- `CleanupPlan` 是已版本化的导出契约([src/model.rs:5](../../../src/model.rs) `CLEANUP_PLAN_VERSION=1`):若改变 JSON 形状必须升版本并说明兼容策略;能不破坏就不破坏。
- 改动需保持现有测试通过,新行为需补测试(项目惯例:DI + tempfile,跨平台分支用 `#[cfg(windows)]`)。
- 遵循 CLAUDE.md:最小改动、不顺手重构无关代码。

## Cross-child Acceptance Criteria

- [ ] `cargo test` 全绿(含各子任务新增测试)。
- [ ] `cargo build --release` 成功且应用了 release profile。
- [ ] 在一棵较大的真实/构造目录树上,扫描/估算耗时较优化前明显下降(子任务给出对比方法)。
- [ ] TUI 中目标按大小有序展示;近期修改的缓存不被默认勾选。
- [ ] 全局扫描覆盖面较优化前扩大,且新目录的 `action`/`risk` 分级符合安全约束。
- [ ] `--json` 计划结构若变更,版本号与兼容性已在 design 中交代。

## Suggested Ordering

1. 先做 `perf-parallel-sizing`:它会产出共享 fs 模块,后续子任务复用。
2. 再做 `size-age-ranking`(独立,数据已具备)。
3. 最后做 `declarative-rules-catalogue`(最大、最易受前两者影响)。

顺序为建议而非硬依赖;各子任务验收彼此独立。

## Non-goals

- 不引入 putzen 的 highscore/游戏化记分板。
- 不照搬 putzen 的 `remove_dir_all` 永久删除路径。
- 不强制改用 `walkdir`/`jwalk` 替换现有手写遍历(除非性能子任务证明遍历本身是瓶颈)。
- Docker provider 视为可选延伸,本父任务范围内仅在规则表预留,不要求完整实现。
