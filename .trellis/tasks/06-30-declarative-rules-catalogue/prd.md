# 扩展全局缓存目录 + 声明式规则表

父任务:`06-30-putzen-inspired-optimization`(R3)

## Goal

把 devsweep 当前**命令式硬编码**的清理规则抽成**数据驱动的规则表**(借鉴 putzen 的 `roots!` 宏 / `CacheType` 思路),在此基础上**扩展全局缓存覆盖面**,并让 `Rules` 标签页/命令真正列出规则——同时严守 devsweep "命令优先、否则 inspect/trash" 的安全分级。

## Background

- 项目规则现散落为命令式 Rust:
  - 项目级:`scan_rust_project`/`scan_node_project`/`scan_python_project_dir`([src/scanner.rs:108](../../../src/scanner.rs) 起),规则以内联数组/分支表达。
  - 全局级:仅 npm/pip/pnpm/yarn/cargo 5 个 `add_*_target` 函数([src/providers.rs:85](../../../src/providers.rs))。
- putzen 用 `roots!` 宏 + `///` 标签声明 ~50 个缓存目录,编译期校验([ref/repo/putzen-rs/src/caches/defaults.rs](../../../ref/repo/putzen-rs/src/caches/defaults.rs))。
- `Ecosystem::Docker` 已在枚举中([src/model.rs:65](../../../src/model.rs))但无任何 provider 产出;`Rules` 命令是占位([src/main.rs:98](../../../src/main.rs)),`Rules`/`JobsLogs` 标签页 `matches_target` 返回 false([src/tui.rs:1263](../../../src/tui.rs))。

## Requirements

- **声明式规则表**:为"标记文件 → 可清理目录"型规则(项目级)与"已知缓存目录/命令"型规则(全局级)建立数据结构,集中声明;扫描器消费该表而非散落分支。保留每条规则的 `ecosystem/kind/risk/selected_by_default/evidence/action` 语义。
- **覆盖面扩展**(在表中新增,保持安全分级):
  - 全局路径型:gradle(`~/.gradle/caches`)、maven(`~/.m2/repository`)、go(`go/pkg/mod`)、huggingface、JetBrains、sccache 等;无官方清理命令者用 `KnownCacheDir` + `MoveToTrash` 或 `NoopInspectOnly`(高风险者 inspect-only)。
  - 跨平台路径解析参考 putzen 的 `SEEDS`/`SEEDS_OS`(unix/windows 分支)。
- **Rules 标签页/命令**:`Rules` tab 与 `devsweep rules` 列出规则表(id/生态/风险/动作/说明),不再是占位文案。
- **Docker**:本任务**仅在规则表预留**(可留 TODO 或 inspect-only 占位),完整实现为可选延伸,非验收必需。

## Acceptance Criteria

- [ ] 存在集中式规则表;`scanner`/`providers` 改为消费它,行为对既有用例不回退。
- [ ] 新增至少 3 个此前未覆盖的全局缓存(如 gradle/maven/go),其 `action`/`risk` 符合安全分级(无官方命令者不盲删内部、用 trash 或 inspect)。
- [ ] `devsweep rules` 与 TUI `Rules` 标签页能列出规则表内容(含新条目)。
- [ ] 规则表有测试:条目唯一性、字段非空、新生态被发现(用 FakeProbe/tempfile 风格)。
- [ ] `cargo test` 全绿;若 `CleanupPlan` 形状变化则升 `CLEANUP_PLAN_VERSION` 并说明。

## Constraints / Risks

- **安全优先于覆盖面**:新增目录若无官方清理命令且删除有风险,优先 `NoopInspectOnly`;`~/.cargo` 已是 inspect-only 的先例([src/providers.rs:237](../../../src/providers.rs))。
- 声明式表不必照搬 putzen 的 `roots!` 过程宏;Rust 的 `const`/`static` 数组 + 结构体即可,优先可读性与可测性(CLAUDE.md:不过度抽象)。
- 工作量最大,建议放在 perf 与 ranking 之后;但验收独立。
- 与 `06-30-size-age-ranking` 在 `selected_by_default`/排序上可能交叉,实现时以各自 design 协调,避免互相覆盖。
