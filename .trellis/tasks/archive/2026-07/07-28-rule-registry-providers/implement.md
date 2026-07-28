# RuleRegistry 统一与 provider 三平台精度：执行计划

## 启动前置与 No-Go

- [ ] 保持本任务为 `planning`，不运行 `task.py start`。只有在最新 planning review 获批且父任务 gate 全部满足后才可进入实现。
- [ ] 取得 `07-28-plan-validation` 的 registry core/`ActionSpec`、`07-28-central-safety-policy` 的 `SafetyPolicy::authorize()` 接口、`07-28-process-runner-cancellation` 的 typed `ProcessRunner` 接口。接口未落地时，记录书面接口协商；不得创建平行实现。
- [ ] 与 `07-28-scan-reliability` 协商 `SizeEstimate`/unresolved 兼容表示；这不是 #11 的父任务排序依赖。接口未到位时，路径解析失败的相关 target 保持 unresolved/InspectOnly，不得写入伪造 `0 B` 或猜测替代字段。
- [ ] 在实现前补齐官方文档/配置格式证据，尤其是 Maven settings、JetBrains 三平台目录、Go `GOMODCACHE`、Gradle 与 NuGet 重定向。任何无法证明的 destructive footprint 保持 unresolved 或 InspectOnly。

## 1. 先写 fail-red 回归测试

- [ ] 在 `src/rules.rs` 与 `src/providers.rs` 建立 registry projection 测试：catalogue、生成 target、action、risk、selection 共享一个 descriptor；故意制造 PNPM risk 不一致时必须失败。
- [ ] 锁定 Yarn classic/modern id 的 catalogue 表示或文档化展开规则，避免基础 id 与实际 target id 漂移。
- [ ] 为 Maven default High/InspectOnly 写回归测试，并证明普通 `settings.xml` 路径不足以降级；只有显式 `ExternalCache` provenance fixture 才能走例外分支。
- [ ] 为 JetBrains 写 vendor-root rejection fixture，且仅接受 product/version-specific `caches`、`log`、`tmp` 子树。
- [ ] 为 Go 写 resolver/action fixture：`go env GOMODCACHE` 用于解析，plan 使用 `go clean -modcache`，从不直接 Trash `~/go/pkg/mod`。
- [ ] 为 provider 输出写绝对路径、存在目录、相对路径、警告文本、空输出、超时与非零退出矩阵；无效输出必须 unresolved + typed diagnostic + unknown/incomplete size。
- [ ] 为 pip 写多 launcher fallback：第一个 launcher 的 pip probe 失败后继续下一个成功 launcher。

## 2. 在上游 core 上实现 registry projection

- [ ] 在 `src/rules.rs` 将既有 table/procedural docs 迁移为对上游 core 的扩展 descriptor；不重命名或复制其 identity/action core。
- [ ] 让 `src/scanner.rs`、`src/providers.rs` 从 descriptor 得到 rule id、risk、action、selection、evidence 及 safety contract，而不是在各自分支手写这些字段。
- [ ] 让 `src/main.rs` 的 `devsweep rules` 与 `src/tui/render.rs` Rules tab 从同一 descriptor 投影行；为 CLI/TUI/plan 三处一致性添加回归测试。
- [ ] 将 cross-layer serializable 字段的任何需要提交给 plan-validation owner；本任务不直接更改 `CLEANUP_PLAN_VERSION`、serde tag 或 canonical serialization。

## 3. 实现 resolver 与 typed probe 边界

- [ ] 用 ProcessRunner 替换 provider 中裸 stdout `Option<String>` probe，保留 program/argv 分离、timeout 与输出上限。
- [ ] 实现 OS-specific home resolver，以及 env -> 官方命令/配置 -> 平台默认的 resolver 链；把 provenance 写入 evidence。
- [ ] 对 npm、pip、pnpm、Yarn 输出应用严格路径验证；无效时不构造可执行 footprint。
- [ ] 将 npm `cache verify` 接入同一 target 的 verify-first preflight，`cache clean --force` 保持非默认；不得为同一路径生成 verify/clean 两个用户可见 target。
- [ ] 将 pip 多 launcher fallback、Yarn variant 选择、Cargo home inspect-only 规则纳入同一 typed probe 结果模型。

## 4. 修正规则粒度和安全策略挂接

- [ ] Maven 默认置为 High/InspectOnly；只有满足 design.md 所列 external-cache provenance 与 safety-contract 证明的独立变体才能获得可清理 action。
- [ ] 将 JetBrains resolver 收窄为产品/version cache/log/tmp 子树，并把 vendor root 写入 explicit non-targets。
- [ ] 用 `go env GOMODCACHE` + `go clean -modcache` 替换 Go 的 home-relative Trash；扫描阶段仅生成 action data。
- [ ] 实现 Gradle、Maven、NuGet 与 Go 的重定向解析，且让 `SafetyPolicy::authorize()` 在实际执行前消费 safety contract。
- [ ] 保留 Cargo home inspect-only、Cargo credentials/bin/registry internals 的保护与现有生态范围。

## 5. 文档、适配与质量检查

- [ ] 更新 `README.md` 与 rule summary，说明 Maven inspect-only 默认、JetBrains 精确范围、Go 官方 action 和 npm verify-first/non-default 行为。
- [ ] 更新 `src/model.rs`、`src/sweep.rs`、`src/tui/app.rs`、`src/tui/test_support.rs` 中因上游 v2 unresolved/completeness contract 引起的编译适配；不在本任务绕过 upstream owner。
- [ ] 运行定向 Rust 测试后运行 `cargo fmt --all -- --check`、`cargo check --all-targets`、`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`，最后运行 `just ci`。
- [ ] 手动检查 `cargo run -- rules` 的 CLI 输出，以及 TUI Rules tab 的 rule id/risk/action 显示；`scan --global --json` 只允许 read-only probe，绝不执行 cleanup。

## 平台验证矩阵与交付门槛

| 平台 | 必须证据 | No-Go 条件 |
|---|---|---|
| Windows | `USERPROFILE`/重定向 env、JetBrains 子树、NuGet/Go resolver fixture；Windows CI 结果 | 无法在 Windows 或 Windows CI 验证时，不宣称 Windows resolver 已通过 |
| Linux | HOME、配置/命令 fallback、pip launcher fixture；Linux CI 结果 | 只有 mock 而无 Linux 执行证据时，不完成该平台验收 |
| macOS | 独立路径集与 resolver unit test；macOS CI 结果 | 不得把 Linux `.cache` 路径作为 macOS pass 证据 |

- [ ] 记录三平台 CI 的提交 SHA、run URL 与 required-check 结果；本地 Windows 验证不能替代其他平台证据。
- [ ] 若某 resolver/path/provenance 证据不充分，保持 unresolved/InspectOnly 并在 task 记录中标明，不以默认路径猜测替代。
- [ ] 若 quality gate 或 contract review 失败，回到对应步骤修正；若需要撤回，限定在本子任务的独立代码提交，不回滚其他任务或放宽安全策略。
