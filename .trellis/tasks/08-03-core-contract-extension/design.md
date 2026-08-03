# 技术设计:GUI 支撑契约

> 细化父任务 `design.md` §2.1。全部增量,无破坏性变更。

## 1. serde 策略

- 派生加在 core 类型本体上,不建平行 DTO(父任务决策)。
- 字段命名保持 Rust snake_case 序列化(与现有 `ScanReport` JSON 一致);
  前端侧统一适配,不在 core 混用 rename_all = camelCase(避免与既有
  `--json` 输出风格分裂)。实现时核对 `ScanReport` 现状后对齐,结论留档。

## 2. ExecutionReport 扩展(草案,实施时以代码核对为准)

```rust
pub struct TargetOutcome {
    pub target_id: TargetId,
    pub action: ActionKind,      // Command{irreversible} | MoveToTrash | InspectOnly
    pub status: OutcomeStatus,   // Succeeded | Failed{message} | Skipped{reason}
    pub estimated_bytes: CapacityEstimate,  // 复用扫描侧 verified/partial/unknown 口径
}
pub struct ExecutionReport {
    …现有字段保持…,
    pub outcomes: Vec<TargetOutcome>,
    pub estimated_recoverable: CapacityTotals,  // 聚合;回收站口径
}
```

- `ActionKind` 是 `CleanAction` 的对外投影(不暴露 argv/path 细节之外的内部
  构造);irreversible 标志必须透出。
- 兼容:`failures` 字段保留(由 outcomes 派生或双写,实现时定,保持既有
  消费方不改)。

## 3. 确认 digest

- 输入:validated manifest 的规范化序列化 + 排序后的 selected TargetId 集。
- 算法:复用 `plan/digest.rs` 的 SHA-256 基础设施。
- API 形态(核心层,供 CLI/TUI/Tauri 共用):
  - `confirmation_digest(&ValidatedPlan, &[TargetId]) -> ConfirmationDigest`
  - `ExecutionRequest` 增加 `expected_digest: Option<ConfirmationDigest>`;
    `Some` 且不匹配 → `ExecutionError::StaleConfirmation`(结构化,不 panic)。
    CLI 现有路径传 `None`,行为不变;Tauri 侧强制 `Some`。

## 4. selected_ids 边界

在 `Executor` 入口统一实现(而非 Tauri 层),CLI/TUI/Tauri 共享同一行为:
未知 id → `ExecutionError::UnknownTarget`;重复 → 去重 + 报告注明;
inspect-only 被选中 → `ExecutionError::InspectOnlyTarget`;
irreversible → `TargetOutcome.action` 携带标志。

## 5. 回滚

- 全部为新增字段/新 API + `Option` 兼容位;失败按提交批次 revert,
  不影响子任务 1 成果。
