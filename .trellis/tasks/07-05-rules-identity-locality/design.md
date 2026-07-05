# Design: 规则身份收拢至规则表

## 现状

- 表驱动规则（PROJECT_DIR_RULES / GLOBAL_CACHE_RULES*）：`rule_catalogue()` 真实派生，无重复——保持不变。
- 过程式规则的身份重复：
  - `rust.target`：rules.rs:318–325 手写 doc；scanner.rs `scan_rust_project` 硬编码 id 字符串（target 构造 + evidence）。
  - `python.__pycache__`：rules.rs:340–347 手写 doc；scanner.rs `scan_dir` 硬编码两处字符串。
  - npm/pip/pnpm/yarn/cargo.home.inspect：rules.rs:350–388 手写 doc；providers.rs 各 `add_*_target` 硬编码 id。
  - `docker`：仅 rules.rs 占位（无实现可同处），保持原地。
- 行格式化重复：main.rs `run_rules`（flat 表 + SCOPE 列）与 tui/render.rs `render_rules`（分组标题 + 行）各写一遍。

## 方案

### 1. RuleDoc 与实现同处声明

scanner.rs（实现旁，靠近使用点）：

```rust
pub(crate) const RUST_TARGET_RULE_DOC: RuleDoc = RuleDoc { id: "rust.target", ... };
pub(crate) const PYCACHE_RULE_DOC: RuleDoc = RuleDoc { id: "python.__pycache__", ... };
pub(crate) const SCANNER_RULE_DOCS: &[RuleDoc] = &[RUST_TARGET_RULE_DOC, PYCACHE_RULE_DOC];
```

providers.rs 同理：`NPM_CACHE_RULE_DOC` / `PIP_CACHE_RULE_DOC` / `PNPM_STORE_RULE_DOC` / `YARN_CACHE_RULE_DOC` / `CARGO_HOME_RULE_DOC` + `PROVIDER_RULE_DOCS: &[RuleDoc]`。

- 实现代码改用 `DOC.id`（target rule_id 与 Evidence::RuleMatched 均引用常量字段），id 字符串全库唯一出现一次。
- 风险等级同源：`add_*_target` 构造 target 的 `risk` 与 doc.risk 一致处取 `DOC.risk`（需为 RiskLevel 加 `Copy` 或使用 clone——RiskLevel 已 Clone，若无 Copy 则 `DOC.risk.clone()`；RuleDoc 为 const 需全字段 const 兼容，Ecosystem/RiskLevel 均为纯枚举，可行）。
- 前提调整：`RuleDoc` 字段类型如 `Ecosystem`/`RiskLevel` 需支持 const 构造（纯枚举天然支持）。

### 2. rule_catalogue() 变纯聚合

```rust
pub fn rule_catalogue() -> Vec<RuleDoc> {
    // scanner 过程式 docs（rust.target 在前，保持现有顺序）
    // + PROJECT_DIR_RULES 派生 + PYCACHE doc
    // + PROVIDER_RULE_DOCS + GLOBAL_CACHE 派生 + docker 占位
}
```

- rules.rs `use crate::{providers, scanner}` 引入 doc 常量（crate 内模块互引合法；scanner/providers 依旧依赖 rules 的表与类型）。
- 现有 catalogue 顺序保持字节级不变（rust.target → 项目表 → __pycache__ → 4 provider → cargo → 全局表 → docker），保证 TUI 标签页内容不动。

### 3. 共享行格式化器（两个 adapter 一条真接缝）

rules.rs：

```rust
pub fn rule_row(doc: &RuleDoc) -> String {
    format!("{:<22} {:<9} {:<16} {}", doc.id, risk_label(doc.risk), doc.action, doc.summary)
}
pub fn risk_label(risk: &RiskLevel) -> &'static str  // 从 tui/render.rs 迁入或共享
```

- TUI `render_rules`：分组标题不变，行改用 `rule_row`（输出与现状一致——现行格式即 `{:<22} {:<9} {:<16} {}`；risk_label 迁移到 rules.rs 后 render.rs re-export/引用）。
- CLI `run_rules`：改为与 TUI 相同的分组布局（"Project rules" / "Global providers & caches" 小节 + `rule_row` 行），SCOPE 列由分组表达。**这是 PRD 已批准的展示层变更**；信息完整性不变。
- 注意 `risk_label` 目前在 tui/render.rs（Low/Medium/High/Dangerous → 文本）。迁至 rules.rs 作为 doc 展示的一部分，render.rs 处保留原有的样式函数（`risk_style`）不动；render.rs 其他调用点若引用 risk_label，改为 `crate::rules::risk_label`。

### 4. 防漂移测试改写

- 删除 `project_dir_rules_match_legacy_scanner_ids`（硬编码 id 清单的负向防漂移）。
- 新增正向聚合完整性断言：
  - `catalogue_covers_all_declared_docs`：catalogue 包含 SCANNER_RULE_DOCS、PROVIDER_RULE_DOCS、两张表的全部 id。
  - `catalogue_ids_are_unique` / `catalogue_has_no_empty_fields` 保留。
- scanner/providers 现有测试中硬编码的 rule_id 字符串断言保留（测试作为契约见证是合法的第二出现处；验收"生产代码只出现一次"不含测试）。

## 兼容性

- TUI Rules 标签页渲染输出不变（现行格式即共享格式）。
- CLI `rules` 输出改版（分组布局），信息无损——PRD 已批准。
- 扫描/执行行为零变化（id 值未变，仅来源收拢）。

## 权衡记录

- **doc 常量放实现文件而非集中在 rules.rs**：locality——改一条过程式规则（id/风险/摘要）只动一个文件；rules.rs 的模块注释相应更新为"表在此、过程式 doc 在实现处、catalogue 聚合"。
- **CLI 改分组布局而非给共享行加 scope 列**：flat 表 + scope 列会让 TUI 行多出冗余列；分组是两个 adapter 已验证的展示形态，收敛到它使行格式只有一处。
- **docker 占位留在 rules.rs**：无实现可同处；它是 catalogue 层的预告，不是走私。
