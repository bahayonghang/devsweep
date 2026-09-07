# 常青项目审查与五套 Harness 对齐 — Design

## Decision

采用“共享事实源 + 有证据的工具差异 + 可失败的工程门禁”。不重构业务算法，不添加兼容旧 CLI，不建立通用 harness 平台；本轮仅规划。

## Requirement-to-mechanism traceability

| 父需求/AC | 机制/交付 | 实施责任 |
| --- | --- | --- |
| R1 / AC1 | research/audit-report.md 的结构/F2/F5/F7；真实命令定义和历史证据路径 | cli-documentation、release-contract |
| R2 / AC2 | research/test-results.md 的命令/退出码/失败复现/历史根因；F1/F3；未来五处失败注入和锁哈希 | validation-gates、release-contract |
| R3 / AC3 | research/harness-matrix.md 的官方/文件/发现/执行分级；F4/F6 | skill-integrity、harness-alignment |
| R4 / AC3 | 每个子 design 的工具与强/低成本分工，固定输入/文件/检查、失败升级 | 全部子任务 |
| R5 / AC4 | 五个独立子任务、PRD/design/implement、真实 JSONL、顺序与独占范围 | 父编排 |
| R6 / AC5 | 下面知识回写表；只写已批准且验收支持的事实，注明工具 | 各子任务+父集成 |
| R7 / AC6 | planning 状态；无任务激活；最终 Git diff 只含新任务目录 | 父检查 |
| R5 / AC7 | backend/quality-guidelines.md 的 CI/release 场景；最终共享 clean/custom target 与 Windows/Unix 动态 process-runner 验证 | 父集成，gate/release/skill 子共享 |

## 子任务与顺序

1. 09-07-evergreen-validation-gates（P1）。
2. 09-07-evergreen-cli-documentation（P1，可与 1 独立编写）。
3. 09-07-evergreen-release-contract（P1，justfile 在 1 后）。
4. 09-07-evergreen-skill-integrity（P1，justfile 在 3 后）。
5. 09-07-evergreen-harness-alignment（P2，最终矩阵在 2/4 后）。

父子关系只表达归属，不自动提供依赖；此顺序由 implement.md 和各子 design 明示执行。

## Owning files

父任务不是产品实施目标。可写范围仅 `.trellis/tasks/09-07-evergreen-five-harness-audit/` 下规划与证据。产品文件由已归档子任务的 `trellis-implement` 修改。

## Shared-file ownership

- justfile：gate 子任务只负责检查/ci；release 负责 dev/release；skill 负责 install/check-skills，严格顺序交接。
- docs/ci.md、docs/zh/ci.md：gate 先更新验证段，release 后更新 release 段；CLI docs 子不编辑。
- AGENTS.md / CLAUDE.md：harness 子独占；保持 managed Trellis block。
- docs/agents/harnesses.md：harness 子独占；消费其他子结果。
- .agents/.claude 的本地 skill 副本：skill 子独占，在批准前不刷新；继续 ignored。
- 主线程只负责任务文件、集成、批准和验收；派生代理不扩授权。

## Knowledge writeback

| 结论 | 回写位置 | 适用工具 |
| --- | --- | --- |
| CI 失败传播/锁/Node/npm/证据卫生 | docs/ci.md + docs/zh/ci.md | 五套；PowerShell 差异单列 |
| 现行命令/审计权限/地图 | README、docs 指南、code_map | 五套 |
| 版本/归档 smoke | docs/ci 的 release 段、版本检查 | 五套编排；Windows 执行 |
| skill 源/同步/全局 binary | skills/devsweep-inspect/README 与分发副本 | 已核实 shared discovery 路径 |
| harness 模式/权限/模型分工 | AGENTS + docs/agents/harnesses.md | 逐行明确五套工具 |

项目说明和 skill 库已能承载本次批准结论；团队知识库/用户原生记忆不必另写，本轮更不提前写。若用户后续另选知识库，先检索已有笔记再更新，禁止复制原生记忆。

## Evidence and bounds

568 Rust、181 desktop、4 skill fixtures 通过；本轮没有产品源码红测试。失败来自真实旧命令/桌面 gate 假绿/缺文档依赖。Hosted 当前 HEAD 绿；历史失败不重复安排修复。未经运行的 native/provider 行为一直保留 UNVERIFIED。

实施阶段的 mandatory process-runner/clean-target 证据不同于可保留缺口的 provider discovery。共享门禁记录于 `research/parent-release-gates.md`：`just ci` 与独占 `--target-dir` workspace 测试在产品 HEAD `93dc07c` 上退出 0；Windows process-runner 退出 0；Unix 动态树终止 **UNVERIFIED**，不宣布该项完成；本次 `dist/` 归档被 ignore。不得拿审查基线的旧 hosted run 替代该证据。

## Rollback and scope expansion

实施逐子验证，不批量大改。若要升级 Trellis、改变全局配置、引入新依赖、增加原生 adapters 或修改清理语义，提交具体差异与理由另行批准；默认方案不包含这些项。
