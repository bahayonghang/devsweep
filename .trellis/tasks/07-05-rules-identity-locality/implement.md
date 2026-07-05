# Implement: 规则身份收拢至规则表

按 design.md 执行；每步验证后前进。

## 步骤

1. **scanner.rs**：声明 `RUST_TARGET_RULE_DOC` / `PYCACHE_RULE_DOC` / `SCANNER_RULE_DOCS`；实现改用 `DOC.id`（含 evidence 字符串）。
   - 验证：`cargo test scanner`
2. **providers.rs**：五个 doc 常量 + `PROVIDER_RULE_DOCS`；`add_*_target` 改用 `DOC.id`（risk 与 doc 同源处取 doc 字段）。
   - 验证：`cargo test providers`
3. **rules.rs**：`rule_catalogue()` 改纯聚合（顺序不变）；新增 `rule_row` 与 `risk_label`；模块注释更新；删除旧防漂移测试，新增 `catalogue_covers_all_declared_docs`。
   - 验证：`cargo test rules`
4. **两个 adapter**：tui/render.rs `render_rules` 改用 `rules::rule_row`/`rules::risk_label`（渲染输出不变，相关 render 测试应全绿）；main.rs `run_rules` 改分组布局。
   - 验证：`cargo test`；`cargo run -- rules` 人工核对分组输出
5. **收口**：`rtk proxy grep -rn '"rust.target"\|"npm.cache.clean"\|"python.__pycache__"' src/` 生产代码各仅 1 处（doc 常量内；测试断言除外）。
   - 验证：`cargo fmt --all -- --check && cargo test && cargo clippy --all-targets -- -D warnings`

## 回滚点

- 步骤 1–3 与 4 可分段回滚；全部完成后一次性提交。

## 评审门

- 步骤 5 后对照 prd.md 验收清单（注意 CLI 分组布局为已批准变更），进入 Phase 3。
