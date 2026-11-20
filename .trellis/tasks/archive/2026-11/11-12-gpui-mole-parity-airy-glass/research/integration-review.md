# 集成验收记录

本次集成使用独立工作树 `D:/ds-integrate-1120`，分支 `integration/mole-parity-1120`，合并输入为本地 `264e7ac` 与远端 `324e72b`。原工作树和原始材质签收证据保留。以下为 2026-11-20 完成集成后的最终记录。

| 父任务验收项 | 当前证据 |
| --- | --- |
| AC1 子任务归档 | 已从归档元数据核对 R1–R9 九项均为 completed，活动目录无同名子任务。各自的证据见父任务[结果索引](../result.md)。 |
| AC2 共享 core | 下表将 Clean、Software、Status、Analyze、Optimize 的所有可变更动作与 CLI 和共享 core 实现逐项对应。每类均有命令契约或同一夹具端到端复验。 |
| AC3 GPUI 进程派生 | 已逐项核对 [GPUI 源码检索](desktop-process-scan.txt) 与相对本地基线的新增调用。生产 Analyze 的既有 Explorer 入口使用分离参数。新增 ARP 点击及辅助功能激活均调用固定的 ms-settings:appsfeatures。其余命令构造位于既有封装和测试/夹具。桌面不派生 DevSweep CLI。 |
| AC4 设计令牌 | R5 集成时的 ui.rs、Clean 页面、工作台与磁盘图均与远端材质签收版本一致。从 PR #7 原生证据复用 check_tokens.py，对本工作树核对 mint 调色板与 DESIGN.md 的 56 个值，差异 0。ui.rs 的 SHA-256 为 0CE68AFC5F923883D0DA7F20FF2E0517E4E119F543D49369BB1625DA2AE341F1。桌面源码守卫另核对字号、间距、圆角与对比度。 |
| AC5 CLI V1 JSON | 旧 Analyze、Software、Optimize V1 文档和旧偏好均有兼容测试。CLI 单项维护继续生成和读取 V1，多项计划显式使用 V2。history totals 为新增命令。 |
| AC6 安全契约 | 当前 Clean 的 Command、InspectOnly 均无法进入回收站执行。新家族所有者或登记不明时停止生成候选。批次计划复验身份及选择，文件只走回收站执行器。固定程序和分离 argv、Cargo home 与 Docker 边界另由根门禁覆盖。 |
| AC7 集成门禁 | 最终同一版本依次执行 just ci、just gpui 和 Python 工具测试。根工作区 943 项通过、3 项忽略；桌面主程序 262 项及画廊 4 项通过、2 项原生收据用例默认忽略并另行启用。release 构建 SHA-256 为 FE2F166AB2D3AE12E8D2A59D3536BF599373C40DD04B60B817648F7F3FA9F5EC。Python 23 项通过。完整日志及初轮失败见[门禁记录](integration-gates.zip)。 |
| AC8 原生矩阵 | R5 的本次隔离桌面验证覆盖两个语言、六种状态、真实 UIA 和按操作核对的历史。五个模式和托盘四组配色已取得 24 张原图，见下方总览与逐项索引。 |

## 决定

| 项目 | 用户确认 |
| --- | --- |
| R4 | 先接入当前用户 MSIX；修正的七项桌面用例与一项 core 审计恢复用例通过后实施步骤 11–15。验证使用任务夹具。 |
| R6 / D4 | Chrome Beta、Chrome Canary、Edge Beta、Edge Dev。仅已登记配置文件的 HTTP Cache、Code Cache。 |
| R7 | 最大文件仅代表保留的快照，显示证据的完整性。卷容量面向当前用户；鼠标和键盘共用窗口内菜单。 |
| R8 / D6 | 默认全不勾，用户自选并确认。 |
| R9 / D5 | 默认关闭即退出，设置可选驻留托盘。用户接受向任务窗口投递 TaskbarCreated 并核对实际重新注册，作为 R9 AC6 的验收口径。 |

## 差距逐项对照

