# 桌面主机动作接入：卸载、更新、启动项、进程与保持唤醒

父任务：[11-12-gpui-mole-parity-airy-glass](../11-12-gpui-mole-parity-airy-glass/prd.md)。差距编号 W3、T5 见父任务 `research/gap-analysis.md`。

## 目标

在桌面端完成卸载、更新、启动项开关、清理卸载残留、结束进程和保持唤醒，不用回到命令行。每个动作都走"保存计划 → 实时摘要 → 二次确认"。

用户在 2026-11-12 选定范围 A（全部接入）。

## 背景

| 能力 | CLI / core | GPUI 现状 |
| --- | --- | --- |
| 卸载（当前用户 MSIX） | `software uninstall`；`software/execution/mod.rs` | 拒绝（`software_review.rs:604-720`，`copy.rs:1590`） |
| 安装更新（winget） | `software updates execute`（`software/updates.rs:202-228`） | 只检查（`workbench_review.rs:166`） |
| 启动项开关 | `software startup execute`（`software/startup.rs:564-580`） | 只预览（`software_review.rs:800-818`） |
| 卸载残留 | `software leftovers`；计划需要一次成功的卸载记录 | 不可用（`copy.rs:281`） |
| 结束进程 | `status process terminate`；受保护进程名单（`status/process_action.rs:143`） | 只读 |
| 保持唤醒 | `status keep-awake`；`KeepAwakeService` | 未接入 |

ARP 与 MSI 条目在 core 里是"manual"。core 不把注册表里的厂商卸载命令当作授权依据。

## 需求

| 编号 | 层 | 需求 |
| --- | --- | --- |
| R1 | GPUI | 当前用户 MSIX 应用的卸载：选择、计划、预览摘要、二次确认、结果。 |
| R2 | core、CLI | 新增 MSI 卸载：仅开放正向资格证据完整、当前用户非托管且上下文唯一的条目；系统绝对路径 `msiexec.exe`，独立参数固定为 `/x {GUID} /qn /norestart`，纳入 `software plan / preview / uninstall`。不读取注册表里的卸载命令字符串。 |
| R3 | GPUI | MSI 卸载接入（在 R2 之后）。 |
| R4 | GPUI | ARP 条目不执行。行上提供"在 Windows 已安装的应用中打开"，并说明原因。 |
| R5 | GPUI | winget 更新的执行：选择更新、计划、预览、确认、逐项结果。 |
| R6 | GPUI | 启动项开关：计划、预览、确认；结果可在界面上恢复。 |
| R7 | GPUI | 一次成功的卸载之后，提供残留的审阅和移入回收站；注册表残留按 CLI 现有的备份规则处理。 |
| R8 | GPUI | 结束进程：进程行的动作，预览后确认。受保护进程不提供该动作并说明原因。 |
| R9 | GPUI | 保持唤醒开关，显示当前状态；应用退出时释放。 |
| R10 | — | 每个动作的预览摘要在选择或主机状态变化后失效。没有有效摘要不能确认。 |
| R11 | — | DevSweep 不主动请求提权、不使用 `runas` 或提权回退，固定关闭 Installer 内部 UI；不对任意第三方 MSI 自定义动作作绝对零 UAC 承诺。已知需管理员、机器级、托管、其他用户、多上下文或身份/限制证据不完整的 MSI 拒绝，并说明原因。等价 CLI 指引不隐含提权或绕过资格限制。 |
| R12 | — | 每个执行结果写入历史，失败如实显示原因。 |

## 验收

- AC1（R1、R3、R5–R8）：每个动作有 GPUI 测试，用测试执行器或夹具覆盖：成功、失败、摘要过期被拒、未确认不执行。
- AC2（R2）：core 测试断言 MSI 卸载的程序是 `msiexec`、参数是分离的数组、产品代码通过格式校验；非法产品代码被拒。CLI 契约测试覆盖计划、预览、执行，缺少摘要或 `--confirm` 时拒绝。
- AC3（R2）：代码里没有读取或执行 `UninstallString`、`QuietUninstallString` 的路径；有检索证据和测试。
- AC4（R4）：ARP 条目没有卸载按钮，只有系统设置跳转。
- AC5（R7）：没有成功卸载记录时，残留清理不可用。新的 MSI 仅 `0` 且确认同一 SID/context 登记消失可记为 `Removed` 并获得残留权限；`3010/1641`、失败、未知或仅观察到消失都不能开放残留。文件只进回收站，保持旧 MSIX 历史解释兼容。
- AC6（R8）：受保护名单里的进程在界面和 core 两层都被拒。进程在预览后退出或 PID 被复用时，执行被拒。
- AC7（R9）：应用仍运行时，CLI 通过显式 endpoint/token/owner 查询指定 GUI 会话为 active/stopped；停止后端点保持可查询，只有清除成功才报告 stopped/off。GUI 退出在同一 owner 线程清除自身请求并 join；旧端点不可达时 CLI 报告 unavailable/unknown，不把连接失败当作 off。不影响独立 owner，不新增生命周期收据或常驻服务。
- AC8（R11）：已知需要提权或不符合已批准 MSI 资格的动作在派发前拒绝并显示原因；DevSweep 不发起提权或交互 Installer 回退。测试断言拒绝时无执行调用。第三方自定义动作不在绝对零 UAC 保证内；未做实机测试时不宣称原生 UAC 验收通过。
- AC9：父任务的"等价 CLI"检查通过：每个动作在 `design.md` 的表里有对应命令。
- AC10：`just ci` 与 `just gpui` 通过。英文与中文原生截图覆盖每个动作的审阅与结果状态。

## 不在范围

执行厂商卸载命令字符串。所有用户范围的 MSIX 卸载和需要提权的卸载。强制结束受保护进程。永久删除残留。批量静默卸载。开机自启。

## 依赖

- 界面在 airy-glass-shell、software-evidence（列表）和 status-dashboard（进程表）之后。
- R2 可以先做。
- 完成后 tray-panel 才显示保持唤醒开关。

## 步骤 2 已确认（2026-11-15）

用户确认固定参数、MSI 结果/残留规则、R11 提权边界和修订后的 AC7，授权继续步骤 3–8。完整决定见 `research/step-2-review.md`；步骤 9 前与步骤 10 前仍分别评审。测试仅用夹具和测试执行器，不使用 Computer Use，不做真实主机变更，不提交或推送。

## 步骤 9 已确认（2026-11-16）

用户按 `research/step-9-review.md` 批准进程动作的 core/CLI 审计与 GPUI 接入，实施和检查仍由 Trellis 子代理完成，只用夹具，不使用 Computer Use。完成门禁后停在步骤 10 前；不提交或推送。
