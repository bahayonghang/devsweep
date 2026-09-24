---
skill: trellis-plan-review
version: 0.4.0
task_dir: D:/Documents/Code/Rust/Exp/devsweep/.trellis/tasks/08-29-mole-inspired-cli-windows-desktop-redesign
task_name: 08-29-mole-inspired-cli-windows-desktop-redesign
task_status: planning
review_scope: task-tree
task_count: 22
task_members:
  - 08-29-mole-inspired-cli-windows-desktop-redesign
  - 08-29-scan-resource-bounds-app-icon
  - 08-29-bound-sizing-concurrency
  - 08-29-generated-app-icon-integration
  - 08-29-cli-contract-localization
  - 08-29-desktop-shell-navigation-brand
  - 08-29-clean-mode-workbench
  - 08-29-analyze-mode
  - 08-29-analyze-core-ipc
  - 08-29-analyze-tui-desktop-treemap
  - 08-29-protect-rules-history-surfaces
  - 08-29-software-mode
  - 08-29-software-inventory-plan
  - 08-29-software-execution-audit
  - 08-29-software-cli-tui-desktop-native
  - 08-29-optimize-mode
  - 08-29-optimize-catalog-execution
  - 08-29-optimize-cli-tui-desktop-native
  - 08-29-status-mode
  - 08-29-status-collector-cli
  - 08-29-status-tui-desktop-native
  - 08-29-five-mode-native-integration
task_statuses:
  08-29-mole-inspired-cli-windows-desktop-redesign: planning
  08-29-scan-resource-bounds-app-icon: planning
  08-29-bound-sizing-concurrency: in_progress
  08-29-generated-app-icon-integration: planning
  08-29-cli-contract-localization: planning
  08-29-desktop-shell-navigation-brand: planning
  08-29-clean-mode-workbench: planning
  08-29-analyze-mode: planning
  08-29-analyze-core-ipc: planning
  08-29-analyze-tui-desktop-treemap: planning
  08-29-protect-rules-history-surfaces: planning
  08-29-software-mode: planning
  08-29-software-inventory-plan: planning
  08-29-software-execution-audit: planning
  08-29-software-cli-tui-desktop-native: planning
  08-29-optimize-mode: planning
  08-29-optimize-catalog-execution: planning
  08-29-optimize-cli-tui-desktop-native: planning
  08-29-status-mode: planning
  08-29-status-collector-cli: planning
  08-29-status-tui-desktop-native: planning
  08-29-five-mode-native-integration: planning
verdict: 需返回规划
blocking: 6
should_fix: 7
notes: 3
generated_at: 2026-08-29T20:48:27.7506059+08:00
---

# Trellis 规划审阅报告

## 审阅范围

- 根任务：08-29-mole-inspired-cli-windows-desktop-redesign
- 模式：task-tree
- 任务数量：22
- 有序成员（根优先；顺序不代表依赖）：
  - 08-29-mole-inspired-cli-windows-desktop-redesign — planning
  - 08-29-scan-resource-bounds-app-icon — planning
  - 08-29-bound-sizing-concurrency — in_progress
  - 08-29-generated-app-icon-integration — planning
  - 08-29-cli-contract-localization — planning
  - 08-29-desktop-shell-navigation-brand — planning
  - 08-29-clean-mode-workbench — planning
  - 08-29-analyze-mode — planning
  - 08-29-analyze-core-ipc — planning
  - 08-29-analyze-tui-desktop-treemap — planning
  - 08-29-protect-rules-history-surfaces — planning
  - 08-29-software-mode — planning
  - 08-29-software-inventory-plan — planning
  - 08-29-software-execution-audit — planning
  - 08-29-software-cli-tui-desktop-native — planning
  - 08-29-optimize-mode — planning
  - 08-29-optimize-catalog-execution — planning
  - 08-29-optimize-cli-tui-desktop-native — planning
  - 08-29-status-mode — planning
  - 08-29-status-collector-cli — planning
  - 08-29-status-tui-desktop-native — planning
  - 08-29-five-mode-native-integration — planning

