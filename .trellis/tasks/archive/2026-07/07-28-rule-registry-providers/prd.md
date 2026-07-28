# RuleRegistry 统一与 provider 三平台精度

> 父任务：07-28-audit-remediation ｜ 审计条目：F-16（残留）、F-14、F-17、F-13（残留）、F-28 ｜ 对应审计 Milestone M3 / PR12

## Goal

把 07-05 已完成的"规则身份单一声明"推进为完整 RuleRegistry（detector / resolver / action factory / safety contract / selection policy 单一事实源），修正过粗的全局规则粒度，补齐三平台路径解析与 env 重定向，并让 provider 探测输出经过验证。

## 背景与证据（当前 HEAD）

1. **身份已收拢但仍有漂移（F-16 残留）**：`PNPM_STORE_RULE_DOC.risk = Medium`（`src/providers.rs:36-43`）而实际 target 构造用 `RiskLevel::Low`（`providers.rs:235`）——catalogue 与 plan 各说各话；yarn catalogue 显示基础 id `yarn.cache.clean`，实际 target id 为 `.classic`/`.modern` 后缀（`providers.rs:45-53,271`）。深层原因：RuleDoc 只统一了"文档"，risk/action/selection 仍散在 procedural 代码里。
2. **全局规则过粗（F-14）**：`maven.repository` 整仓 Trash（Maven local repo 可含 `mvn install` 写入的 local-only artifact，"可重下"不成立）；Windows `jetbrains.caches` 以整个 `AppData/Local/JetBrains` vendor root 为删除单元（JetBrains system 目录含 local history 等非缓存数据）；`go.mod_cache` 直接 Trash `~/go/pkg/mod` 而非官方 `go clean -modcache`（`src/rules.rs:197-298`）。npm 全量 `cache clean --force` 作为常规动作也应重新定位（官方文档称 cache self-healing）。
3. **平台解析只有 windows/非-windows（F-17）**：`GLOBAL_CACHE_RULES_OS` 二分（`rules.rs:255-298`），macOS 得到 Linux-shaped `.cache/JetBrains`；忽略 `GRADLE_USER_HOME`、Maven settings.xml、`GOMODCACHE`、`NUGET_PACKAGES` 等重定向。
4. **provider 上下文与输出（F-13 残留 / F-28）**：`home_dir()` 混合 USERPROFILE/HOMEDRIVE+HOMEPATH/HOME（`providers.rs:119-132`）；pip 只取第一个可解析 launcher、失败不尝试下一个（`providers.rs:184-195`）；`output_path` 取 stdout 第一条非空行、仅过滤 "undefined"/"null"，不要求 absolute/existing/dir（`providers.rs:467-477`）——警告文本或错误消息可能被当作路径。
5. **规则契约缺失（审计 §6.2）**：每条规则缺少 allowed roots、explicit non-targets、data class、active probe、live fingerprint、estimate semantics、fixture matrix 等声明。

## Requirements

1. 在 07-28-plan-validation 定型的 **registry contract/core 之上扩展，不迁移不重建**（二轮评审 ARCH-002）：为每条规则补 detector / resolver / platforms / safety_contract / selection_policy / docs 字段，action 重建继续走同一 contract；scanner、providers、`devsweep rules`、TUI Rules tab、plan validator、executor 全部消费同一 registry。
2. 修正漂移：pnpm risk 单一来源；yarn 变体 id 在 catalogue 中如实列出（或 doc 明示变体展开规则并有测试锁定）。
3. 规则粒度修正：`maven.repository` → High/InspectOnly（或按 resolver provenance 分级）；JetBrains → 仅定位 product/version-specific 的 cache/log/tmp 子树，永不以 vendor root 为删除单元；`go.mod_cache` → 走 `go env GOMODCACHE` 发现 + `go clean -modcache` 官方动作；npm 全量 clean 保持非默认并考虑 verify 优先。
4. 三平台 resolver：macOS 独立路径集；发现顺序 env → 官方命令/配置 → 平台默认，provenance 记入 evidence；`GRADLE_USER_HOME`/`NUGET_PACKAGES`/`GOMODCACHE`/Maven settings 生效。
5. provider 探测：OS-specific home resolver；pip 多 launcher fallback（`py`→`python`→`python3` 逐个尝试直到 `pip cache dir` 成功）；typed probe 结果（NotFound/Exit/Timeout/InvalidOutput，消费 process-runner 任务的 runner）。
6. 输出验证：provider stdout 解析出的路径必须 absolute + 存在 + 目录，否则 target 标记 unresolved 并降级（unknown size 与真实 0 区分，衔接 scan-reliability 的 SizeEstimate）。

## Acceptance Criteria

- [x] 测试：catalogue 与实际 target 的 risk/action/selection 由同一数据推导，注入不一致即编译失败或测试失败（不可能再漂移）
- [x] fixture 矩阵（每 provider）：默认路径、env 重定向、配置重定向、缺工具、多 launcher、无效 stdout（警告文本/相对路径/不存在路径）——行为全部符合契约
- [x] macOS 路径集与 Linux 分离，三平台各有 resolver 单元测试
- [x] maven/JetBrains/go 规则按新粒度出 plan：整仓/vendor-root Trash 的旧行为在测试中被明确拒绝
- [x] pip 在第一个 launcher 无 pip 模块时成功回退到下一个
- [x] `devsweep rules`、TUI Rules tab、plan 中的 rule id/risk 三处一致（回归测试锁定）

## 约束与依赖

- 依赖：07-28-plan-validation（registry contract owner——本任务是扩展方而非重建方）、07-28-central-safety-policy（safety_contract 挂接 authorize）、07-28-process-runner-cancellation（typed probe 依赖 runner core）。数据修正（风险漂移、规则粒度）可在 contract 落地后立即开始。
- 不在本任务新增生态（Wave 1–3 provider 扩展另行立项）；只修正既有 13 条项目规则 + 5 个 provider + 8 条全局规则。
- 规则行为变化（maven/JetBrains/go）属用户可见变更，需在 README 与 rule summary 中说明。
