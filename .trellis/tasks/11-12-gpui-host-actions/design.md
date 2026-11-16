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
- 执行：系统 API 解析绝对 `msiexec.exe` 并绑定同一实际文件身份/哈希；参数数组固定 `["/x", "{产品代码}", "/qn", "/norestart"]`，GUID 严格校验。不经过 shell，不拼接命令行，不回退 PATH、交互模式或提权。
- 已确认资格边界：仅开放正向证据完整的当前用户非托管、已安装且上下文唯一的 MSI。已知需管理员、机器级、托管、其他用户、多上下文或身份/限制证据不完整者拒绝。明确正向证据来源及 typed SID/context 实时重查后再实现；不以默认 false、显示元数据或夹具资格代替证明。
- R11 保证 DevSweep 自身不请求提权并关闭 Installer 内部 UI，不承诺任意第三方自定义动作绝不出现外部 UAC；原始研究的绝对保证/全部拒绝候选不是已批准方案。
- 资格：清单条目的 `eligibility` 增加相应取值，使 MSI 条目从"manual"变为可计划。ARP 条目不变。
- 结果按现有卸载操作的形状写入历史，这样残留计划能认到它。
- MSI 单独分类与验证审计重放：只有 `0 + 同一 SID/context 权威消失` 可记 `Removed` 并获得残留权限。`3010` 保留需重启，`1641` 保留已触发重启的区别，二者均不立即开放残留；失败/超时/丢失状态/派发后取消不因登记消失而变为成功，不自动重新派发。同步 V1 可选证据并保持旧计划/ID/历史和 MSIX 解释兼容。
- 先读 `.trellis/spec/backend/provider-authority.md`；这项能力要符合其中的授权依据规则，并在完成后更新该文档。

## GPUI

- **执行器**：`software_review.rs` 现有 `UninstallExecutor`（`:34`）抽象和环境变量测试钩子（`:28-31`）。生产路径接到 core 的真实执行，测试继续用隔离执行器。把"isolated_executor_unavailable"的拒绝分支（`:604-720`）改为真实分派；`preview_host_startup`（`:800-818`）的第二次激活改为执行。
- **统一的确认流程**：一个共用的审阅组件，列出将要发生的事、摘要的简短形式、确认按钮。各动作只提供自己的条目和文案。使用 airy-glass-shell 的固定动作栏。
- **摘要失效**：选择变化时立即失效。执行前 core 再校验一次实时状态，不一致则拒绝并提示重新预览。
- **长时间动作**：卸载与更新在后台任务里运行，界面显示当前项，不估算时间。运行期间禁用同域的其他主机动作。
- **残留**：卸载成功后在结果页提供入口。没有成功记录时入口不出现。
- **结束进程（步骤 9 已批准）**：进程行进入共用审阅。采样行继续使用现有 `ProcessV1`，后台新查询身份后绑定现有 `ProcessTerminationPlanV1` 并实时预览，不给采样 DTO 增加执行权限字段。确认框显示该实际身份的进程名、PID 与未保存数据风险；UI 使用 core 的保护结果并显示原因，core 在派发前及实际终止句柄处复核身份。仅明确的 Confirm 控件可以确认，根区域/搜索框的全局 Enter 不派发进程动作，行内 Review 始终只准备审阅。选择/导航/身份变化使摘要失效，执行期间持有后台任务锁，过期投递不能替换新审阅。只用 fake probe/killer 与任务拥有的进程夹具验证，不结束本机用户进程。
- **进程历史（R12）**：GUI 接入前，先让 core 与 CLI 通过共用的 Status 动作审计路径记录进程专属动作与身份，不把它写成保持唤醒事件。保留旧 V1 计划与旧 keep-awake 历史兼容；Started 持久化失败不调用 killer，派发后终态持久化失败或历史缺少终态时保留未知，不自动重新结束进程。历史不是进程当前存活状态证明。
- **保持唤醒**：把现有显式 endpoint/token/owner 会话协议提到 core 供 CLI/GPUI 复用。`Session` 拥有一个专用 worker，在同一线程获取、刷新和清除 OS 请求，退出时清除并 join；开关在 Status 首行。应用活着时保留停止后的可查询端点；CLI 查询旧的已关闭端点报告 unavailable/unknown。区分独立 owner，真实报告 OS 获取/清除失败。不开新守护进程、不持久化生命周期收据、不派生 CLI。
- **保持唤醒授权**：core 提供共用的有限租期请求计划、实时 `sha256:` 预览与执行校验，GUI 保留不可变计划和摘要，第二次确认后显式传给 core；不能只在 GUI 自签摘要后调用 bool 开关。现有 CLI `run --confirm` 语法保持，其便利入口内部使用同一请求校验链。不新增磁盘计划格式。
- **更新的 core 补齐**：现有程序名字符串指纹不能作为实际工具身份。步骤 8 同步补实际文件/版本与当前用户范围绑定、即时重校验、派发前和终态的持久化审计。继续使用已计划的精确包 ID/版本/来源；只提供派发前取消，不承诺中止 Installer 或回滚，执行域锁持有到真实终态。
- **结果历史**：启动项沿用现有 Software support 审计，更新补齐相同的脱敏生命周期/终态；保持唤醒增加独立 Status action 审计并让 History/CLI 识别。审计用于记录操作，不是 owner 生命周期收据，不用于把端点不可达推成 off。开始审计失败不得执行，派发后终态记录失败如实报告不确定；不记录 token、endpoint、argv 或环境。
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
