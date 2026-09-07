# P1 修复本地与托管检查的失败传播 — Design

## Evidence and scope

参见父任务 `research/audit-report.md`、`research/test-results.md` 和 `research/harness-matrix.md`。报告中的事实对应审查基线，实施前复核变动。

## Owning files

- justfile（fmt/check/test/clippy/ci、desktop-web-check；不占用 dev/release/install-skill 配方）
- .github/workflows/ci.yml（桌面步骤拆分或显式检查退出码；生成漂移检查；文档构建）
- desktop/scripts/generate-types.mjs（`--check`：覆盖前比较已提交 types.gen.ts，不写入）
- tools/tests/test_validation_gates.py（新增，标准库隔离假命令与锁文件保真回归）
- docs/ci.md、docs/zh/ci.md（本地/托管 gate 范围、Node/npm 前置和依赖安装说明）

## Mechanism

将 desktop-web-check 拆成有顺序且可单独失败的 recipe/步骤。Windows 原生外部命令必须检查退出码，不能依赖分号链的最后结果。ci 移除 sync-lock 依赖。类型一致性复用已有 types-generation.test.ts/--stdout 路径，在生成覆盖前比较；不再另造 schema。托管桌面检查用独立步骤或明确退出；增加独立 docs 构建步骤/作业，使用现有锁定依赖。

## Ordering and shared ownership

可先实施。与 release-contract、skill-integrity 都修改 justfile，三者必须顺序实施；先本任务，再发布入口，再 skill 同步。

## Tool and model assignment

Codex 强模型主审 Windows 退出码与锁文件边界；Claude Code 强模型可独立复核 CI 覆盖。便宜模型可按冻结清单拆 recipe/YAML、补隔离回归；根因判断与最终失败注入验收保留强模型。

## Knowledge writeback

docs/ci.md + docs/zh/ci.md；适用全部五套 harness，说明 PowerShell 特有失败传播要求。

## Rollback

保留实施前差异和生成文件哈希；仅回退本任务负责的变更，不恢复其他任务文件。若验证揭示新产品语义选择，暂停该依赖项并回到父任务修订批准范围。
