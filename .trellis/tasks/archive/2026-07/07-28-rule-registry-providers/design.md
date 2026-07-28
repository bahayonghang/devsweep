# RuleRegistry 统一与 provider 三平台精度：技术设计

## 目标与边界

本子任务在 `07-28-plan-validation` 已定义的 registry contract/core 上扩展既有 13 条项目规则、5 个 provider 与 8 条全局规则。目标是让规则的发现、路径解析、动作重建、安全约束、默认选择与展示资料从同一个规则声明推导，消除 catalogue、plan、CLI 与 TUI 之间的漂移。

本任务不新增生态、不重建或迁移上游 registry core、不自行改变 `CleanupPlan` serde/version，也不在 scanner/provider 层执行清理动作。所有新增探测只可使用官方只读/验证命令；实际 cleanup 仍由 executor 在经过验证的 plan 上执行。

## 上游契约与所有权

| 上游 owner | 本任务消费的契约 | 本任务不得做的事 |
|---|---|---|
| `07-28-plan-validation` | `RuleId -> ActionSpec` registry core、provider action 重建、plan schema v2 与 canonical serialization | 新建平行 registry、修改 plan version 或重定义 action identity |
| `07-28-central-safety-policy` | `SafetyPolicy::authorize()` 与保护语义分类 | 在 resolver/provider 中绕过或复制授权判断 |
| `07-28-process-runner-cancellation` | `ProcessRunner` port、timeout/output-limit 与 `NotFound`/`Exit`/`Timeout`/`InvalidOutput` 诊断 | 在 provider 中直接用无边界的进程调用替代 runner |
| `07-28-scan-reliability`（兼容交接，非父任务排序依赖） | unknown/incomplete `SizeEstimate` 表示 | 将解析或 size 失败伪装为可信的 `0 B` |

前三个 owner 是父任务为 #11 指定的启动依赖；`scan-reliability` 仅提供需要协商的显示/size 兼容契约，不改变 #11 的排序。任一公共接口尚未落地时，本任务只可完成接口协商和 fail-red 测试准备；不得猜测 API 形状后开始实现。任务启动仍须满足父任务的 planning gate。

## Registry 扩展模型

具体 Rust 类型名称跟随上游 core；本设计定义语义而不预占其公共 API。每个注册规则在同一声明处提供以下信息：

| 字段/职责 | 语义 |
|---|---|
| `id`、平台、文档 | 规则身份、可用平台、CLI/TUI 展示名、说明与变体展开规则 |
| detector | 标识可用 provider、项目 marker 或已知规则；缺工具是可诊断的未发现，不是 scan 失败 |
| resolver | 按 `env -> 官方命令/配置 -> 平台默认` 解析候选路径，并产出 provenance |
| action factory | 仅从同一 descriptor 重建 canonical `ActionSpec`；命令始终保留 program/argv/cwd 分离 |
| safety contract | allowed roots、explicit non-targets、data class、live fingerprint/验证要求，交给 `SafetyPolicy::authorize()` 消费 |
| selection policy | 默认是否选中及 unresolved/incomplete 时的保守降级规则 |
| estimate semantics | logical/unknown/incomplete 语义与 warning 来源 |
| fixture matrix | 每条规则必须覆盖的默认、重定向、失败与平台案例 |

`devsweep rules`、TUI Rules tab、scanner/providers 生成的 target、plan validator 与 executor 都通过该 descriptor 或其上游 `ActionSpec` 投影读取 id/risk/action/selection。Catalogue 不再手工复制 provider 或 global-rule 的风险和动作字段。

## 发现、解析与安全数据流

1. registry 枚举当前平台可用规则及其 detector。
2. detector 通过 `ProcessRunner`/配置读取进行只读探测，返回 typed probe result，而非裸 `Option<String>`。
3. resolver 按优先级生成候选路径并记录 `Environment`、`OfficialCommand`、`Configuration` 或 `PlatformDefault` provenance。
4. 对命令 stdout 提取出的路径，先 trim，再要求 absolute、存在且是目录；任一条件失败都产生 `InvalidOutput`/unresolved diagnostic，绝不把警告文本、相对路径或不存在路径写入 target。
5. resolver 将 resolved footprint、provenance、size completeness 与 safety contract 交给 action factory。unresolved 或 incomplete 状态不得默认选中，size 显示为 unknown/incomplete，而不是 `0 B`。
6. validator 消费 canonical action；executor 在执行前调用中央 `SafetyPolicy::authorize()`。scan/probe 阶段不执行 trash、`go clean`、npm clean 或其他 cleanup 命令。

