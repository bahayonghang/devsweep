# Implement — 声明式规则表 + 全局缓存扩展 + Rules 展示

> 契约与依据见 `design.md`。每步给出 verify;分组间有 review/rollback 点。
> 全程校验命令:`just ci`(= `cargo fmt --all -- --check` + `cargo check --all-targets` + `cargo test --all-targets` + `cargo clippy --all-targets -- -D warnings`)。
> 快速回环用 `cargo test`;最终必须 `just ci` 全绿。

## 阶段 A — 新建规则模块(纯数据,无消费方)

1. 新建 `src/rules.rs`,定义 §2 全部类型:`ProjectMarker`、`ProjectDirRule`、`KnownCacheAction`、`GlobalCacheRule`、`RuleScope`、`RuleDoc`。
   - verify:`cargo check` 通过(此时模块尚未注册,仅语法自检可暂缓到步 4)。
2. 写 `PROJECT_DIR_RULES`(§2.1,10 条,逐条对应现 scanner 内联规则,含 `label`)。
3. 写 `GLOBAL_CACHE_RULES` + `#[cfg(windows)]/#[cfg(not(windows))]` 的 `GLOBAL_CACHE_RULES_OS`(§2.2),及 `project_dir_rules()`、`global_cache_rules()` 迭代辅助。
4. `src/lib.rs` 注册 `pub mod rules;`。
   - verify:`cargo check --all-targets` 通过。
5. 写 `rule_catalogue()`(§2.3,聚合四类,Project 先 Global 后)。
   - verify:`cargo check` 通过。
6. 加 `rules.rs` 单元测试:`catalogue_ids_unique`、`catalogue_no_empty_fields`、`project_dir_rules_match_legacy_ids`、`global_cache_rules_ids_unique`(含 gradle/maven/go 断言)。
   - verify:`cargo test rules::` 全绿。

**Review/Rollback 点 A**:此时无任何消费方改动、对外行为零变化。规则表 + 目录册独立可测通过后再进阶段 B。

## 阶段 B — 扫描器消费表(A 类,行为等价)

7. `src/scanner.rs`:`scan_node_project` 内联 `rules` 数组 → 迭代 `rules::project_dir_rules(ProjectMarker::Node)`(§3);`ecosystem/kind/risk` 传 `rule.field.clone()`。
8. 同法改 `scan_python_project_dir` → `ProjectMarker::Python`。删除两处内联数组。
   - 保持 `marker` 查找、`python_context` 传播、`__pycache__` descent、`scan_rust_project`、`dedupe_targets`、`should_stop_descent` 不变。
   - verify:`cargo test --lib scanner::` 全绿——尤其 `scanner_finds_marker_backed_project_targets`(11 条规则全命中)、`scan_roots_returns_ranked_targets_after_dedupe` 不回退。

**Review/Rollback 点 B**:project 扫描行为必须与改造前逐条等价。若 `scanner::` 任一用例失败,回退步 7–8,复核 id/relative/risk/selected 是否与旧数组一一对应。

## 阶段 C — 新增全局缓存 provider(C 类,新覆盖面)

9. `src/providers.rs`:新增 `add_known_cache_targets`(§4),`use crate::rules::{global_cache_rules, KnownCacheAction};`(按现有 import 风格并入)。
10. 在 `scan_with_probe` 的 `add_cargo_home_target` 后追加 `add_known_cache_targets(probe, &mut targets);`。
11. 加 provider 测试:`known_cache_targets_emit_trash_for_present_dirs`、`known_cache_inspect_rule_is_noop`(遍历规则找 InspectOnly,平台无关)、`missing_home_dir_emits_no_known_cache_targets`。
    - verify:`cargo test --lib providers::` 全绿——现有 5 个用例(尤其 `command_providers_emit_official_actions...` 的「无 MoveToTrash」断言、`missing_command_providers_are_non_fatal`)必须不受影响(`FakeProbe` 默认 `home=None`)。

**Review/Rollback 点 C**:确认 `scan --json --global` 手测能看到 gradle/maven 等新条目(若本机存在对应目录),且 npm/pip 等旧行为不变。

## 阶段 D — 展示层(Rules tab / CLI)

12. `src/main.rs::run_rules`:替换占位,遍历 `devsweep::rules::rule_catalogue()` 表格化打印(§5.1);加表头行。
    - verify:`cargo run -- rules` 输出含 `rust.target`、`gradle.caches` 等,无占位文案。
13. `src/tui.rs::render_rules`:替换硬编码 `lines`,遍历 `rule_catalogue()` 按 scope 分组渲染(§5.2);签名不变。
14. 加 `render_rules` 的 TestBackend 用例(镜像现有 tui 测试),断言缓冲区含 `gradle` 与 `rust.target`。
    - verify:`cargo test --lib tui::` 全绿。

**Review/Rollback 点 D**:`cargo run -- tui` 进入 Rules tab(按键 `4`)目视规则列表正常。

## 阶段 E — 全量校验与收尾

15. `just ci` 全绿(fmt / check / test / clippy -D warnings)。
    - 若 clippy 报 `needless_return`/`clone_on_copy` 等,按提示修正(注意 `Ecosystem` 非 Copy,`.clone()` 合理,勿被误导删除)。
16. 自检对照 `prd.md` Acceptance Criteria 逐条打勾:
    - [ ] 集中式规则表存在,scanner/providers 消费它,既有用例不回退。
    - [ ] ≥3 个新全局缓存(gradle/maven/go...),action/risk 合规(trash 或 inspect,无官方命令不盲删内部)。
    - [ ] `devsweep rules` 与 TUI Rules tab 列出规则表(含新条目)。
    - [ ] 规则表测试:唯一性、字段非空、新生态被发现。
    - [ ] `cargo test` 全绿;`CleanupPlan` 形状未变,**无需**升 `CLEANUP_PLAN_VERSION`(在收尾说明中记一句)。

## 备注(明确排除,非本次范围)

- Docker 仅目录册 planned 占位,不实现扫描。
- 不新增 `Ecosystem` 变体(gradle/maven/go 归 `Generic`);专用生态为可选后续。
- 不做 GOPATH/GOMODCACHE/env 覆盖、sccache/更多 OS 缓存;留作后续易扩点(表内加行即可)。
