# Design — 声明式规则表 + 全局缓存扩展 + Rules 展示

## 0. 结论先行

devsweep 现有规则有**四种形态**,不能强行统一成一张表(会为单点强造抽象,违反 CLAUDE.md「不过度抽象」):

| 形态 | 现状位置 | 是否纯数据 | 处置 |
|------|----------|-----------|------|
| A. 标记文件 → 相对缓存目录(trash) | `scan_node_project` / `scan_python_project_dir` 内联数组 | ✅ 是 | **抽成 `PROJECT_DIR_RULES` 表**,扫描器消费 |
| B. 标记文件 → 官方命令 | `scan_rust_project`(`cargo clean`) | ⚠️ 单点特例 | 保留函数,仅登记进目录册 |
| C. 已知家目录缓存(无官方命令) | 目前**不存在** | ✅ 是 | **新建 `GLOBAL_CACHE_RULES` 表** + 新 provider |
| D. 探测命令 → 解析路径(官方命令) | `add_npm/pip/pnpm/yarn_target` | ❌ 过程式(需运行工具、解析 stdout、yarn 还按版本分支) | 保留原样,仅登记进目录册 |

**设计核心**:把 **A、C** 两类纯数据规则集中进新模块 `src/rules.rs`;**B、D** 的过程式逻辑保持不变,但同样在 `rules.rs` 里以静态描述项登记,使 `rule_catalogue()` 覆盖全部规则,供 `Rules` tab / `devsweep rules` 展示。

这样既满足 PRD「扫描器消费规则表而非散落分支」(针对 A/C),又不为 D 的过程式 provider 强造数据编码。

## 1. 模块边界

- 新增 `src/rules.rs`;`src/lib.rs` 注册 `pub mod rules;`(紧随 `pub mod ranking;`,保持字母序附近)。
- `rules.rs` 只含**数据 + 纯函数**(表、目录册构建),不做任何 IO、不依赖 `ProviderProbe`。消费方各自做 IO:
  - `scanner.rs` 迭代 `PROJECT_DIR_RULES` 产出 project 目标。
  - `providers.rs` 新增 `add_known_cache_targets` 迭代 `global_cache_rules()` 产出 global 目标。
  - `main.rs::run_rules` / `tui.rs::render_rules` 消费 `rule_catalogue()`。
- **不改** `model.rs`(不新增字段/枚举变体)、`ranking.rs`、`executor.rs`。

## 2. 数据结构(`src/rules.rs`)

```rust
use crate::model::{Ecosystem, RiskLevel, TargetKind};

/// A(项目级)—— 标记文件在场 → 相对缓存目录可 trash。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectMarker { Node, Python }

#[derive(Debug, Clone)]
pub struct ProjectDirRule {
    pub id: &'static str,
    pub label: &'static str,        // 目录册展示用
    pub ecosystem: Ecosystem,
    pub marker: ProjectMarker,
    pub relative: &'static str,     // 项目目录下相对路径,'/' 分隔
    pub kind: TargetKind,
    pub risk: RiskLevel,
    pub selected_by_default: bool,
}

/// C(全局级)—— 已知家目录缓存,无官方命令。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownCacheAction { Trash, InspectOnly }

#[derive(Debug, Clone)]
pub struct GlobalCacheRule {
    pub id: &'static str,
    pub label: &'static str,
    pub ecosystem: Ecosystem,
    pub relative: &'static str,     // 家目录相对路径,'/' 分隔
    pub kind: TargetKind,
    pub risk: RiskLevel,
    pub action: KnownCacheAction,
}

/// 目录册条目(供 Rules tab / CLI 展示,聚合全部四类)。
#[derive(Debug, Clone)]
pub struct RuleDoc {
    pub id: &'static str,
    pub ecosystem: Ecosystem,
    pub scope: RuleScope,           // Project | Global
    pub risk: RiskLevel,
    pub action: &'static str,       // "cargo clean" / "trash" / "official command" / "inspect only" / "deferred"
    pub summary: &'static str,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleScope { Project, Global }
```

> `Ecosystem`/`TargetKind`/`RiskLevel` 均为单元变体枚举,可在 `const` 上下文构造;表用 `const` 数组即可。结构体派生 `Clone`(非 `Copy`,因含非 Copy 的 `Ecosystem`),消费时 `.clone()` 字段。