`08-29-bound-sizing-concurrency` 是唯一已启动成员；本轮对其执行了 Pass 7 规划/实现漂移比对。其任务说明仍要求在恢复前重新取得明确批准，Measurement Record 仍未完成。

## 结论

需返回规划 — 阻断 6 / 应修 7 / 提示 3

## 问题清单

### TPR-01 · 阻断 · 图标子任务仍越权修改 React/header，违反已冻结的 shell 所有权

- Task: 08-29-generated-app-icon-integration
- Affected tasks: 08-29-scan-resource-bounds-app-icon、08-29-generated-app-icon-integration
- Location: `08-29-generated-app-icon-integration/task.json:5`、`08-29-generated-app-icon-integration/prd.md:18-19,44-48,76-85,93-99`、`08-29-generated-app-icon-integration/design.md:68-73`、`08-29-generated-app-icon-integration/implement.md:24-30,47,62-63,73-74`、`08-29-scan-resource-bounds-app-icon/task.json:5,8`
- Claim: 图标 PRD、design 和 scope 已把 React/TUI/CSS/header placement 交给 `08-29-desktop-shell-navigation-brand`，图标子任务只交付稳定 master、完整原生图标集、16/32 px 证据和 handoff；但 implement.md 仍要求修改 `desktop/src/App.tsx`、样式和 `App.test.tsx`，并把响应式 header 结果列为本子任务证据。两个 task.json 的 description/scope 也仍保留 React/header 交付。
- Evidence: `08-29-desktop-shell-navigation-brand/prd.md:39-41` 明确 shell 独占 React/TUI brand placement；根 `design.md:74-77` 明确 icon foundation 完成后才进入 shell；图标 `check.jsonl:2` 要求检查 React/TUI/CSS ownership 未泄漏回来。implement.md 的第 3 节和 Visual Evidence Record 与上述边界直接矛盾。
- Impact: 按 implement.md 执行会在 shell/spec 任务之前修改 shell 文件，违反图标 AC3/AC5 和根 AC6；同一 `App.tsx`/测试/样式会被两个任务拥有，无法确定提交、回滚和验收边界。
- Route: 保留已经冻结的 shell 独占方案：从图标 implement.md 删除 React/header 修改与相应证据项，只保留资产生成、原生 16/32 px 验证和 handoff；同步收窄父/子 task.json 的 description/scope。不要反向扩大图标 PRD 或重开 shell ownership。

### TPR-02 · 阻断 · CLI-owned `Locale` 被 core store 直接引用，形成不可实现的依赖环

- Task: cross-task
- Affected tasks: 08-29-cli-contract-localization、08-29-desktop-shell-navigation-brand
- Location: `08-29-cli-contract-localization/prd.md:34-41`、`08-29-cli-contract-localization/design.md:109-123,169`、`08-29-desktop-shell-navigation-brand/design.md:41-46,55-56`
- Claim: CLI task 独占 `Locale`、catalogue 和 resolver；shell design 同时要求 `crates/devsweep-core/src/presentation_settings.rs` 的 `PresentationSettingsV1` 直接存储 `Option<Locale>`。
- Evidence: 当前依赖方向只有 `devsweep-cli -> devsweep-core`（`crates/devsweep-cli/Cargo.toml:15-23`）；`devsweep-core` 不依赖 CLI（`crates/devsweep-core/Cargo.toml:11-30`），Tauri 同样只依赖 core（`desktop/src-tauri/Cargo.toml:20-27`）。core 无法引用定义在 CLI crate 内的类型而不制造循环或新增反向依赖。
- Impact: persisted-language AC 无法按计划编译实现；临时改成互不相关的字符串/枚举会破坏单一 locale 合同、未知值处理和 TUI/Desktop/CLI 一致性。
- Route: 其一，把最小、稳定、locale-neutral 的语言值类型下沉为 core-owned DTO，CLI 继续独占 catalogue、解析、格式化和 human rendering；其二，让 core store 保存明确版本化并受校验的稳定语言标签，CLI/shell 通过同一个校验边界映射。两条路线都必须同步冻结 store 的序列化、路径和 unknown/corrupt 处理，且不得让 core 依赖 CLI。

