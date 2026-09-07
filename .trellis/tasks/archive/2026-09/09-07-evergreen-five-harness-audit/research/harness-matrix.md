# 五套 Harness 能力与本项目对齐矩阵

核验日期：2026-09-07。OMP 在本报告中指 Oh My Pi。
Harness 是工具/权限/上下文执行环境，模型是推理能力和成本选择；下面的分工是针对本项目的建议，不是通用质量排名或价格承诺。

本机版本和 discovery 输出摘要见 harness-observations.md；其中 Codex CLI 版本只是安装观察，不据此推断当前桌面会话使用相同二进制版本。

## 能力、现有入口与证据

| 工具 | 能力边界和适合的规划/审查 | 本项目现状 | 低成本执行安排 |
| --- | --- | --- | --- |
| Claude Code | CLAUDE @ import、skills、具名 subagents、hooks；适合跨文档冲突、安全契约和 skill 语义审查 | CLAUDE.md 引 AGENTS；本地 .claude assets 存在且 ignored；安装健康已检查；项目模型/hook 握手 UNVERIFIED | 冻结事实后的双语文档/skill 资源同步；给定角色工具范围，不让执行者自行批准 |
| Codex | AGENTS 分层、.agents skills、可选模型/effort 的 subagents、原生 Windows shell；适合本机 Rust/PowerShell/发布根因与最终验收 | 当前会话收到规则/阶段上下文且强模型子代理实际完成审查；其他 fresh-session/具体 hook 路径仍应单列 | recipe/YAML/元数据变更可以下放，失败注入与清理权限主审保留强模型 |
| Grok Build | AGENTS/Claude compatibility、subagents、plan/permissions；适合独立规则发现和方案反证 | Grok inspect 已发现 shared AGENTS/skills/.claude agents/hooks；无 .grok 原生 Trellis 目录；实际 agent/hook 执行 UNVERIFIED | 只在实际运行时支持所选模型与工具时承接有界文档/测试；plan 标签不替代只读权限 |
| Kimi Code | AGENTS/shared skills、agents/subagents/hooks；适合文件证据梳理和冻结规格后的文档维护 | CLI 已安装；无原生 .kimi-code 适配；项目运行握手 UNVERIFIED，不能沿用旧模板“无 hooks”的说法 | 独立会话/已验证模型入口选成本档；不能假定导入 Claude agent 的 model 字段会生效 |
| OMP / Oh My Pi | shared rules/skills、task agents 与 role/model routing；适合明确的强审查→便宜执行交接 | CLI 已安装；无项目 .omp 原生角色；实际项目运行握手 UNVERIFIED | 固定 role 可选较低成本模型运行复制/文档/确定性检查；角色解析与权限先验证 |

Codex 当前任务环境是 danger-full-access / sandbox disabled：即使产品支持 Windows sandbox，本轮也不能把它记作“已在沙盒隔离中执行”。Grok 官方说明 plan mode 仅 gate edit tools，bash 仍可写，子代理也不受父 plan edit gate 自动保护。所有委派必须带明确只读/允许文件范围。

## 共同契约与最小对齐选择

1. AGENTS.md 为项目规则源；CLAUDE.md 保留 @AGENTS.md。工具局部入口只写差异/启动方法，不复制安全契约。
2. skills/devsweep-inspect 为 skill 源；.agents/.claude 的项目副本受 check-skills 约束。Grok discovery 已证实兼容消费；Kimi/OMP 官方共享入口仍须在该机新会话确认。
3. 保留当前 Trellis 0.6.12 模板。trellis platforms 只证明原生生成资产识别；本机 0.7.0-beta.3 的 init 支持额外 flag 不证明项目已配置。
4. 优先兼容加载/明确手动读取工作流；本次不为对称性创建 .grok/.kimi-code/.omp，也不进行全局 hook/模型变更。以后若必须要原生具名 Trellis 角色，另行审查薄适配。
5. 五工具都记录 official → local file → discovery → execution。只有第 4 级才能称相应行为实际验证；矩阵可在准确保留缺口的前提下完成文档对齐，不能宣称五套运行全通过。

## 问题到工具/模型的分配

| 工作 | 规划/审查 | 可下放部分 | 必须升级的边界 |
| --- | --- | --- | --- |
| Gate 假绿/锁文件 | Codex 强主审，Claude 强复核 CI | recipe 分拆、步骤命名、已设计 fake-command 测试 | 退出码/锁语义不明或跨平台分歧 |
| CLI/中英文说明 | Claude/Codex 强冻结命令和安全事实 | Kimi/OMP 或其他可选便宜模型按清单改路径/文案 | 要改 CLI/API 或放宽权限 |
| 发布/version | Codex 强审 Windows 产物 | 根版本同步、固定测试脚本 | 临时路径、执行真实清理、包选择或安装 |
| Skill v1/v2 漂移 | Claude/Codex 强审语义，Grok discovery 反证 | 无模型文件同步/哈希优先；便宜模型补说明 | 重写清理流程、用户全局更改 |
| Harness 规则冲突 | 强模型跨源审查，当前 Codex 编排 | 每行按已证实事实补链接/日期/状态 | 运行时不支持、模型静默降档、启用 hooks/安装工具 |

最省成本的执行者首先是确定性脚本。只有需要文本/代码编辑才选较低成本模型；没有本轮实测 tokens/费用，不给节约比例。任何 executor 失败不能放松验收；回到强模型分析。

## 官方资料（已打开核对）

- [Claude Code features](https://code.claude.com/docs/en/features-overview)、[memory / CLAUDE imports](https://code.claude.com/docs/en/memory)：规则加载、skills/subagents/hooks 的职责。
- [Codex AGENTS](https://learn.chatgpt.com/docs/agent-configuration/agents-md)、[skills](https://learn.chatgpt.com/docs/build-skills)、[subagents](https://learn.chatgpt.com/docs/agent-configuration/subagents)、[Windows sandbox](https://learn.chatgpt.com/docs/windows/windows-sandbox)：分层规则/发现/模型配置与运行环境边界。
- [Grok skills and compatibility](https://docs.x.ai/build/features/skills-plugins-marketplaces)、[plan caveats](https://docs.x.ai/build/features/plan-mode)：兼容读取和 plan 不等于 shell 只读。
- [Kimi agents](https://www.kimi.com/code/docs/en/kimi-code-cli/customization/agents.html)、[hooks](https://www.kimi.com/code/docs/en/kimi-code-cli/customization/hooks.html)：自定义 agents/实际前端字段与 hook 控制。外来 model 字段被忽略，不是可移植的廉价路由。
- [Kimi skills](https://www.kimi.com/code/docs/en/kimi-code-cli/customization/skills.html)：Skill Locations 明确项目根 `.kimi-code/skills/` 和 `.agents/skills/`，支持共享发现的依据。
- [OMP task agent discovery](https://github.com/can1357/oh-my-pi/blob/main/docs/task-agent-discovery.md)：agent model/role precedence 与实际 dispatch 解析。
- [OMP context files](https://github.com/can1357/oh-my-pi/blob/main/docs/context-files.md)、[OMP skills](https://github.com/can1357/oh-my-pi/blob/main/docs/skills.md)：共享规则与 skill 发现的专门依据；此为官方能力，项目会话仍需验证。

能力随工具版本变化；这份矩阵是核对日快照，不替代新会话的实际 tool schema 和权限。
