# Plan 信任边界：fail-closed 校验与类型化 ActionRegistry

> 父任务：07-28-audit-remediation ｜ 审计条目：F-01、F-15（残留）、F-27 ｜ 对应审计 Milestone M1 / PR3+PR4
> **本任务是以下跨任务契约的唯一 owner**：plan schema v2、ValidatedPlan/ValidatedManifest、canonical serialization 与 digest、canonical ActionFingerprint、registry contract/core。

## Goal

把外部 plan 从"可执行命令包"改成"声明式意图"：JSON 只保存 intent 与事实证据，执行时由受信任的 registry 重建动作。消除"被替换/手改/来路不明的 plan JSON 可运行任意本地程序、或展示 A 移动 B"这一最大安全缺口，并为全体子任务提供 manifest 身份与去重不变量的单一定义。

## 背景与证据（当前 HEAD）

1. `run_clean` 对 plan 只做 serde 解码即进入 executor（`src/main.rs:53-70`）；`version` 字段被写出但从不校验。
2. `CleanAction::Command` 允许 plan 自由提供 `program`/`args`/`cwd`（`src/model.rs:99-114`）；`MoveToTrash` 允许自由路径。argv 虽不过 shell，但任意本地程序可被调起。
3. 展示/守卫与执行对象可不一致：self-exe guard 检查 `CleanTarget.path`（`src/executor.rs:260`），真正 Trash 用 `CleanAction::MoveToTrash.path`（`executor.rs:283-284`）。
4. `risk`/`reversible`/`evidence`/`scope`/`rule_id` 均不参与授权（F-15 残留）。
5. CLI 暴露 `--allow-permanent-delete`（`src/cli.rs:63-65`），executor 完全忽略（F-27）。
6. **重复执行的安全防线缺位**（二轮评审 SEC-002）：外部 plan 完全绕过 scanner，scanner 端 dedupe（tui-race-hardening 修复的展示层问题）对 plan 路径无效；同一 footprint 以不同 ID/等价路径写法（Windows 大小写、`\` vs `/`、尾分隔符）重复出现时，executor 会逐项执行。
7. **manifest 身份无定义**（二轮评审 ARCH-003）：父任务要求"确认 digest = 执行 digest"，TUI 与 durable-audit 都要消费 digest，但至今没有 canonical serialization/digest 定义。

## Requirements

### A. 类型化信任转换（F-01）

1. `UntrustedPlan -> ValidatedPlan` 类型强制转换；executor 只接受 `ValidatedPlan`（编译期不可绕过）。
2. 类型化意图 schema（v2）：`TrashProjectArtifact { rule_id, target_id }` / `RunBuiltInAction { provider_id, action_id }` 等；JSON 不再携带任意 `program`/`args`/`cwd`。
3. **Registry contract/core**：定义稳定的 `RuleId → ActionSpec` 最小接口并实现最小 core（覆盖现有 13 条项目规则 + 5 provider + 8 全局规则的动作重建）。此 contract 即后续 07-28-rule-registry-providers 扩展 detector/resolver 的同一接口——**一次定型，后续只扩展不迁移**（二轮评审 ARCH-002）。
4. 校验不变量（全部 fail closed，首个副作用前失败）：exact version dispatch、target.path 与 action path 同一性、绝对路径、scope containment、Noop 目标不可被选执行、risk/action/reversible 一致性、未知 rule/action id 拒绝。
5. **v1 plan 兼容策略（决策 D1，已采纳默认）**：直接拒绝并提示 "plan format v1 is no longer accepted; re-run `devsweep scan --json`"；不做迁移。

### B. Manifest 身份与去重不变量（二轮评审 ARCH-003 / SEC-002）

6. **canonical serialization + digest**：定义 ValidatedPlan/ValidatedManifest 的规范化编码（字段排序、路径归一）与 digest 算法（如 SHA-256），作为唯一身份定义；TUI 确认框展示、durable-audit record、执行前比对全部消费此定义。
7. **canonical `ActionFingerprint`**：footprint（canonical path，Windows 大小写/分隔符/尾缀归一）+ action identity 的规范化身份；ValidatedPlan 不变量 = 无重复 fingerprint（等价路径写法的重复在校验期 fail closed 或合并——语义在 design.md 定案）。
8. **executor 防御性 once-ledger**：执行循环内维护已执行 fingerprint 台账，重复 fingerprint 到达时拒绝执行并记录——即使上游校验被未来代码路径绕过，同一 footprint 也不会二次副作用。

### C. CLI 面（F-27）

9. 移除 `--allow-permanent-delete` flag；`clean --help` 无残留；README 同步。
10. dry-run 与 execute 消费同一 `ValidatedPlan`；execute 相对 preview 只能缩小选择，不能扩大。

## Acceptance Criteria

- [ ] 恶意 plan 套件：注入 `cmd.exe`、PowerShell、`/bin/sh`、额外 args —— runner 调用次数为 0
- [ ] `target.path=A`、`action.path=B` 在第一个副作用前失败，错误指明字段不一致
- [ ] version 0/1/未知 version、相对路径、未知 action/rule id 均 fail closed，退出码非 0；v1 拒绝文案含 rescan 指引
- [ ] fingerprint 套件：同 path 不同 ID、大小写变体（Windows）、`\`/`/` 变体、尾分隔符变体 —— 校验期拒绝或合并，runner 每 fingerprint 至多 1 次；executor once-ledger 单测（直接喂重复 fingerprint 的 ValidatedPlan 构造）证明第二次被拒
- [ ] digest 契约测试：同一逻辑 plan 的不同 JSON 字段顺序 → 同一 digest；任一 target/action 变化 → digest 变化
- [ ] executor 公共接口在类型上不再接受未验证 DTO（编译期保证）
- [ ] `--allow-permanent-delete` 从 CLI 消失
- [ ] 现有合法路径（scan → 保存 plan → dry-run → execute）行为不回归；`just ci` 全绿

## 约束与依赖

- 本任务是唯一允许变更 `CleanupPlan` serde 格式的子任务；digest/fingerprint/registry contract 一经定义即冻结接口，下游（TUI/audit/rule-registry）只消费。
- 与 07-28-tui-race-hardening 无硬依赖可并行；race-hardening 的 manifest 冻结先用结构等价判身份，本任务落地后其 digest 展示切换到 canonical digest（见该任务 PRD）。
- `SafetyPolicy`（protected roots、live revalidation）不在本任务，归 07-28-central-safety-policy。
- 审计预估 4–7 工程日；须先写 fail-red 恶意 plan / fingerprint 测试再实现。