## 已采纳的安全默认

### Maven

`maven.repository` 的默认策略固定为 `High` + `InspectOnly`。`settings.xml` 或默认路径只证明仓库位置，不能证明其内容完全可从外部重新下载，因此不得自动降级或整仓 Trash。

仅当 resolver 提供可测试的、明确标注为 `ExternalCache` 的 provenance，且 safety contract 同时证明该 footprint 是 exclusively redownloadable external cache 时，才可定义单独的可清理变体。配置路径本身不是上述证明；证明缺失、解析失败或存在本地安装产物的可能时，一律保持 High/InspectOnly。

### npm

`npm cache clean --force` 保持非默认选择。`npm cache verify` 是同一 physical cache footprint 的必经预检/验证步骤，不创建第二个可见 cleanup target，不与 clean 一起作为扫描副作用。verify 失败、超时或输出不可验证时，clean 不得被自动升级或默认选中，并将 typed diagnostic 展示给用户。

### JetBrains 与 Go

JetBrains resolver 只能返回 product/version-specific 的 `caches`、`log`、`tmp` 子树；任何 vendor root（例如整个 `AppData/Local/JetBrains`）都是 explicit non-target。macOS 必须有独立 resolver/path set，不得沿用 Linux `.cache` 路径。

Go module cache 必须由 `go env GOMODCACHE` 解析，cleanup action 使用 `go clean -modcache` 的官方 command shape；扫描不得对 `~/go/pkg/mod` 直接构造 Trash action。

## 平台解析与 provider 规则

`home_dir` 改为 OS-specific resolver，不把 Windows、Unix 与 macOS 的环境变量回退混为同一无 provenance 的结果。Gradle、NuGet、Go 与 Maven 的解析必须遵循 env -> 官方命令/配置 -> 平台默认的顺序，并把实际来源记录为 evidence。

pip launcher 按当前平台顺序逐一尝试；launcher 存在但 `-m pip cache dir` 失败时必须继续下一个，而不是在第一个可执行文件处停止。Yarn classic/modern 变体要么作为 catalogue 的显式规则项，要么以单一文档化的变体展开规则从 descriptor 推导；两者都需要测试锁定。

## 兼容性、失败降级与回滚

本任务不自行序列化新的 plan 字段。所有 model/schema 演进通过 plan-validation owner；在接口到位前，规则可保留为 unresolved/InspectOnly，但不能伪造路径、风险或可执行 action。

规则行为改变是用户可见的：README 与 `devsweep rules` summary 必须说明 Maven 默认 inspect-only、JetBrains 精确子树、Go 官方 action 与 npm verify-first/non-default。若平台路径、配置格式或 provenance 证据无法验证，降级到 unresolved 或 InspectOnly；不回退到旧的 vendor-root/整仓 Trash 行为。

回滚以本子任务的独立代码提交为单位：恢复前一 registry projection，并保留上游 core 与安全策略的接口边界。不得通过删除 plan/audit 数据或临时放宽 safety contract 来恢复功能。

## 验收设计

测试必须证明 descriptor 是 catalogue、target risk/action/selection 与 UI/CLI 行的一致来源；以故意制造不一致的 fixture 或编译级约束防止漂移。provider 矩阵覆盖默认路径、env/config 重定向、缺工具、pip 多 launcher、Yarn 变体、无效 stdout、unknown size 与三平台路径集。

对 Maven、JetBrains 与 Go 的回归测试必须明确拒绝旧的整仓/vendor-root Trash。Windows、Linux、macOS 各自保留可执行 resolver 测试；不能取得某平台真实或 CI 证据时，该平台验收为 No-Go，而不是静默绿。
