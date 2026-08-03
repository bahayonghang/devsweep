# 执行计划:契约扩展

## 步骤

1. 核对 `ScanReport` 现有序列化风格(字段命名、enum 表示),把结论写进
   design.md §1(决定新类型的 serde 属性)。
2. serde 派生批:ScanOptions / ScanPhase / ScanProgress / ExecutionReport /
   ActionFailure + 字段类型闭包;serde 往返单测。
3. `ActionKind` / `OutcomeStatus` / `TargetOutcome` 新类型 + `ExecutionReport`
   扩展;Executor 填充 outcomes;一致性单测(计数 == 明细聚合)。
4. 可回收字节:目标容量口径从扫描侧带入执行报告;聚合字段 + 单测。
5. digest:`confirmation_digest` + `ExecutionRequest.expected_digest` +
   `StaleConfirmation` 错误;性质单测(稳定/敏感/过期拒绝)。
6. selected_ids 边界四条在 Executor 入口实现 + 各自单测。
7. 全量检查:`just ci`;复跑子任务 1 的 JSON 等价门并留档。

## 验证命令

```bash
just ci
# JSON 等价门:按 08-03-core-api-extraction 留档的 fixture 与比较命令复跑
```

## review 门

- 公开 API 命名(回收站口径)与错误枚举形态需人工 review 后再进 Phase 3。

## 回滚点

- 按步骤批次提交;任一批验收失败可独立 revert,前序批次不受影响。
