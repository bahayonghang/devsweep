# 审计整改：执行安全闭环与可靠运行时

## Goal

基于《DevSweep 极度详细代码、稳定性、规则与架构审计》（`DevSweep_Deep_Audit_2026-07-27.md`）的整改父任务。本任务持有需求集与任务地图，负责跨子任务验收和最终集成评审，自身不承担实现工作。

整改总目标（沿审计 §15 策略）：

1. 先把执行功能收紧到"任何一次副作用都能证明其来源、对象、范围和用户确认"；
2. 再让所有扫描和命令可取消、有预算、有部分结果；
3. 然后补规则精度与三平台正确性；
4. 最后补许可证、CI 与可信发布链。

## 需求基线（冻结）

- 审计报告：`DevSweep_Deep_Audit_2026-07-27.md`
- 报告 SHA-256：`6440B6295FDC3DF109091C04A8359F59D089813252689238475C73C268F24EE2`（2026-07-28 复核确认）
- 代码复核基准：当前 dev HEAD（审计原基线 `02e0925` 落后约 4 周，行号引用已全部按当前 HEAD 重定位进各子任务 PRD）
- 审计报告与全部任务 PRD 已作为可审阅规划基线提交于 `3c9ca9c`；后续实现须以该提交后的当前 `dev` HEAD 为准。

## 复核结论摘要

2026-07-28 已对照当前代码逐条复核全部 29 项发现：

- **已解决、不立项**：F-24（tui.rs 已拆为 `src/tui/` 模块目录 + `sweep.rs` 管线）；F-15 接缝部分（`ExecutionRequest.selected` 已显式）。
- **部分改善、残留并入子任务**：F-16（RuleDoc 身份已收拢，pnpm 风险漂移与执行侧 registry 仍缺）；F-09（条目级错误已跳过，子目录级仍中止整根）；F-03（有 `Cancelling` 状态但无真实取消，终态可被复活）。
- **其余全部确认成立**，按主题归入 12 个子任务。F-29（.trellis 卫生）属工作流选择，有意识排除。

## 跨任务契约所有权（防返工环）

以下契约有唯一 owner，其余任务只消费、不得另造平行实现：

| 契约 | Owner 子任务 | 消费方 |
|---|---|---|
| plan schema v2、`ValidatedPlan`/`ValidatedManifest` 类型、canonical serialization、**digest 算法**、版本兼容策略 | 07-28-plan-validation | tui-race-hardening（确认框 digest 展示）、durable-audit（record 中的 plan digest）、scan-reliability（新增 JSON 字段） |
| **canonical `ActionFingerprint`**（footprint + action identity 的规范化身份，Windows 大小写/分隔符归一）与 "每 fingerprint 恰好执行一次" 不变量 + executor 防御性 once-ledger | 07-28-plan-validation | tui-race-hardening（scanner 级 dedupe 仅为展示层优化，不是安全边界）、executor |
| **Registry contract/core**（`RuleId → ActionSpec` 的稳定最小接口，含 provider action 重建）| 07-28-plan-validation（定义 contract + 最小 core） | 07-28-rule-registry-providers（在同一 contract 上扩展 detector/resolver/safety_contract，**不迁移不重建**） |
| `SafetyPolicy::authorize()` 漏斗与保护语义分类 | 07-28-central-safety-policy | executor、rule-registry-providers（safety_contract 字段） |
| `ProcessRunner` port 与 typed 进程诊断 | 07-28-process-runner-cancellation | true-cancellation、durable-audit（sanitized stderr）、rule-registry-providers（typed probe） |
| cancellation token 接口 | 07-28-true-cancellation（定义）；scan 侧先以 no-op token 预留 | scanner/fs_size/providers/executor |

## 任务地图与执行顺序（12 子任务）

| # | 子任务 | 优先级 | 审计条目 | 依赖 |
|---|---|---|---|---|
| 1 | 07-28-tui-race-hardening | P1 | F-02、F-07(UI层)、F-03(止血)、F-20 | 无 — 先行（Phase 0 止血） |
| 2 | 07-28-plan-validation | P1 | F-01、F-15(残留)、F-27 + 契约 owner | 无，可与 #1 并行 |
| 3 | 07-28-license-baseline | P1 | F-10 | 无外部依赖；MIT 基线已采用，可立即做 |
| 4 | 07-28-scan-reliability | P1 | F-09、F-26 | 独立可并行 |
| 5 | 07-28-central-safety-policy | P1 | F-04、F-05 | 后于 #2（消费 ValidatedAction）与 #6（有界 cargo metadata runner） |
| 6 | 07-28-process-runner-cancellation | P1 | F-06、F-13(进程上下文) | 独立可并行 |
| 7 | 07-28-durable-audit | P1 | F-08、F-22 | 后于 #2（digest）；stderr sanitize 接口与 #6 协商 |
| 8 | 07-28-true-cancellation | P1 | F-03(真取消) | 后于 #1（状态机约定）与 #6（runner/进程树） |
| 9 | 07-28-scan-walker-budgets | P2 | F-12、F-18、F-19 | 后于 #4（错误模型先立） |
| 10 | 07-28-tui-ux-reliability | P2 | F-11、F-21、F-25 | 独立；建议 #1 先合入 |
| 11 | 07-28-rule-registry-providers | P2 | F-16(残留)、F-14、F-17、F-13(残留)、F-28 | 后于 #2(contract)、#5(safety_contract)、#6(typed probe) |
| 12 | 07-28-release-supply-chain | P2 | F-23、发布配方 | 最后收口；required checks 待安全套件成形 |

## 决策台账（实施前须定案）