### TPR-03 · 阻断 · 新 Clean V1 audit writer/schema 和固定路径无人负责

- Task: 08-29-clean-mode-workbench
- Affected tasks: 08-29-clean-mode-workbench、08-29-protect-rules-history-surfaces
- Location: `08-29-clean-mode-workbench/prd.md:9-12,30-32`、`08-29-clean-mode-workbench/design.md:30-35,41-59`、`08-29-clean-mode-workbench/implement.md:7-24`、`08-29-protect-rules-history-surfaces/design.md:14-17,24-28`
- Claim: Clean 要保留 execution -> audit，History 必须读取已经实现并接受的固定 V1 Clean/Software/Optimize journals；CLI contract 已冻结 `%LOCALAPPDATA%\DevSweep\audit\v1\<domain>.jsonl`、不导入旧 audit 的行为。Clean exact change list 却不包含 audit writer、schema、固定路径 resolver 或锁机制。
- Evidence: 当前 `crates/devsweep-core/src/execution/audit.rs:155-184` 的 `JournalEvent` 无 `schema_version`，仍包含 `command` 与 `action_path`；`:324-331` 的默认位置仍为旧 `audit.jsonl`。Software/Optimize 各自声明新 audit owner，只有 Clean 没有，而 History 的 phase gate 要求三者都先存在。
- Impact: Clean AC1、History R3/AC3 和最终 audit-version fixture 没有可实现机制；若直接复用当前 journal，固定路径、版本拒绝和“不可重建 argv/path”的红线都会被违反。
- Route: 由 Clean 子任务显式拥有 Clean V1 audit writer/schema、固定路径解析、独占锁、旧文件不发现/不导入、unknown-version fail-closed 和 History redaction fixture；把实际 core 路径加入 exact change list/implement，并同步 History 的 handoff 验收。

### TPR-04 · 阻断 · “current-user MSI 可直接卸载”无法同时证明精确 context 与严格无提权

- Task: 08-29-software-execution-audit
- Affected tasks: 08-29-mole-inspired-cli-windows-desktop-redesign、08-29-software-mode、08-29-software-inventory-plan、08-29-software-execution-audit、08-29-software-cli-tui-desktop-native
- Location: `08-29-software-inventory-plan/prd.md:9-24`、`08-29-software-execution-audit/prd.md:9-18,29-37`、`08-29-software-execution-audit/design.md:11-17`、`08-29-software-execution-audit/implement.md:25-46`、`08-29-software-cli-tui-desktop-native/prd.md:13-19,31-33`
- Claim: MSI identity 是 ProductCode + context，机器范围拒绝；执行方案却固定调用只接收 ProductCode 的 `MsiConfigureProductExW`，并声称所有 supported current-user MSI 都能在 `asInvoker` 下无 UAC/无 elevated child 卸载。
- Evidence: windows-sys 0.59 的 `MsiEnumProductsExW` 返回 SID/context，而 `MsiConfigureProductExW` 无 SID/context 参数；Windows Installer 支持同一 ProductCode 存在于不同 installation context。Microsoft 的 `Determining Installation Context` 与 `Using Windows Installer with UAC` 文档还说明 per-user-managed 产品的后续 Installer 操作使用提升权限。官方依据：`https://learn.microsoft.com/en-us/windows/win32/msi/determining-installation-context`、`https://learn.microsoft.com/en-us/windows/win32/msi/using-windows-installer-with-uac`、`https://learn.microsoft.com/en-us/windows/win32/api/msi/nf-msi-msiconfigureproductexw`。
- Impact: 当前方案可能对错误 context 操作，或由 Windows Installer 以提升权限执行，直接违反根 no-UAC/no-elevated-child 与 Software AC1/AC3/AC4；仅以“current-user”分类不足以证明安全。
- Route: 其一，v1 只执行 current-user MSIX，全部 MSI 仅 inventory/manual；其二，只允许经 SID/context、多实例冲突和 managed/elevation 预检证明为 current-user USERUNMANAGED 的 MSI，任何不确定项均 manual，并增加标准用户原生证据。该产品范围选择尚未得到用户答复，不能由实施者临时决定。

