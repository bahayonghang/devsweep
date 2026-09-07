# P2 对齐五套 Harness 的规则和能力证据 — Design

## Evidence and scope

参见父任务 `research/audit-report.md`、`research/test-results.md` 和 `research/harness-matrix.md`。报告中的事实对应审查基线，实施前复核变动。

## Owning files

- AGENTS.md（只在 managed Trellis block 外更新）
- AGENTS 的具体修订：纠正 just dev；删除 `--allow-permanent-delete` 仍可传入的暗示（当前 flag 不存在）；补 `.trellis/spec/desktop-frontend/index.md` 的桌面开发路由，并保留 backend/TUI 分层。
- CLAUDE.md（继续引用 AGENTS，只在实际需要时改动）
- docs/agents/harnesses.md（新增，五工具矩阵、入口/权限/模型分工、可复现只读验证）
- docs/agents/domain.md（添加 harness 指南入口，如现有导航适合）

## Mechanism

推荐共享契约 + 已证实兼容读取的最小方案；不要求创建 .grok/.kimi-code/.omp 目录来追求形式一致。记录 Claude @ import、Codex AGENTS/.agents、Grok inspect 确认的兼容发现、Kimi/OMP 官方已支持的共享路径。原生 Trellis adapter 状态独立列出：当前仅部分平台配置；若以后要求原生适配，另行审查版本和变更，不本次自动升级。启动/分派由实际运行时工具 schema 决定；hooks 文本不等于生效，当前 Codex sandbox disabled 也不能称为隔离执行。

## Ordering and shared ownership

规则稿可先准备；最终验收在 CLI 文档、skill-integrity 之后，汇总其最终入口。无 product-code 修改。

## Tool and model assignment

Claude Code/Codex 强模型审查跨文件冲突；Grok 可作兼容加载的独立核验；Kimi/OMP 可承接冻结后的文档校对。任何 harness 的低成本执行者都不得自行改变权限、任务阶段、技能语义或扩大文件范围。

## Knowledge writeback

AGENTS.md + docs/agents/harnesses.md；矩阵每行明确 Claude Code / Codex / Grok Build / Kimi Code / OMP（Oh My Pi）。团队知识库本次不写，因为尚无批准结论。

## Rollback

保留实施前差异和生成文件哈希；仅回退本任务负责的变更，不恢复其他任务文件。若验证揭示新产品语义选择，暂停该依赖项并回到父任务修订批准范围。