| # | 决策 | 状态 |
|---|---|---|
| D1 | v1 plan 兼容策略 | **已采纳默认**：直接拒绝并提示 rescan（不做迁移）；错误文案在 plan-validation design.md 确认 |
| D2 | audit 默认持久化等级 | **已采纳默认**：`action_started` 必须 sync 落盘先于副作用；`finished` flush + 尽力 sync，持久化失败 → outcome unknown 并停止后续 action（故障矩阵见 durable-audit PRD） |
| D3 | 排名语义（F-19） | **已采用**：保持 size-first 主序；`target_score` 改为显式命名的 freshness tiebreaker，仅在大小相等时排序。依据：size-first 是清理工具最可预期的空间收益 UX；年龄仍保留为稳定次序和新鲜度保护信号。 |
| D4 | 许可证选择 | **已采用 MIT**：`7300d8a` 已加入根 `LICENSE` 与 Cargo `license = "MIT"`。license-baseline 继续补齐其余分发 metadata、provenance 与 README。 |
| D5 | 确认期间的扫描更新 | **已采用**：立即使确认失效并要求重新确认。扫描结果不得与已打开的确认窗口并存为可执行的最新状态；Enter 必须被拒绝并提示重新确认。 |
| D6 | UserProtectionList 持久化与管理 | **已采用**：OS app-data 下版本化 JSON；`devsweep protect add|remove|list` 管理。add 只接受存在的 canonical 绝对路径，remove 对已不存在条目按规范化已存值匹配。 |
| D7 | TUI clean 成功后的 stale 行 | **已采用**：保留禁用 tombstone 行直到下一次 rescan；从选择、总量和执行集合移除，显示已清理状态和 rescan 提示。 |
| D8 | TUI 列表翻页 | **已采用**：提供标准 PgUp/PgDn 行为，光标始终位于可视窗口。 |
| D9 | mutation 运行时 TUI 退出 | **已采用**：只允许继续等待，或请求取消并等待 worker 确认；不提供默认或隐式 detach，未达终态不得退出。 |

## 启动门禁（Planning No-Go）

**当前状态：全部 12 个子任务处于 planning，不得对任何任务执行 `task.py start`。** 单个子任务的启动前置条件：

1. `prd.md` 定稿 + `design.md`（技术设计）+ `implement.md`（执行清单）齐备——`task.py validate` 通过只代表 JSONL 语法合法（当前全部为空模板），**不构成启动依据**；
2. `implement.jsonl`/`check.jsonl` 填入真实 spec/research 条目；
3. `implement.md` 必须包含**可执行验证矩阵**：per-defect fail-red 测试清单、`just ci`、平台特定动态验证项（Windows reparse/junction/Job Object、Unix process group、macOS resolver）、远端三平台 CI 引用，以及**取不到证据时的 No-Go 条件**（例如无权限创建 junction 时该项测试必须显式 fail/skip-with-reason，不得静默绿）；
4. 依赖的上游契约（见契约所有权表）已落地或已有书面接口约定。

## 跨子任务验收标准（沿审计 §13 量化指标）

- [ ] 未验证 plan 的执行入口 = 0；JSON 不再携带任意 program/argv/path 授权
- [ ] plan version 强制校验 100%；无效 manifest 在首个副作用前 fail closed
- [ ] 确认框 manifest 与执行 manifest 身份 100% 相同（canonical digest 判等，有回归测试证明）
- [ ] "每 canonical ActionFingerprint 恰好执行一次"是 ValidatedPlan 不变量 + executor once-ledger 双层保证（scanner dedupe 不计入安全边界）
- [ ] 所有副作用经唯一 `SafetyPolicy::authorize()`；path 类动作执行前 100% 实时重验证；合法 cleanup footprint（home 下全局缓存、scan root 下项目产物）不被保护语义误伤
- [ ] mutation job 并发上限 = 1
- [ ] 全部外部命令有 timeout 与输出上限；cancel 确认后下一 action 调用次数 = 0
- [ ] audit：started sync-before-side-effect；故障矩阵全路径有测试；崩溃后可重放
- [ ] incomplete/unknown estimate 100% 可见，且不参与默认选中
- [ ] CI 覆盖 3 OS；恶意 plan / TOCTOU / cancel / audit-failure 安全回归套件全部存在
- [ ] 稳定版执行功能开放前，审计 open P1 = 0

## 约束

- 子任务各自独立可验证、独立可提交；不做单个"safety rewrite"巨型 PR（沿审计附录 C）。
- 每个历史缺陷先提交 fail-red 回归测试，再提交修复；测试不得只验证 UI 文案或 mock 结构。
- `CleanupPlan` serde 格式变更只允许发生在 plan-validation 子任务内。
- 保留审计 §0.4 的已做对部分：默认 dry-run、argv 边界、permanent delete 硬禁用、symlink/reparse 跳过、Cargo home inspect-only、单 target 失败可继续。

## 明确不在本父任务范围（后续另行立项）

- 审计 §10 Wave 1–3 provider 扩展（.NET/VS/Go/uv/Conda/Docker/WSL/移动端/AI 生态）。
- 审计 §9.4 产品面新命令 `analyze` / `purge` / `doctor` / `history`。
- winget/scoop/Homebrew 分发、自更新、SBOM/签名/provenance（release 任务仅记录为 stretch）。
- F-29（.trellis 仓库卫生）。

## Notes

- 各子任务 PRD 中的文件行号一律以当前 HEAD 为准。
- 2026-07-28 二轮评审（Codex）结论已吸收：契约所有权表、启动门禁、决策台账、P1 归位（F-09/F-10）、三处任务拆分均源于该评审。
