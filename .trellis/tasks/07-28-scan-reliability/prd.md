# 扫描部分失败模型：child 降级与 size 未知语义

> 父任务：07-28-audit-remediation ｜ 审计条目：F-09（审计定级 **P1 稳定性**）、F-26 ｜ 对应审计 Milestone M0 / PR11
> 二轮评审 CORR-002/ARCH-004 修正：F-09 按审计原级归位 P1 并独立成任务；遍历预算/剪枝/排名（F-12/F-18/F-19）拆至 07-28-scan-walker-budgets（P2）。

## Goal

统一 scan 与 size 的错误模型为"root error / child warning / partial estimate"三层：不可读子目录不再中止整根，size 错误不再无痕变成 0，incomplete/unknown 不参与默认选中；并消除 `scan --global` 的误导性摘要。

## 背景与证据（当前 HEAD）

1. **不可读子目录中止整根（F-09）**：条目级错误已跳过（`src/scanner.rs:119-127`），但递归 `self.scan_dir(&path, ...)?`（`scanner.rs:129`）内的 `symlink_metadata(dir)?`（`:73-74`）与 `fs::read_dir(dir)?`（`:116-118`）失败仍向上传播——一个权限受限的子目录使整根失败。这与仓库规范 `.trellis/spec/backend/error-handling.md` 的承诺（"Inaccessible nested entries are skipped so one unreadable child does not abort the whole scan"）直接矛盾。
2. **size 错误伪装成 0（F-09）**：`estimate_tree` 对 metadata/read_dir 失败返回 `(0, None)` 或 `(0, self_mtime)`（`src/fs_size.rs:9-27`），无 complete/partial 标记；UI 显示为可信的 "0 B"。同类 I/O 错误在 discovery 中可能中止一切、在 size 中被吞掉——两个方向都错。
3. **unknown mtime 保留默认选中**：freshness guard 对无 mtime 的 target 不生效（`src/ranking.rs:35-37`），incomplete 目标可被默认选中执行。
4. **误导性摘要（F-26）**：`scan --global` 不扫 roots，但摘要仍打印 `command.roots.len()`（默认 `.` → "from 1 root(s)"，`src/main.rs:37-42`）。

## Requirements

1. `ScanOutcome { candidates, diagnostics, completeness }`：root 级失败仍是 hard error；child 目录级失败降为 diagnostic 并继续 siblings；JSON 与 TUI 呈现 partial 状态；spec 承诺与实现对齐。
2. `SizeEstimate { logical_bytes: Option<u64>, complete: bool, warnings }`：错误 → unknown/incomplete，绝不显示可信 0；UI 明示 "≥ X（incomplete）" 或 "unknown"。
3. incomplete/unknown 的 target 不参与默认选中（扩展 freshness guard 的语义或新增 completeness guard）。
4. `scan --global` 摘要不再报告未扫描的 roots 数。
5. 本任务**不做**遍历预算/剪枝/排名变更（归 scan-walker-budgets），错误模型接口预留 cancellation 检查点位（no-op token）。

## Acceptance Criteria

- [ ] fixture：不可读子目录 —— sibling 结果保留，JSON/TUI 显示 partial + diagnostic；`.trellis/spec/backend/error-handling.md` 与实现一致
- [ ] fixture：size 遍历中途权限错误 —— 显示 incomplete/unknown 而非 0 B；该 target 不被默认选中
- [ ] fixture：root 本身不可读 —— 仍为 hard error（语义不放宽）
- [ ] `scan --global` 文本输出无 "from 1 root(s)" 误导；`scan --json` 新增字段经由 plan-validation 的 schema v2 契约（不得私改 serde 格式）
- [ ] `just ci` 全绿；先有 fail-red（当前不可读子目录导致整根失败的用例）再修复

## 约束与依赖

- 独立可并行；`ScanOutcome`/`SizeEstimate` 是 scanner/providers/sweep 的公共契约变更，TUI 与 JSON 同步适配；涉及 plan JSON 字段的部分消费 plan-validation 的 schema v2 所有权。
- Windows 权限 fixture（拒绝 ACL）与 Unix chmod fixture 均需真实生效验证；无法构造时 implement.md 标注 No-Go 条件。
- 审计预估 2–3 日。