| 编号 | 归属 | 当前依据或处置 |
| --- | --- | --- |
| S1 | R1 | 已解决。已归档。页面首行显示状态、数字与操作。 |
| S2 | R1 | 已解决。已归档。排版、间距和圆角令牌及源码守卫在同一桌面工作区。 |
| S3 | R1 / R5 | 已解决。已归档外壳及独立签收的 Clean 新材料；已整合并复验正常工作流。 |
| S4 | R1 / R5 | 已解决。外壳最初验证透明效果，PR #7 改为窗口内绘制底场。以最终 DESIGN.md 及原生结果为准。 |
| S5 | R1 | 已解决。已归档。720 逻辑像素起保留标签文字，材质验收另有 720 截图。 |
| S6 | R1 | 已解决。已归档。页面占满可用空间，内容区内滚动。 |
| S7 | R1 | 已解决。已归档。System 偏好映射到实际窗口外观。历史签收中系统设置热切换的限制保留。 |
| S8 | R1 / R5 | 已解决。已归档。按下控件用填充反馈，Clean 材质验收覆盖悬停、禁用与向前 Tab。 |
| S9 | R1 / R3 / R7 | 已解决。软件来源和路径已统一。Analyze 的中文页面、未知与下界数值经独立复核和双语回归。 |
| S10 | R1 | 已解决。已归档。桌面设计契约位于 desktop-gpui/DESIGN.md。 |
| C1 | R5 | 已解决。全范围证据与全局授权隔离见 R5。 |
| C2 | R5 | 已解决。分组小计、三态与仅供审阅行见 R5。 |
| C3 | R5 | 已解决。隔离夹具单次执行对应完成页与历史，见 R5。 |
| C4 | R6 | 已解决。四个预发布通道只取已登记且可验证的缓存目录。占用、状态未知和损坏配置的拒绝均已验证。 |
| C5 | 有意差异 | Downloads 安装包与清空回收站不在本轮授权范围。 |
| W1 | R3 | 已解决。已归档。安全取得时用软件图标，WindowsApps 不可安全读取时保留回退头像。 |
| W2 | R3 | 已解决。已归档。支持排序与可选安装日期。 |
| W3 | R4 | 已解决。MSIX、合格的当前用户 MSI、更新、启动项及具备卸载收据的文件残留均经二次确认。ARP 打开固定的系统设置页面。七项主机动作有双语审阅/结果原图。 |
| O1 | R8 | 已解决。有序批次使用一个实时摘要，逐项复检。双语夹具验证失败后继续、跳过原因及成功项，并与 CLI 历史一一核对。 |
| O2 | R8 | 已解决。七项执行项初始全不勾选，十一项指引/设置位于折叠区域，只读观察保留在目录状态。四组普通窗口均显示默认零勾选。 |
| A1 | R7 | 已解决。analyze largest 对保留的快照做有界投影，说明完整性、排序、大小阈值和容量证据。 |
| A2 | R7 | 已解决。宽窗口列表与树图并排，窄窗口堆叠，同一节点在两个视图中同时高亮。 |
| A3 | R7 | 已解决。卷卡显示当前用户口径的可用量和总量。读取成功的真实零、部分读数和缺失路径分别保留。 |
| A4 | R7 | 已解决。三个节点保存到同一计划，改选撤回旧摘要；实测右键、菜单键与 Shift+F10 打开同一菜单，Esc 恢复原行。 |
| A5 | 延后 | 与上次扫描比较需要稳定的基线和归属契约。 |
| T1 | R2 | 已解决。已归档。Status 进入即采样。 |
| T2 | R2 | 已解决。已归档。以采样节拍绘制历史趋势。 |
| T3 | R2 | 已解决。已归档。硬件、系统、磁盘等证据按可得性呈现，未知与零分别处理。 |
| T4 | R2 | 已解决。已归档。展示健康证据，保留不打分的产品决定。 |
| T5 | R4 | 已解决。保持唤醒及受保护进程检查具备 CLI 同源契约与双语审阅/结果图。托盘按钮也经 UIA 验证只进入待确认页。 |
| T6 | R9 | 已解决。Status 与托盘同时可见时只启动一次订阅。全部隐藏后停采，暂停主页面趋势时面板可继续接收同一采样。 |
| T7 | R9 | 已解决。两种语言、两套主题均获得托盘面板与原生双项菜单图。累计数据按丢失或异常证据标下界。 |
| T8 | R9 | 已解决。默认关窗退出；另三组保存驻留偏好后关窗、重开，并通过原生菜单退出。按批准的模拟重建消息验证图标重新注册。 |

## 原生截图索引

[五个模式与托盘的 24 张原生图总览](native/matrix.jpg)

本次截图由任务窗口在独立的非输入 Windows 桌面绘制，以 PrintWindow 收取窗口输出。常规主窗口使用 125% DPI。软件模式经正常导航读取三条隔离软件记录。清理模式四种组合来自真实夹具项目的审阅。分析模式三项为 120、60、30 MiB 的任务文件。