### TPR-05 · 阻断 · Software dispatch 后终态、`partial` 与 crash/restart 恢复没有确定状态机

- Task: 08-29-software-execution-audit
- Affected tasks: 08-29-software-mode、08-29-software-execution-audit、08-29-software-cli-tui-desktop-native
- Location: `08-29-software-execution-audit/prd.md:16-25,32-39`、`08-29-software-execution-audit/design.md:23-38`、`08-29-software-execution-audit/implement.md:25-36,58-60`、`08-29-software-cli-tui-desktop-native/prd.md:16-18,23-28`
- Claim: 规划要求区分 removed、still-present、reboot-required、failed、partial、unknown-after-dispatch，并覆盖 cancel、timeout、app crash/restart；design 只规定 0/2/10 秒 requery 和冲突时 unknown。
- Evidence: design 未定义 adapter result 与 requery 冲突优先级、removed + reboot evidence 的终态、`partial` 成立条件、crash 后由谁何时重查，以及 dispatch 前后 durable journal 的精确恢复语义。implement.md 只列出 crash/restart 原生场景，没有恢复步骤或转移表。
- Impact: 同一 OS 结果可被不同实现分类为 success/partial/reboot/unknown；崩溃后还可能重试不可逆卸载或永远遗留无法解释的 intent，AC2/AC4 和前端 fixture parity 不可判定。
- Route: 在 execution design/implement 中补完整状态/证据优先级与恢复转移表；副作用前持久化 dispatch intent，禁止自动重试；重启后按 tagged identity 重查，无法核实时保守为 unknown-after-dispatch。保留或删除 `partial` 必须与现有 R3/AC/表现层 fixture 同步，不能留给实现阶段猜测。

### TPR-06 · 阻断 · Status JSON/NDJSON/IPC wire contract 未按根 ownership 冻结

- Task: cross-task
- Affected tasks: 08-29-mole-inspired-cli-windows-desktop-redesign、08-29-cli-contract-localization、08-29-status-mode、08-29-status-collector-cli、08-29-status-tui-desktop-native
- Location: `08-29-mole-inspired-cli-windows-desktop-redesign/prd.md:254-256,329-332`、`08-29-cli-contract-localization/design.md:100-107`、`08-29-status-collector-cli/prd.md:9-25,35-39`、`08-29-status-collector-cli/design.md:14-21,23-40`、`08-29-status-tui-desktop-native/design.md:3-11,18-29`
- Claim: 根任务与 CLI task 声称在下游实现前冻结 JSON/NDJSON schemas；CLI design 只给通用 envelope/event。Status collector 声称消费 frozen envelope 并拥有 V1 DTO，UI 又直接消费 frozen stream。
- Evidence: 全树没有给出 Status snapshot data/event 的具体字段、primitive types、单位和时间基准、availability tagged-union wire shape、process truncation/partial 元数据、terminal/broken-pipe event。Status PRD 只列语义字段，无法唯一导出 JSON/NDJSON/Tauri fixture。
- Impact: CLI、Tauri generator、React decoder/TUI renderer可以各自发明不兼容形状；root AC11、CLI AC2 和 Status AC2 的跨表面 parity 无法在实现前判定。
- Route: 按既有 root ownership 在 CLI contract 中冻结 Status snapshot/NDJSON common wire shape，并在 Status collector/UI design 与 AC 中同步 payload/event、单位/时间、availability/truncation/terminal 机制；不要把 wire 决定留给各适配器独立发明。

