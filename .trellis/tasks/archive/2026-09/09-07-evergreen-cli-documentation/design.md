# P1 对齐现行 CLI 文档与证据入口 — Design

## Evidence and scope

参见父任务 `research/audit-report.md`、`research/test-results.md` 和 `research/harness-matrix.md`。报告中的事实对应审查基线，实施前复核变动。

## Owning files

- README.md、code_map.md
- docs/index.md、docs/zh/index.md
- docs/guide/getting-started.md、scan.md、safety-model.md、tui.md、clean.md（逐一核对）
- docs/zh/guide/getting-started.md、scan.md、safety-model.md、tui.md、clean.md（逐一核对）
- docs/guide/inventory.md、protection.md、software.md、optimize.md（现行指南面，避免留下旧根或不一致的 V1 审计路径）
- docs/zh/guide/inventory.md、protection.md、software.md、optimize.md
- docs/guide/cli-migration.md（保持标注的旧→新对照表，不是现行教程）
- docs/reference/plan-and-report.md、docs/zh/reference/plan-and-report.md
- docs/validation/five-mode-native.md（修复已归档 evidence 路径，保留历史状态）

## Mechanism

以 application/cli.rs、现有 cli_contract 测试和 docs/guide/cli-migration.md 为命令证据。直接重写旧段落，避免旧/新教程拼接。安全文字先由强模型冻结事实表，再由有界执行者同步两种语言。地图改向实际 CLI modes/application output 与 core analysis/software/optimize/status/history；不重命名产品模块。证据路径改指 archive/2026-09/08-29-five-mode-native-integration，旧 PASS 不转写为本轮 HEAD 已验证。

## Ordering and shared ownership

可与 validation-gates 并行编辑文档，但 docs/ci.md 归前者独占。本任务不改 justfile 或 AGENTS.md；最终文档构建依赖前者准备就绪。

## Tool and model assignment

Claude Code 或 Codex 强模型审查完整命令与权限事实；Kimi Code/OMP 的低成本可选模型适合按冻结事实表同步双语文档、路径和代码地图；强模型最终逐例复核。

## Knowledge writeback

README/docs/code_map 本身即为项目知识回写；适用全部五套 harness。

## Rollback

保留实施前差异和生成文件哈希；仅回退本任务负责的变更，不恢复其他任务文件。若验证揭示新产品语义选择，暂停该依赖项并回到父任务修订批准范围。
