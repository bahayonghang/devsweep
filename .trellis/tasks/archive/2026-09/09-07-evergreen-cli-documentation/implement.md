# P1 对齐现行 CLI 文档与证据入口 — Implementation plan

## Approval and sequence

可与 validation-gates 并行编辑文档，但 docs/ci.md 归前者独占。本任务不改 justfile 或 AGENTS.md；最终文档构建依赖前者准备就绪。

1. 取得本父子任务最新方案的明确实施批准后，复核 Git/任务状态与本任务证据；仅激活本子任务。
2. 强模型确认 `design.md` 文件范围、事实和改造机制，分配独占文件责任。
3. 先保存最小失败证据/反例，再完成设计中的最小改动；不扩大权限或加入兼容旧根。
4. 按下面命令运行适用检查，保留命令、环境、退出码和未验证项。
5. 强模型独立核验每条 AC 的每个子句和回写；若低成本模型遇到跨模块语义/权限/两次失败，升级给主审，不继续堆修补。
6. 形成验收记录供父任务汇总。本方案不授权 push、安装应用、签名、发布或额外知识库写入。

## Required checks

- cargo test --locked -p devsweep-cli --test cli_contract --test five_mode_contract
- npm run docs:build（先完成 validation-gates 的文档依赖前置）
- 逐例通过当前 --help 和隔离临时项目的 scan→plan→preview；execute 示例只检查参数契约，不执行
- rg -n 'devsweep (tui|scan|inventory|protect|rules)|--execute|--audit-log' README.md docs：人工逐项区分当前建议与迁移/拒绝示例，不设盲目字符串禁令
- 文件存在性、证据 commit/hash 的人工抽核；git diff --check

## Final evidence

记录 AC → 文件/行为 → 命令/输出映射。通过结构校验不等于实现验收；对 native/provider 等未运行项保留 UNVERIFIED，不能靠删除或弱化 AC 过关。
