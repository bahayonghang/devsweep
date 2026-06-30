# 按大小/时间排序与智能默认选择(size x age)

父任务:`06-30-putzen-inspired-optimization`(R2)

## Goal

让 devsweep 的清理计划更接近用户实际决策方式:大目标先浮出水面,同时避免自动勾选最近仍在使用的缓存。实现应复用已完成的 `06-30-perf-parallel-sizing` 产出的共享 `fs_size` 模块和现有 `estimated_bytes` / `last_modified` 字段,不扩大清理执行能力。

## Background

- `CleanTarget` 已包含 `estimated_bytes` 与 `last_modified` 字段([src/model.rs:37](../../../src/model.rs)).
- `fs_size::estimate_tree` 已作为唯一目录估算实现,会返回字节数和子树最新 mtime([src/fs_size.rs](../../../src/fs_size.rs)).
- 项目扫描当前只在 `dedupe_targets` 中按路径深度排序以移除嵌套目标([src/scanner.rs:341](../../../src/scanner.rs));全局扫描按 provider 追加顺序输出([src/providers.rs:84](../../../src/providers.rs)).
- CLI `scan --json` 将 project/global targets 直接 append 到同一个 plan([src/main.rs:21](../../../src/main.rs)).
- TUI 扫描进度和完成态直接消费 plan 的目标顺序,并用 `selected_by_default` 初始化 `selected_ids`([src/tui.rs:707](../../../src/tui.rs), [src/tui.rs:1462](../../../src/tui.rs)).
- putzen-rs 的参考行为是 `score = size_MB x age_days`,默认按 score 排序,并用 `--floor` 默认 7 天把较新的缓存标为 ACTIVE([ref/repo/putzen-rs/src/caches/model.rs:37](../../../ref/repo/putzen-rs/src/caches/model.rs), [ref/repo/putzen-rs/src/caches/mod.rs:28](../../../ref/repo/putzen-rs/src/caches/mod.rs)).

## Requirements

- R2.1 排序:清理计划中的目标应按 `estimated_bytes` 降序排列,使 TUI 和 `scan --json` 都看到同一稳定顺序。相同大小时用确定性 tie-breaker,避免测试和 JSON 输出抖动。
- R2.2 评分基础:提供可测试的 `size x age` score helper,基于 `estimated_bytes` 和 `last_modified` 计算。当前交付不要求新增 CLI/TUI 切换排序模式,但 helper 应能支撑后续切换。
- R2.3 新鲜度守卫:默认 7 天内修改过的目标如果原本 `selected_by_default == true`,应改为 false;缺失 `last_modified`、未来 mtime 或已经不默认勾选的目标不得因此变得更激进。
- R2.4 可解释性:新鲜度守卫应在目标证据或详情可见信息中留下原因,让 TUI Details 能解释为什么原本可默认清理的目标未自动勾选。
- R2.5 契约兼容:不得改变 `CleanupPlan` / `CleanTarget` JSON 形状;若只改变目标顺序和字段值,保持 `CLEANUP_PLAN_VERSION = 1` 并在 design 中说明。

## Acceptance Criteria

- [ ] 给定多个目标时,最终 plan 目标按 `estimated_bytes` 降序输出;相同大小时排序稳定且确定。
- [ ] 7 天内修改且原本默认勾选的目标最终 `selected_by_default == false`;陈旧目标维持原默认;原本不默认勾选的高/中风险目标不会被自动勾选。
- [ ] 缺失 `last_modified` 或 mtime 晚于当前时间的目标不会触发 panic,且不会因为 freshness guard 改得更激进。
- [ ] TUI 使用同一排序后的目标列表,不在展示层另行实现一套排序;`selected_ids` 仍来自目标的最终 `selected_by_default`.
- [ ] 新增单元测试覆盖排序、score、freshness guard、缺失/未来 mtime 回退、TUI 扫描合并顺序。
- [ ] `cargo test --all-targets` 全绿;实施完成前运行 `just ci`.

## Constraints

- 排序不是默认选择策略。不得仅因目标体积大就改为默认勾选。
- Scanner/provider/model 层仍只能创建计划;不得删除文件、移动回收站或执行清理命令。
- Cargo home 和其他 inspect-only / 高风险目标继续不默认勾选。
- 不新增 CLI 参数作为本子任务的验收要求;可配置 floor 或交互排序切换留给后续任务,避免扩大范围。
- 子任务顺序依赖写入本 artifact:本任务在 `06-30-perf-parallel-sizing` 完成后实施,因为它复用共享 `fs_size` 的 `last_modified` 语义;不依赖 `06-30-declarative-rules-catalogue`.

## Out Of Scope

- 不实现 putzen 的 highscore/热力条/游戏化 score UI.
- 不新增 `scan --sort`、`--floor` 或 TUI sort-cycle 快捷键。
- 不扩展全局缓存规则;该范围归 `06-30-declarative-rules-catalogue`.
- 不改变执行器的选择语义:executor 仍只执行 plan 中 `selected_by_default == true` 的目标。

## Open Questions

无阻塞问题。默认实现选择为:按大小降序作为当前用户可见排序,7 天 freshness guard 作为常量策略,`size x age` 先做内部 helper 与测试基础。
