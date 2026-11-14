# 设计：桌面主机动作

## 等价 CLI

| 桌面动作 | core | CLI |
| --- | --- | --- |
| 卸载 MSIX | `software/execution/mod.rs` 的当前用户 MSIX 分派 | `software plan` → `software preview` → `software uninstall --plan --preview-digest --confirm` |
| 卸载 MSI | 新增的 MSI 分派 | 同上（新增支持） |
| 安装更新 | `software/updates.rs:202-228` | `software updates plan / preview / execute` |
| 启动项开关 | `software/startup.rs:564-580` | `software startup plan / preview / execute` |
| 残留 | `software/leftovers.rs` | `software leftovers scan / plan / preview / execute` |
| 结束进程 | `status/process_action.rs:165-202` | `status process plan / preview / terminate` |
| 保持唤醒 | `status/keep_awake.rs` 的 `KeepAwakeService` | `status keep-awake run / status / stop` |

桌面端在进程内调用这些 core 函数。

## 新的 core 能力：MSI 卸载

- 输入只来自清单条目里 MSI 来源的产品代码。校验为花括号 GUID 格式后才进入计划。
- 执行：程序 `msiexec`（解析到系统目录的绝对路径），参数数组 `["/x", "{产品代码}", ...]`。静默与重启抑制参数在实施第一步调研后固定，写入 `research/msi.md`。不经过 shell，不拼接命令行。
- 需要提权的包：不自行提权。预览时能判断就标为"需要提权"，执行时如实报告安装程序的退出码。
- 资格：清单条目的 `eligibility` 增加相应取值，使 MSI 条目从"manual"变为可计划。ARP 条目不变。
- 结果按现有卸载操作的形状写入历史，这样残留计划能认到它。
- 先读 `.trellis/spec/backend/provider-authority.md`；这项能力要符合其中的授权依据规则，并在完成后更新该文档。

## GPUI

- **执行器**：`software_review.rs` 现有 `UninstallExecutor`（`:34`）抽象和环境变量测试钩子（`:28-31`）。生产路径接到 core 的真实执行，测试继续用隔离执行器。把"isolated_executor_unavailable"的拒绝分支（`:604-720`）改为真实分派；`preview_host_startup`（`:800-818`）的第二次激活改为执行。
- **统一的确认流程**：一个共用的审阅组件，列出将要发生的事、摘要的简短形式、确认按钮。各动作只提供自己的条目和文案。使用 airy-glass-shell 的固定动作栏。
- **摘要失效**：选择变化时立即失效。执行前 core 再校验一次实时状态，不一致则拒绝并提示重新预览。
- **长时间动作**：卸载与更新在后台任务里运行，界面显示当前项，不估算时间。运行期间禁用同域的其他主机动作。
- **残留**：卸载成功后在结果页提供入口。没有成功记录时入口不出现。
- **结束进程**：进程行的动作菜单。受保护进程显示原因。确认框写明进程名和 PID。
- **保持唤醒**：`Session` 持有一个 `KeepAwakeService` 句柄；开关放在 Status 首行。应用退出路径释放。先确认它与 CLI 的 `keep-awake run` 是否共享状态，以及两者同时使用时的行为。
- **提权**：检测到需要提权时显示原因和一个"复制等价 CLI 命令"按钮。
- **文案**：删除"Host uninstall is refused."（`copy.rs:1590`）等只读立场的句子，改为各动作的审阅与结果文案，中英文。

## 兼容与回滚

- 每个动作单独接入，可以单独撤回到只读状态。
- MSI 卸载是 core 与 CLI 的新增能力，撤回 GPUI 接入不影响它。
- AI 助手的 inspect 流程规则不变：它仍然不运行 `optimize run` 和 `software uninstall`。

## 风险

- 这是桌面端第一次能改变主机状态。每个动作都要有"未确认不执行"和"摘要过期被拒"的测试，评审时逐个核对。
- `msiexec` 的行为因包而异（需要重启、弹出自有界面、返回 3010）。退出码如实映射为成功、需要重启或失败，不把未知码当成功。
- 结束进程存在 PID 复用。沿用 core 绑定计划的做法（`bind_process_termination_plan`），执行前校验进程身份。
- winget 不存在或版本过旧时，更新动作不可用并说明原因。