### TPR-07 · 应修 · Clean、Analyze 与 support CLI human renderer 没有文件所有者

- Task: cross-task
- Affected tasks: 08-29-cli-contract-localization、08-29-clean-mode-workbench、08-29-analyze-core-ipc、08-29-protect-rules-history-surfaces
- Location: `08-29-cli-contract-localization/design.md:160-185`、`08-29-clean-mode-workbench/design.md:41-59`、`08-29-analyze-core-ipc/design.md:54-70`、`08-29-protect-rules-history-surfaces/design.md:34-55`
- Claim: CLI task 建立 `application/presentation/mod.rs`，要求 downstream task 创建 `application/presentation/<mode>.rs`；Software、Optimize、Status 已列出对应 renderer，但 Clean、Analyze、Protect/Rules/History exact change lists 只列 handler/TUI/Desktop 文件。
- Evidence: 全树 `application/presentation` 路径只有 CLI root、Software、Optimize、Status 命中；上述三个下游任务没有 renderer path 或实施步骤，尽管其 AC 要求 bilingual human output/fixture parity。
- Impact: 实施者必须把 human rendering 偷塞进 handler、临时扩大 CLI task，或跳过多语言快照；这会破坏刚冻结的单一 presentation tree 和任务所有权。
- Route: 在 CLI contract 中枚举完整 renderer tree，并将 `presentation/clean.rs`、`presentation/analyze.rs`、`presentation/{protect,rules,history}.rs` 分配给对应任务；同步 exact change list、implement 和聚焦验证。

### TPR-08 · 应修 · Clean 的 Cargo 聚焦门禁把三个过滤词写进单条命令

- Task: 08-29-clean-mode-workbench
- Affected tasks: 08-29-clean-mode-workbench
- Location: `08-29-clean-mode-workbench/implement.md:16-21`
- Claim: 聚焦验证为 `rtk cargo test -p devsweep-core scan plan execution`。
- Evidence: 本机按原文运行（加 `--no-run` 仅避免实际测试）返回 exit 1：`unexpected argument 'plan' found`；Cargo test 只接受一个 `[TESTNAME]`。
- Impact: Clean 第一个 rollback gate 在实施时不能启动，后续实现者只能脱离计划临时改命令或跳过核心门禁。
- Route: 拆成 `scan`、`plan`、`execution` 三条独立 package test filter，或改为一条无 filter 的 `devsweep-core` 包级测试；保留原有三类覆盖意图。

### TPR-09 · 应修 · Protection 并发写、损坏恢复与 mutation audit 只有 AC，没有机制

- Task: 08-29-protect-rules-history-surfaces
- Affected tasks: 08-29-protect-rules-history-surfaces
- Location: `08-29-protect-rules-history-surfaces/prd.md:9-11,26-28`、`08-29-protect-rules-history-surfaces/design.md:3-7,19-20`、`08-29-protect-rules-history-surfaces/implement.md:9-19`
- Claim: AC1 要求 concurrent writers、corrupt store recovery、atomic update 和 mutation audit；design 只写 temp-file + atomic replace，implement 只泛称 concurrent/corrupt fixtures。
- Evidence: 当前 `crates/devsweep-core/src/execution/safety/protections.rs:109-126,185-235` 在内存修改后写文件，使用 PID 临时文件但无跨 writer lock/read-modify-write 冲突协议；规划也未定义 corrupt/unknown 原字节保留、fail-closed 行为或 protection mutation audit owner/schema。
- Impact: 两个前端并发更新可能丢失写入；损坏文件可能被当空列表覆盖；AC1 的“recovery”可被实现成互相矛盾的删除、重置或拒绝，且 History 无法解释 protection mutation audit。
- Route: 冻结锁覆盖的 load/compare/atomic-replace 协议和冲突测试；corrupt/unknown 保留原字节并 fail closed，不能静默当空列表；明确 protection mutation audit 的 owner/schema。若要自动清空或删除损坏文件，必须另行取得用户决定。

