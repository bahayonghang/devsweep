# Optimize 批量与默认勾选调研

- 问题：七项可执行操作的真实前置条件、时间约束、审计粒度及旧客户端依赖。
- 范围：集成工作树的生产源码、既有夹具和微软工具文档，未在本机运行维护。
- 日期：2026-11-20。

## D6 默认值

建议七项均由用户主动勾选。可选较积极的默认值是只选 DNS 刷新；也可按现有适配器选择 DNS 和时间偏差探测，接受后者连接 time.windows.com、普通账户可能受平台策略限制的代价。这两个操作的现有适配器不请求提权；这项源码证据不足以承诺所有标准账户都能成功。Microsoft 对 w32tm 本地使用给出管理员要求，DNS 故障诊断也从管理员终端开始。[Windows Time 工具](https://learn.microsoft.com/en-us/windows-server/networking/windows-time-service/windows-time-service-tools-and-settings) · [DNS 客户端诊断](https://learn.microsoft.com/en-us/windows-server/networking/dns/troubleshoot/troubleshoot-dns-client) · [ipconfig](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/ipconfig)

| ID | 生产预检与影响 | 等待上限 |
| --- | --- | --- |
| `dns.flush` | 验证绝对 System32/Sysnative ipconfig 身份，独立 argv 仅刷新解析器缓存；无显式 token gate | 10 秒，进程树终止有单独宽限 |
| `maintenance.clock_probe` | 固定 w32tm，取 3 次偏移，每隔 1 秒，网络请求，未改本机时钟 | 30 秒 |
| `maintenance.clock_sync` | 已提权，未待重启，固定 w32tm，触发系统时间重同步 | 60 秒 |
| `maintenance.retrim` | 已提权，电量、卷健康、SSD、有效卷、未待重启、7 日冷却、defrag 可用 | 15 分钟 |
| `maintenance.image_scan` | 已提权、未待重启、DISM 可用，在线扫描映像 | 30 分钟 |
| `maintenance.image_repair` | 在扫描前置条件之上要求可修复证据；生产适配器当前固定为 false，应显示跳过原因 | 60 分钟 |
| `maintenance.dead_startup` | 当前实现也要求已提权，先写备份，再检查目标确实消失，才处理 Run 项或回收 Startup 快捷方式 | 此路径没有命令等待计时；不要套用通用 60 秒 |

这些是适配器的上限。没有隔离性能样本，无法据此报告典型实测时长。运行中某些映像/维护进程在等待超时后仍可能运行；报告未知/超时，不能提前声明停止或回滚。`crates/devsweep-core/src/optimize/catalogue.rs:97–189`、`execution.rs:29,98–112`、`windows.rs:44–67,97–107`、`maintenance.rs:30–41,163–171,198–340,361–432,440–469,495–525`。

## 计划、审计和兼容

- 19 项包含 7 个 Execute、3 个 SettingsHandoff、8 个 Guidance、1 个 Observe。设置跳转可在旧单项计划中使用。某些 V2 目录指引有可保存的只读 V1 计划，但其预览拒绝执行。维持这些既有 V1/单个参数行为，在新批量计划入口限制 Execute。计划版本与 catalogue_version 是两套版本。`optimize/plan.rs:156–165,248–280,311–359`。
- 一份批次一个有序摘要。纳入每项 ID、类、实时可用或跳过原因，以及可执行项原有摘要。确认时拒绝过期整份摘要。后续每项重新预检，以免前项改变后项状态；维持单项固定程序/argv、幂等/审计前置，不能把预览时拒绝的操作自动升级为可执行。
- 现有审计每项包括多个事件，Validated、DispatchStarted、AdapterCompleted、Terminal；唯一运行 operation_id 与 catalogue_id 不同。批次需要共享批次 ID 而保留各项生命周期，否则历史重放把两项混为重复派发。历史成功只计 Execute 的真实 Succeeded 终态。取消只发生在项之间，明确记录哪些未开始；一项失败后继续。建议用 fake 覆盖成功、跳过、失败、项间取消、权限和写审计失败。`optimize/audit.rs:54–91`、`execution.rs:284–350,438–469`。
- TUI 的事件和状态拥有单个 `MaintenancePlanV1`；worker 单项 `plan_operation` 后取实时预览，再用旧执行入口。不要求 TUI 改成批量界面。旧 JSON、旧计划 digest、Settings handoff、TUI 报告语义分别做兼容测试。`crates/devsweep-cli/src/tui/app/events.rs:66,145`、`tui/modes/optimize/mod.rs:42`、`tui/runtime/workers.rs:224–228,256`。

## 规范与限制

参见 `.trellis/spec/backend/provider-authority.md`、`error-handling.md`、`quality-guidelines.md` 与本任务 PRD。没有运行真实 DNS 清空、时间同步、重整、映像维护或启动项修改。表中提权条件描述现有代码；权限检测或环境拒绝需在界面和历史中按实际证据呈现。
