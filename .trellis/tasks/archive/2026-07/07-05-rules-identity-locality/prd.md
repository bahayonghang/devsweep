# 规则身份收拢至规则表

## Goal

让每条过程式规则的 `RuleDoc` 与其实现同处声明，`rule_catalogue()` 变为纯聚合；提取共享的规则行格式化器供 CLI 与 TUI 两个 adapter 复用。规则身份（id、风险级、动作、摘要）在全库只有一份权威来源。

## 背景（评审证据）

- `rules.rs::rule_catalogue()`（314–417）对表驱动规则能真实派生文档；但对过程式规则（`rust.target`、`python.__pycache__`、npm/pip/pnpm/yarn 缓存、`cargo.home.inspect`、`docker`）是**手写复述**——这些规则实际产生于 scanner.rs（113–216）与 providers.rs（`add_npm_targets`…`add_yarn_target`，103–241），id/风险级/摘要在 2–3 个文件各有一份。
- 已存在测试 `project_dir_rules_match_legacy_scanner_ids` 专门守一小片漂移——重复是已知问题的证据。
- 行格式化重复：`main.rs::run_rules`（100–116）与 `tui::render_rules`（1918–1959）都在迭代 `rule_catalogue()` 并格式化 `id / scope / risk / action / summary` 行，一个输出 stdout，一个渲染 Rules 标签页。
- 按删除测试判断：`rule_catalogue()` 是遮蔽 locality 问题的浅聚合器——删掉它会把"列出所有规则"散到两个调用方，留着它则维持三处重复。

## Requirements

- 每条过程式规则在其实现处声明一份 `RuleDoc`（常量或等价机制，形状在实现时定夺），scanner/providers 产 target 时与文档共用同一来源的 id 与风险级。
- `rule_catalogue()` 改为纯聚合：表驱动规则照旧派生 + 过程式规则收集各自声明的 RuleDoc，不再有手写第二副本。
- 提取共享的规则行格式化函数（一处），`main.rs::run_rules` 与 `tui::render_rules` 作为两个 adapter 消费它（stdout 与 Rules 标签页各自只保留输出方式的差异）。
- `project_dir_rules_match_legacy_scanner_ids` 防漂移测试在收拢后应变得不必要——可删除或改写为对聚合完整性的正面断言（如"catalogue 覆盖所有产出 target 的 rule_id"）。

## Acceptance Criteria

- [ ] 每个 rule id 字符串（如 `"npm.cache.clean"`、`"cargo.home.inspect"`）在生产代码中只出现一次
- [ ] `rule_catalogue()` 中不再有手写的过程式规则 RuleDoc 文字
- [ ] 行格式化逻辑只有一处实现，CLI 与 TUI 两个 adapter 复用
- [ ] `devsweep rules` 输出与 TUI Rules 标签页信息完整（每条规则的 id/scope/risk/action/summary 仍全部可见）；行格式统一后 CLI 允许改为与 TUI 一致的分组布局（scope 由分组标题表达），此为本任务批准的展示层变更
- [ ] 防漂移测试被移除或改写为聚合完整性断言
- [ ] `cargo test` 全量通过

## 约束与顺序

- 独立任务，可与其他子任务并行；与 07-05-tui-module-split 有轻微文件重叠（render_rules），若并行需注意合并顺序。
- 轻量偏中等：PRD + 简短 design.md（RuleDoc 声明机制的选型）即可，implement.md 可选。

## Notes

- 词汇约定沿用 /codebase-design：规则身份获得 **locality**（与实现同处）；两个 adapter（stdout、Rules 标签页）证明格式化器是一条真**接缝**。