### TPR-10 · 应修 · Software eligibility/refusal 矩阵与可选 evidence DTO 未冻结

- Task: 08-29-software-inventory-plan
- Affected tasks: 08-29-software-inventory-plan、08-29-software-cli-tui-desktop-native
- Location: `08-29-software-inventory-plan/prd.md:9-24,36-46`、`08-29-software-inventory-plan/design.md:31-54`、`08-29-software-cli-tui-desktop-native/prd.md:9-15`
- Claim: Inventory 必须为每种来源给出 exact eligibility/refusal，presentation 只在有证据时显示 size/last-used；上游只泛列 system/update/framework/resource/protected/unsupported。
- Evidence: `08-29-mole-inspired-cli-windows-desktop-redesign/research/windows-software-boundary.md:19-22` 还列出 `NoRemove`、hidden、dependency/stub、unhealthy/conflicting/incomplete 等状态。Inventory design 虽读取部分 MSIX flags，却未把每个来源字段映射到 eligibility/refusal，也未冻结 size/last-used 的来源、单位、置信度或 unknown wire value。
- Impact: 相同条目可能在 CLI/TUI/Desktop得到不同可选性；缺乏证据的数据可能显示为 0 或被误认为完整，违反 Software R1/R2。
- Route: 增加逐来源 eligibility/refusal 表并冻结 optional evidence DTO；若 v1 不采集 size/last-used，则明确为 unknown，而不是留给表现层推断。

### TPR-11 · 应修 · Status 所需 windows-sys feature 集未在实施计划中列明

- Task: 08-29-status-collector-cli
- Affected tasks: 08-29-status-collector-cli
- Location: `08-29-status-collector-cli/prd.md:9-12,37-39`、`08-29-status-collector-cli/design.md:14-17,23-40`、`08-29-status-collector-cli/implement.md:5-22`
- Claim: design 承诺枚举现有 windows-sys 的最小 feature additions，implement 只写“feature flags together”，没有 feature/API 清单。
- Evidence: 当前 `crates/devsweep-core/Cargo.toml:20-27` 只有 Foundation、Security、Storage_FileSystem、JobObjects、Threading。windows-sys 0.59 源码把计划 API 分别放在 `Win32_NetworkManagement_IpHelper`（`GetIfTable2`）、`Win32_System_Diagnostics_ToolHelp`、`Win32_System_ProcessStatus`、`Win32_System_Power`、`Win32_System_SystemInformation`；对应 crate `Cargo.toml:114,205,236-237,253` 的 features 均存在。
- Impact: 实施者必须临时猜测 feature 集，AC4 的“minimal and documented”无法按计划复核，也可能遗漏编译条件或加入过宽 feature。
- Route: 在 Status design/implement 明列 current + added exact flags 及每个 API 的映射，并让 AC4/聚焦门禁验证最小化。

### TPR-12 · 应修 · Status post-exit 数值门禁未定义取样判据

- Task: cross-task
- Affected tasks: 08-29-status-mode、08-29-status-tui-desktop-native、08-29-five-mode-native-integration
- Location: `08-29-status-mode/prd.md:18-22,31-33`、`08-29-status-tui-desktop-native/implement.md:31-32`、`08-29-five-mode-native-integration/design.md:19-45`
- Claim: Status stop 后五秒内 CPU 要回到 `idle+0.5 percentage point`、threads 回到 `idle+1`；统一协议只定义 200 ms sampling、nearest-rank p95、median 和 maximum。
- Evidence: 规划没有说明 post-exit 使用五秒窗口的最后一个样本、首次满足样本、整个窗口最大值、连续保持窗口或其他统计量；这些解释会对同一 trace 给出不同 PASS/FAIL。
- Impact: 根资源 AC14 与 Status AC3 的结果不确定，实施者可以通过选择有利样本获得通过。
- Route: 在 integration protocol 定义精确 post-stop window/statistic/pass predicate，并同步 Status parent/UI verification；不得在看到样本后再选择统计方法。