### 2.1 `PROJECT_DIR_RULES`(A 类,10 条 = 现有内联规则原样搬迁)

id/relative/kind/risk/selected 逐条对应现 `scan_node_project`(4 条)与 `scan_python_project_dir`(6 条),**零语义变更**:
`node.node_modules`(DependencyDirectory/Medium/false)、`node.next_cache`(`.next/cache`/BuildArtifacts/Low/true)、`node.turbo`(Low/true)、`node.parcel_cache`(Low/true);`python.venv_dot`(`.venv`/VirtualEnv/Medium/false)、`python.venv`(Medium/false)、`python.pytest_cache`(Low/true)、`python.mypy_cache`(Low/true)、`python.ruff_cache`(Low/true)、`python.tox`(Medium/false)。

辅助:`pub fn project_dir_rules(marker: ProjectMarker) -> impl Iterator<Item = &'static ProjectDirRule>`(按 marker 过滤)。

### 2.2 `GLOBAL_CACHE_RULES` + OS 分支(C 类,新增覆盖面)

跨平台家目录相对(unix / Windows 同为 `~` 下同名):

| id | relative | kind | risk | action |
|----|----------|------|------|--------|
| `gradle.caches` | `.gradle/caches` | PackageCache | Medium | Trash |
| `gradle.wrapper_dists` | `.gradle/wrapper/dists` | ToolCache | Medium | Trash |
| `maven.repository` | `.m2/repository` | PackageCache | Medium | Trash |
| `go.mod_cache` | `go/pkg/mod` | PackageCache | Medium | Trash |
| `ivy.cache` | `.ivy2/cache` | PackageCache | Medium | Trash |
| `nuget.packages` | `.nuget/packages` | PackageCache | Medium | Trash |

OS 特定(`#[cfg]` 分支,id 相同但按平台各编译一份):

| id | Windows relative | 非 Windows relative | risk | action |
|----|------------------|---------------------|------|--------|
| `jetbrains.caches` | `AppData/Local/JetBrains` | `.cache/JetBrains` | Medium | Trash |
| `huggingface.hub` | `AppData/Local/huggingface` | `.cache/huggingface` | **High** | **InspectOnly** |

`pub fn global_cache_rules() -> impl Iterator<Item = &'static GlobalCacheRule>` = `GLOBAL_CACHE_RULES.iter().chain(GLOBAL_CACHE_RULES_OS.iter())`。

**风险分级依据**(严守 PRD「安全优先于覆盖面」):
- gradle/maven/go/ivy/nuget:纯可重下载的包缓存 → Trash + Medium。删了只是重新下载,`trash` 可逆。与现有 npm/pip 缓存(Medium)同级。
- huggingface:模型体积巨大、离线重下代价高 → **InspectOnly**(不自动建议 trash 几十 GB)。沿用 `~/.cargo` inspect-only 先例。
- 所有全局缓存 `selected_by_default = false`(与现有 npm/pip/pnpm/yarn 全局 provider 一致;避免默认勾选大目录)。

**生态归类**:gradle/maven/go 等映射到 `Ecosystem::Generic`。**不新增枚举变体**——新增 `Java/Go/Ml` 会改动 `model.rs` 枚举、`tui.rs::render_categories` 计数、并影响 JSON,属范围蔓延。人类可读名放 `label`。专用生态列为**可选后续**,非本任务验收项。

### 2.3 `rule_catalogue()`(聚合四类)

```rust
pub fn rule_catalogue() -> Vec<RuleDoc>
```
拼装顺序(Project 先、Global 后):
1. 静态描述:`rust.target`(Project/Rust/Low/"cargo clean")。
2. 由 `PROJECT_DIR_RULES` 派生(action 恒为 "trash",summary 用 `label`)。
3. 静态描述:`python.__pycache__`(Project/Python/Low/"trash";descent 期发现,不在 A 表内)。
4. 静态描述:命令型 provider `npm.cache.clean`/`pip.cache.purge`/`pnpm.store.prune`/`yarn.cache.clean`(Global/"official command")+ `cargo.home.inspect`(Global/Rust/High/"inspect only")。
5. 由 `global_cache_rules()` 派生(action = Trash→"trash" / InspectOnly→"inspect only")。
6. 静态描述:`docker`(Global/Docker/"deferred","Docker prune —— 已预留,暂不扫描")。保留当前占位里的 Docker 提示,诚实标注为 planned。

