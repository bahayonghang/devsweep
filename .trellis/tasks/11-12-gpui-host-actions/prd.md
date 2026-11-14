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
| R2 | core、CLI | 新增 MSI 卸载：按清单里的产品代码调用 `msiexec /x`，程序与参数分离，纳入 `software plan / preview / uninstall`。不读取注册表里的卸载命令字符串。 |
| R3 | GPUI | MSI 卸载接入（在 R2 之后）。 |
| R4 | GPUI | ARP 条目不执行。行上提供"在 Windows 已安装的应用中打开"，并说明原因。 |
| R5 | GPUI | winget 更新的执行：选择更新、计划、预览、确认、逐项结果。 |
| R6 | GPUI | 启动项开关：计划、预览、确认；结果可在界面上恢复。 |
| R7 | GPUI | 一次成功的卸载之后，提供残留的审阅和移入回收站；注册表残留按 CLI 现有的备份规则处理。 |
| R8 | GPUI | 结束进程：进程行的动作，预览后确认。受保护进程不提供该动作并说明原因。 |
| R9 | GPUI | 保持唤醒开关，显示当前状态；应用退出时释放。 |
| R10 | — | 每个动作的预览摘要在选择或主机状态变化后失效。没有有效摘要不能确认。 |
| R11 | — | 界面不弹出提权。需要提权的动作显示原因，并给出等价的 CLI 命令供复制。 |
| R12 | — | 每个执行结果写入历史，失败如实显示原因。 |

## 验收

- AC1（R1、R3、R5–R8）：每个动作有 GPUI 测试，用测试执行器或夹具覆盖：成功、失败、摘要过期被拒、未确认不执行。
- AC2（R2）：core 测试断言 MSI 卸载的程序是 `msiexec`、参数是分离的数组、产品代码通过格式校验；非法产品代码被拒。CLI 契约测试覆盖计划、预览、执行，缺少摘要或 `--confirm` 时拒绝。
- AC3（R2）：代码里没有读取或执行 `UninstallString`、`QuietUninstallString` 的路径；有检索证据和测试。
- AC4（R4）：ARP 条目没有卸载按钮，只有系统设置跳转。
- AC5（R7）：没有成功卸载记录时，残留清理不可用。文件只进回收站。
- AC6（R8）：受保护名单里的进程在界面和 core 两层都被拒。进程在预览后退出或 PID 被复用时，执行被拒。
- AC7（R9）：开启后 `status keep-awake status` 报告为开启；关闭或退出应用后报告为关闭。
- AC8（R11）：未提权时触发需要提权的动作，界面显示原因，不出现 UAC 提示。
- AC9：父任务的"等价 CLI"检查通过：每个动作在 `design.md` 的表里有对应命令。
- AC10：`just ci` 与 `just gpui` 通过。英文与中文原生截图覆盖每个动作的审阅与结果状态。

## 不在范围

执行厂商卸载命令字符串。所有用户范围的 MSIX 卸载和需要提权的卸载。强制结束受保护进程。永久删除残留。批量静默卸载。开机自启。

## 依赖

- 界面在 airy-glass-shell、software-evidence（列表）和 status-dashboard（进程表）之后。
- R2 可以先做。
- 完成后 tray-panel 才显示保持唤醒开关。
