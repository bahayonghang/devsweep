# core GUI 支撑契约扩展(serde/执行明细/确认 digest)

> 父任务:`08-03-tauri-desktop-app`(缺口清单见父任务 `design.md` §2.1)。
> 依赖:`08-03-core-api-extraction` 必须先完成(需要 workspace 与公开 API 就位)。

## Goal

补齐 GUI 复用所需而 core 目前缺失的契约:serde 派生、逐目标执行明细、
预计可回收字节聚合、执行确认 digest API。全部为增量变更,CLI 行为不变。

## Requirements

- serde 派生(Serialize + Deserialize):`ScanOptions`、`ScanPhase`、`ScanProgress`、
  `ExecutionReport`、`ActionFailure` 及其字段类型闭包(现状均无派生,
  证据:`src/scan/mod.rs`、`src/execution/mod.rs`)。
- `ExecutionReport` 扩展逐目标结果:每个被选目标一条(target_id、动作种类、
  成功/失败/跳过 + 原因),现有计数字段保持并与明细一致。
- 可回收字节:沿用扫描侧既有容量口径(verified / partial / unknown),在执行
  报告中聚合"预计可回收字节";**口径为回收站语义,类型与字段命名不得出现
  "freed" 字样**(父任务已决策)。
- 确认 digest API:对(已验证清单, 选择集)计算稳定 digest;公开
  "dry-run 返回 digest、execute 校验 digest(不匹配即结构化错误)"的核心层
  能力;语义与 TUI 冻结语义一致(`.trellis/spec/frontend/state-management.md`);
  实现应复用 `plan/` 现有 SHA-256 摘要基础设施。
- selected_ids 边界行为在核心层定义并测试:未知 id 整体拒绝、重复 id 去重并
  注明、inspect-only 目标不可选中执行、irreversible 动作在报告中显式标记
  (父任务 `design.md` §3)。
- CLI / TUI 行为不变;TUI 可在后续迭代改用新 digest API,本任务不动 TUI。

## Acceptance Criteria

- [ ] `just ci` 全绿;新增字段与派生均有单元测试(含 serde 往返测试)。
- [ ] 子任务 1 留档的 JSON 等价门在本任务完成后复跑仍通过
      (`scan --json` 输出不因本任务改变;若确需新增字段,记录差异并说明)。
- [ ] 逐目标明细与计数一致性有测试覆盖(succeeded+failed+skipped == attempted 等)。
- [ ] digest 性质有测试:同输入稳定;计划内容或选择集任一变化 → digest 变化;
      execute 带过期 digest → 结构化错误。
- [ ] 边界行为四条(未知/重复/inspect-only/irreversible)各有测试。

## Notes

- 完成后解锁 `08-03-tauri-shell-backend`。
- 命名审查点:所有新增公开项经得起"回收站口径"检查。