> 目录册对 B/D 用**静态描述项**而非从扫描表派生——因为 B/D 的可扫描逻辑本就是过程式,其**文档**是静态的。这是有意的边界,不是遗漏。

## 3. 扫描器改造(`src/scanner.rs`)

`scan_node_project` / `scan_python_project_dir` 的内联 `let rules = [ ... ];` 替换为迭代 `rules::project_dir_rules(ProjectMarker::Node|Python)`:

```rust
for rule in crate::rules::project_dir_rules(ProjectMarker::Node) {
    let path = dir.join(rule.relative);
    if is_real_dir(&path) {
        targets.push(build_path_target(PathTargetInput {
            rule_id: rule.id,
            ecosystem: rule.ecosystem.clone(),
            kind: rule.kind.clone(),
            project_root: dir.to_path_buf(),
            path,
            risk: rule.risk.clone(),
            selected_by_default: rule.selected_by_default,
            evidence: vec![
                Evidence::MarkerFile { path: marker.clone() },
                Evidence::RuleMatched { rule_id: rule.id.to_string() },
            ],
        }));
    }
}
```

- `PathTargetInput.rule_id` 现为 `&'static str` → 直接传 `rule.id`,无需改签名。
- `dir.join(".next/cache")` 与原 `PathBuf::from(".next").join("cache")` 指向同一目录;分隔符差异仅影响 `id` 展示字符串,现有测试用 `id.starts_with(rule_id)` 断言,不受影响。
- `python_context` 传播、`__pycache__` descent、`scan_rust_project`、`should_stop_descent`、`dedupe_targets` **均不变**。

## 4. Provider 改造(`src/providers.rs`)

新增 `add_known_cache_targets(probe, &mut targets)`,在 `scan_with_probe` 中于 `add_cargo_home_target` 之后调用:

```rust
fn add_known_cache_targets(probe: &impl ProviderProbe, targets: &mut Vec<CleanTarget>) {
    let Some(home) = probe.home_dir() else { return; };
    for rule in crate::rules::global_cache_rules() {
        let path = home.join(rule.relative);
        if !probe.is_dir(&path) { continue; }
        let (estimated_bytes, last_modified) = probe.estimate_path_size(&path);
        let action = match rule.action {
            KnownCacheAction::Trash => CleanAction::MoveToTrash { path: path.clone() },
            KnownCacheAction::InspectOnly => CleanAction::NoopInspectOnly,
        };
        targets.push(CleanTarget {
            id: TargetId::new(format!("{}:{}", rule.id, path.display())),
            scope: Scope::Global,
            ecosystem: rule.ecosystem.clone(),
            kind: rule.kind.clone(),
            path: Some(path.clone()),
            estimated_bytes,
            last_modified,
            risk: rule.risk.clone(),
            reversible: true,               // trash 可逆;inspect 为 no-op,与 cargo-home 先例一致
            selected_by_default: false,
            evidence: vec![
                Evidence::KnownCacheDir { source: rule.label.to_string(), path },
                Evidence::RuleMatched { rule_id: rule.id.to_string() },
            ],
            action,
        });
    }
}
```

- 复用现有 `ProviderProbe`(`home_dir`/`is_dir`/`estimate_path_size` 已具备),无需扩 trait。
- **不碰 cargo**:`~/.cargo` 仍由 `add_cargo_home_target` 整体 inspect-only;不新增 `.cargo/*` 子目录 trash 规则,既守安全先例,又避免与 cargo-home 目标嵌套。

## 5. 展示层

### 5.1 CLI(`src/main.rs::run_rules`)

替换占位打印为遍历目录册,表格化输出:
```
ID                           SCOPE    RISK    ACTION           SUMMARY
rust.target                  project  low     cargo clean      Rust build dir ...
gradle.caches                global   medium  trash            gradle caches ...
...
```
`scope`/`risk` 用小写字符串(内联 `match` 或小 helper)。函数体外新增 `use devsweep::rules;`。

