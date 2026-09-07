# P1 保证 Skill 源与发现副本一致 — Design

## Evidence and scope

参见父任务 `research/audit-report.md`、`research/test-results.md` 和 `research/harness-matrix.md`。报告中的事实对应审查基线，实施前复核变动。

## Owning files

- justfile（install-skill 与新增只读 check-skills 配方，不扩大到 install-all）
- tools/skill_distribution.py（check/install 实现：相对文件集合、字节哈希、仓库内 dest 校验；check 不修复）
- tools/tests/test_skill_distribution.py（新增，标准库临时 fixture 测试同步/漂移/路径边界）
- skills/devsweep-inspect/README.md（可复用安装/校验说明、工具适用范围）
- skills/devsweep-inspect/manifest.json、reports/creation-handoff.md（如现有元数据涉及工具范围，最小同步）
- .agents/skills/devsweep-inspect/**、.claude/skills/devsweep-inspect/**（已授权实施阶段的项目生成副本，保持 ignored）

## Mechanism

沿用现有两条共享 discovery 根，补明确只读 check-skills 与幂等同步；不为每个 harness 复制一套业务正文。同步前解析并验证绝对目标在仓库内且是本 skill 目录，安全替换该目录；别碰其他 skills/全局目录。核对文件集合与哈希涵盖 source-only 七文件及陈旧额外文件。测试对临时复制件运行，生产发现目录仅在批准后更新。以现有脚本/eval 为契约，避免再做规则引擎。

## Ordering and shared ownership

在 validation-gates、release-contract 之后修改 justfile。harness-alignment 的最终发现矩阵依赖本任务修复，规则文案可以先准备。

## Tool and model assignment

Codex/Claude Code 强模型审查 source-of-truth、同步边界和清理授权；便宜模型只执行既定资源同步、哈希检查和文档更新。禁止便宜模型擅自改清理规则。

## Knowledge writeback

skills/devsweep-inspect/README.md 与项目 harness 指南；适用 Claude/Codex/Grok/Kimi/OMP 的已核实共享发现路径。

## Rollback

保留实施前差异和生成文件哈希；仅回退本任务负责的变更，不恢复其他任务文件。若验证揭示新产品语义选择，暂停该依赖项并回到父任务修订批准范围。