| 模式 | EN 浅色 | EN 深色 | 中文浅色 | 中文深色 |
| --- | --- | --- | --- | --- |
| Clean | [图](../../11-12-gpui-clean-review-flow/evidence/integration-1120/en-review.png) | [图](../../11-12-gpui-clean-review-flow/evidence/integration-1120/en-dark-review.png) | [图](../../11-12-gpui-clean-review-flow/evidence/integration-1120/zh-light-review.png) | [图](../../11-12-gpui-clean-review-flow/evidence/integration-1120/zh-review.png) |
| Software | [图](native/software-en-light.png) | [图](native/software-en-dark.png) | [图](native/software-zh-CN-light.png) | [图](native/software-zh-CN-dark.png) |
| Optimize | [图](../../11-12-gpui-optimize-batch/evidence/native/en-light-ordinary.png) | [图](../../11-12-gpui-optimize-batch/evidence/native/en-dark-ordinary.png) | [图](../../11-12-gpui-optimize-batch/evidence/native/zh-light-ordinary.png) | [图](../../11-12-gpui-optimize-batch/evidence/native/zh-dark-ordinary.png) |
| Analyze | [图](../../11-12-gpui-analyze-insights/evidence/native/en-light-wide.png) | [图](../../11-12-gpui-analyze-insights/evidence/native/en-dark-wide.png) | [图](../../11-12-gpui-analyze-insights/evidence/native/zh-light-wide.png) | [窄窗](../../11-12-gpui-analyze-insights/evidence/native/zh-dark-narrow.png) |
| Status | [图](../../11-12-gpui-tray-panel/evidence/en-light-status.png) | [图](../../11-12-gpui-tray-panel/evidence/en-dark-status.png) | [图](../../11-12-gpui-tray-panel/evidence/zh-light-status.png) | [图](../../11-12-gpui-tray-panel/evidence/zh-dark-status.png) |
| 托盘 | [图](../../11-12-gpui-tray-panel/evidence/en-light-panel.png) | [图](../../11-12-gpui-tray-panel/evidence/en-dark-panel.png) | [图](../../11-12-gpui-tray-panel/evidence/zh-light-panel.png) | [图](../../11-12-gpui-tray-panel/evidence/zh-dark-panel.png) |

## 桌面动作与命令行的对照

| 动作 | CLI 与共享执行路径 | 覆盖 |
| --- | --- | --- |
| Clean | clean plan、preview、execute。桌面调用同一计划构造、实时预览与回收站执行器 | R5 正常桌面流程及同一次操作的 CLI history show。Command、InspectOnly 排除在桌面回收站计划之外。 |
| MSIX、MSI | software plan、preview、uninstall 进入同一 SoftwareExecutor。当前用户身份、活资格和摘要重验后才调适配器 | cli_contract 的旧清单及 MSI 兼容、software/execution 的命令处理器测试、R4 的独立检查和两种语言原生审阅。 |
| 更新及启动项 | software updates / startup 的 plan、preview、execute 使用 core 的 software::updates 与 software::startup | c05_software_maintenance 的共享服务输出及更新拒绝/重启证据，c01_shipped_parity 的启动项预览。 |
| 成功卸载后的文件残留 | software leftovers scan、plan、preview、execute 在 software::leftovers 中共同核对成功记录、候选目录身份与摘要 | c01_shipped_parity 的正向和拒绝预览、R4 成功和失败夹具，以及防止旧审阅重复提交的桌面用例。 |
| 当前进程、保持唤醒 | status process plan、preview、terminate 调用同一进程动作服务。status keep-awake run、status、stop 使用同一 owner/endpoint 协议 | status 命令处理器核对 PID 身份、受保护目标、过期摘要、确认与双语 JSON。keep_awake_contract 核对另一端的 CLI 状态、停止及已关闭端点。 |
| Analyze | analyze trash-plan、trash-preview、trash-execute 使用 core 的 plan_analyze_trash、preview_analyze_trash_plan、execute_analyze_trash_plan。Reveal 解析已验证的快照节点并交给系统文件管理器 | c01_shipped_parity 的预览、无确认拒绝及安全输出；GPUI 原生三节点预览后改选立即撤回确认。 |
| Optimize | optimize plan、preview、run 对多项计划调用 MaintenanceExecutor 的 preview_batch 与 execute_batch。设置移交和指引沿用原单项入口 | 缺少实时摘要或确认时拒绝，批次内每项重新验证。两次三项夹具的 UIA、CLI history list 和 history totals 同源，比对失败、跳过、成功与唯一批次 ID。 |
| ARP | 固定交给 Windows 已安装的应用设置页，程序不构造厂商卸载命令。管理员提示只能复制等价计划命令 | R4 的固定 URI 与手动处理分支检查。 |

## 首轮与复跑记录

[集成门禁日志](integration-gates.zip)保留首轮 Rust/Python 结果、定向失败、复跑、R6 损坏登记的红绿对照和调色板核对。初次 Python 共 23 项，其中 hooks.local.json 忽略规则失败。修复项目忽略配置及其回归后，同一命令 23 项通过。根工作区在 R8、R9 写入期间的三次增量门禁分别发现双语命令参考清单与 Clap 不一致、未格式化代码，以及测试内的 Cargo 夹具构建读到未完成的模块。清单已由 CLI 契约复验，缓存任务的首个完整集成根门禁通过 940 项、3 项忽略，Clippy 通过。


R8 最后再次用新审计格式从桌面执行两份隔离批次。CLI 凭批次 ID 各读取九条事件、三种终态与一次成功，旧记录缺少预期项数时返回原 ID 并标为未知。见[R8 原生复验](../../11-12-gpui-optimize-batch/evidence/native/acceptance.md)。

归档包保留各轮门禁和定向测试日志、私有桌面探针以及逐个窗口的输入记录。子任务阶段记录中的 dist/verify-1120 日志文件可在本报告的门禁归档包中按原文件名找到。