### TPR-13 · 应修 · 三个 implement 调用不存在的 npm `check` script

- Task: cross-task
- Affected tasks: 08-29-protect-rules-history-surfaces、08-29-status-tui-desktop-native、08-29-five-mode-native-integration
- Location: `08-29-protect-rules-history-surfaces/implement.md:28-31`、`08-29-status-tui-desktop-native/implement.md:21-27`、`08-29-five-mode-native-integration/implement.md:23-29`
- Claim: 三处验证列表调用 `rtk npm --prefix desktop run check`。
- Evidence: `desktop/package.json:10-18` 只有 `lint`、`typecheck`、`test`、`build`、`types:generate`、`tauri` 等脚本，无 `check`。本机按原文运行返回 exit 1：`Missing script: "check"`。
- Impact: 三个叶任务/集成任务的桌面检查会在脚本解析阶段失败，无法达到各自的注册和最终门禁。
- Route: 三处同步替换为现有 `rtk npm --prefix desktop run lint` 与 `rtk npm --prefix desktop run typecheck`；其余已单列的 test/build/types 命令保持不变。不要无归属地新增 `check` script。

### TPR-14 · 提示 · Sizing 的 pre-change Evidence 行号已被 in-progress diff 推移

- Task: 08-29-bound-sizing-concurrency
- Affected tasks: 08-29-bound-sizing-concurrency
- Location: `08-29-bound-sizing-concurrency/prd.md:11-27`
- Claim: PRD 仍以当前时态引用 `sizing.rs:224-236,229-231,238-255,637-639` 证明原 uncapped `par_iter()`、预算语义和伪 serial helper。
- Evidence: live `sizing.rs` 已因该任务的 531-line diff 改变；当前机制在 `:70-87,277,321-363,493-516,885-899` 一带，原行号不再承载这些基线事实。原事实仍可在任务启动前的基线提交上核对。
- Impact: Pass 7 审阅者按 live path:line 打开会得到错误证据，容易把“基线事实”与“当前 paused checkpoint”混为一谈。
- Route: 把这些行号标为明确的 pre-change commit evidence，并补当前 paused checkpoint 的 live anchors；Measurement Record 的 TBD 不得改成已验证。

### TPR-15 · 提示 · 两个基础叶任务的命令未遵循当前 RTK 前缀约定

- Task: cross-task
- Affected tasks: 08-29-bound-sizing-concurrency、08-29-generated-app-icon-integration
- Location: `08-29-bound-sizing-concurrency/implement.md:29-33,65-72`、`08-29-generated-app-icon-integration/design.md:42-48`、`08-29-generated-app-icon-integration/implement.md:32-36,49-56`
- Claim: 这些 code fence 直接运行 cargo/just/git/Tauri CLI，而同树其余任务使用 `rtk`。
- Evidence: 当前 personal RTK contract `C:/Users/lyh/AppData/Roaming/orca/codex-runtime-home/home/RTK.md:7-15` 要求 shell 外部命令统一加 `rtk`；仓库任务树的大多数验证命令已遵循。
- Impact: 命令本身可执行，但后续 Codex 实施会偏离当前工具约定，产生不一致的日志与输出压缩行为。
- Route: 统一添加 `rtk`；图标生成可使用现有 `npm run tauri -- icon ...` 脚本形式，不新增依赖。

### TPR-16 · 提示 · 最终 README 变更与既有用户 hunk 同文件重叠，计划未写保留方法

- Task: 08-29-five-mode-native-integration
- Affected tasks: 08-29-five-mode-native-integration
- Location: `08-29-five-mode-native-integration/design.md:69-71`、`08-29-five-mode-native-integration/implement.md:42-44`
- Claim: 集成任务计划修改 `README.md`，同时泛称保留 unrelated dirt。
- Evidence: 当前 `git status --short` 显示 `README.md` 已有用户修改；README 又位于集成 exact change list 内，因此仅按“change list 外脏文件”规则不能识别并保护同文件既有 hunks。
- Impact: 实施者可能把既有 README hunk 误当本任务变更一起改写或提交。
- Route: 在集成 baseline 步骤中记录并保留 README 的既有 hunk，实施/提交时按 hunk 归属复核；不要把它误列为可覆盖的干净基线。