### 5.2 TUI(`src/tui.rs::render_rules`)

`render_rules(frame, area)` 签名不变(无需 app 状态,保持渲染纯函数、TestBackend 友好)。内容改为遍历 `rule_catalogue()`,按 scope 分组,每行 `id — risk — action — summary`,仍用 `panel_block("Rules")` + `Paragraph` + `Wrap`。`matches_target` 对 `Rules` 仍返回 false(Rules tab 不复用目标列表,独立渲染目录册)。

## 6. 与 `06-30-size-age-ranking` 的协调

- 新全局目标 `selected_by_default = false` → `ranking::apply_freshness_guard` 对其为 no-op(guard 只把 true→false)。
- 新目标随 `rank_cleanup_plan`(已在 `scan_with_probe` 末尾调用)按 size 降序参与排序,无需改 `ranking.rs`。
- **不把任何全局缓存设为 `selected_by_default = true`**,避免与 freshness guard 交叉、避免默认勾选大目录——即 PRD 所述「避免互相覆盖」。

## 7. 兼容性

- **不改 `CleanTarget` 形状、不加枚举变体** → `CLEANUP_PLAN_VERSION` 保持 `1`,无需升版。
- 新目标仅用既有变体:`CleanAction::{MoveToTrash, NoopInspectOnly}`、`Evidence::{KnownCacheDir, RuleMatched}`。
- 既有测试全绿的关键:`FakeProbe` 默认 `home=None` → `add_known_cache_targets` 提前返回,`command_providers_emit_official_actions_when_tools_are_available` 里「无 MoveToTrash」断言不受影响。

## 8. 测试设计

`src/rules.rs`(`#[cfg(test)]`):
- `catalogue_ids_unique`:`rule_catalogue()` 全部 id 唯一。
- `catalogue_no_empty_fields`:每条 id/action/summary 非空。
- `project_dir_rules_match_legacy_ids`:表中 id 集合 == 现扫描器硬编码的 node/python id 集合(回归防护)。
- `global_cache_rules_ids_unique` 且含 `gradle.caches`/`maven.repository`/`go.mod_cache`(新覆盖面断言)。

`src/providers.rs`:
- `known_cache_targets_emit_trash_for_present_dirs`:`FakeProbe` 设 `home=/home`,注册某 Trash 规则目录 + size → 目标 id 前缀匹配、`action==MoveToTrash`、evidence 含 `KnownCacheDir`+`RuleMatched`、risk==Medium、`!selected_by_default`。
- `known_cache_inspect_rule_is_noop`:在测试内遍历 `global_cache_rules()` 找到 `InspectOnly` 规则,注册其目录 → `action==NoopInspectOnly`(平台无关,不硬编码 huggingface 路径)。
- `missing_home_dir_emits_no_known_cache_targets`:`home=None` → 无该类目标。
- 现有 5 个 provider 测试**不改**,必须继续通过。

`src/scanner.rs`:现有 `scanner_finds_marker_backed_project_targets` 覆盖 node/python 规则改由表驱动后仍全部命中(回归防护),不改。

`src/tui.rs`:新增 `render_rules` TestBackend 用例(镜像现有 TUI 测试风格),断言缓冲区含新条目文本(如 `gradle`)与项目规则(如 `rust.target`),验证 Rules tab 列出规则表。

`src/main.rs`:CLI stdout 不易直接单测(沿用 size-age design 结论);目录册逻辑由 `rules.rs` 测试覆盖,`run_rules` 仅打印。

## 9. 风险与回滚

- **范围克制**:Docker 仅目录册占位(planned);专用生态枚举、GOPATH/env 覆盖、sccache 等为可选后续,不进本次 MVP。
- **路径分隔符**:`relative` 用 `/`,`Path::join` 在 Windows 亦正确解析;仅 `id` 展示串有 `/` vs `\` 差异,无功能影响、无测试依赖。
- **回滚**:改动集中在新 `rules.rs` + 4 处消费点;若需回退,删模块注册并还原 4 处调用即可,`model`/`ranking`/`executor` 未触碰。
- **验收门槛**:`just ci`(fmt + check + test + clippy)全绿。
