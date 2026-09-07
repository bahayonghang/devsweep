# P2 对齐五套 Harness 的规则和能力证据 — Implementation plan

## Approval and sequence

规则稿可先准备；最终验收在 CLI 文档、skill-integrity 之后，汇总其最终入口。无 product-code 修改。

1. 取得本父子任务最新方案的明确实施批准后，复核 Git/任务状态与本任务证据；仅激活本子任务。
2. 强模型确认 `design.md` 文件范围、事实和改造机制，分配独占文件责任。
3. 先保存最小失败证据/反例，再完成设计中的最小改动；不扩大权限或加入兼容旧根。
4. 按下面命令运行适用检查，保留命令、环境、退出码和未验证项。
5. 强模型独立核验每条 AC 的每个子句和回写；若低成本模型遇到跨模块语义/权限/两次失败，升级给主审，不继续堆修补。
6. 形成验收记录供父任务汇总。本方案不授权 push、安装应用、签名、发布或额外知识库写入。

## Required checks

- 逐一读取矩阵所有本地路径与已打开的官方原始来源，注明版本与核验日期
- python .trellis/scripts/task.py validate <each-child>；强模型审核需求→设计→检查闭环
- 静态核对 CLAUDE @AGENTS.md、managed block 哈希与项目被忽略路径；just check-skills
- 核对 AGENTS 不再把 `--allow-permanent-delete` 描述为现行选项；用当前 CLI help/拒绝契约校验；检查 backend、TUI、desktop-frontend 三个 spec 入口均有明确适用路径。
- 可用时 grok inspect --json、trellis platforms（只读）；其他 harness fresh-session discovery 请求只列出加载文件/任务，不启动实施或模型型付费批任务
- 明确区分“说明对齐通过”与“provider/hook实际执行未验证”；git diff --check

## Final evidence

记录 AC → 文件/行为 → 命令/输出映射。通过结构校验不等于实现验收；对 native/provider 等未运行项保留 UNVERIFIED，不能靠删除或弱化 AC 过关。