## 未能核实

- MSI v1 是选择 “MSIX-only execution” 还是 “仅 USERUNMANAGED MSI execution” —— 已通过 AskUserQuestion 请求决定，但本轮未收到有效选择；TPR-04 保持阻断。
- 标准用户 MSI/MSIX 原生卸载、UAC/完整性级别进程树、MSIX 依赖包副作用与 crash/restart 恢复 —— 未执行，只有规划和官方 API 文档证据。
- 全部原生 Windows UI 证据（100/125/150/200% 缩放、键盘/屏幕阅读器、高对比/减少动画、16/32 px 标题栏/任务栏图标）—— 未构建或人工检查应用。
- `08-29-bound-sizing-concurrency` 的吞吐 A/B、worker 数和 Measurement Record —— 仍为 TBD；本轮只跑了聚焦测试，不能替代同机 release 测量。
- Status/Analyze/Software/Optimize 的原生资源阈值、sleep/resume、Defender 与 Settings policy 行为 —— 未运行固定主机协议。
- 五张 Mole for Mac 截图的逐项 accept/adapt/reject 视觉对应 —— 截图不在仓库文件作用域内，仅核实了研究记录存在。
- macOS/Linux 图标 native appearance —— 无对应主机，保持 UNVERIFIED。

## 可靠部分

- 22/22 个 `task.py validate` 均通过；`plan_precheck.py --include-descendants` 解析 22 个成员且结构 blocking items 为 0。所有成员都有 PRD/design/implement/JSONL 产物，JSONL 无仅 `_example` 的 seed。
- 上一轮 TPR-01～TPR-11 的修订已分别由两个只读审阅者复核为 PASS：Optimize WDK feature、Cargo package 名、CLI module tree、Tauri/type-generator/doc 路径、spec-first palette、legacy audit disposition、本地化复数/单位/accelerator/truncation、sizing paused 状态、协调型 implement.md、nearest-rank p95 和通用 dirty-path 保护均不应重做。
- 当前命令事实已直接复现：`rtk cargo test -p devsweep-core scan plan execution --no-run` 在第二个 filter 处失败；`rtk npm --prefix desktop run check` 因脚本不存在失败。相对地，`rtk npm --prefix desktop run tauri build -- --help` 正确解析为 Tauri build，Vitest `--run`、`just desktop-web-check`、`just desktop-build` 和 `just ci` 形态存在。
- Pass 7 当前聚焦检查通过：`rtk cargo test -p devsweep-core filesystem::sizing` 为 16 passed；`rtk cargo test -p devsweep-desktop scan` 为 8 passed；`git diff --check` 通过。当前 sizing 产品 diff 仍只落在其声明的 `crates/devsweep-core/src/filesystem/sizing.rs`，但性能 AC 未验证。
- Microsoft 当前 Launch Windows Settings 文档明确写明 `ms-settings:energyrecommendations` 适用于 Windows 11 22H2 build 22624+，旧报告中的该外部声明已补核。
- Optimize 的 `RtlGetVersion`/WDK feature、WOW64/System32/Sysnative 和八项 catalogue 机制本轮未发现新冲突；Software 的 `windows =0.56.0` 版本/feature/license/Rust 基线仍可复核，问题集中在 MSI context/提权与状态机，而非 MSIX projection 是否存在。
- windows-sys 0.59 的 Status API/feature 映射可在本机 crate source 直接解析；TPR-11 是计划未列清单，不是 API 不存在。

## 盲区

An agent reviewing an agent's plan is not an independent second opinion. The reviewer and the
author share most of the same blind spots. A clean report means "this pass found nothing", not
"the plan is complete". Treat the findings as a triage list, not as an approval.
