# 对标分析：Mole for Windows 与 DevSweep GPUI

基线：`dev` 分支 `1c66830`，工作树干净。日期 2026-11-12。

## 1. 证据边界

- **Mole for Windows 0.22.0** 安装在 `%LOCALAPPDATA%\Tw93.Mole`（WinUI 3 + .NET，含 `Mole.Core.dll`、`Mole.Helper.exe`），源码不公开。本轮四次检查进程列表都没有运行中的 Mole 进程，所以没有操作本机实例。界面依据是 mole.fit 上同版本的六张 Windows 截图（Clean、Apps、Optimize、Analyze、Status、托盘面板）和站点文档。站点文档由抓取代理转述，其中尾部和旧版发布说明只是摘要，没有用作依据。
- **ref/Mole**：`main` 的 `SYNC.md` 全文；`origin/windows`（`20f11fb1`，2026-11-12）的文件清单、README、清理与优化函数清单。没有逐行读 PowerShell 实现。
- **DevSweep**：`cli.rs` 的命令定义、core 各模块的公开接口、`desktop-gpui` 的 `ui.rs` 全文、`view.rs` 主渲染段和各页面函数清单；2026-11-03 至 11-06 归档任务里的原生截图。本轮没有构建或运行 GPUI，也没有运行 CLI（PATH 上没有 `devsweep`）。`shell.rs`（5388 行）、`clean_workbench.rs`、`software_review.rs` 只做了定点检索。
- 两个盘点子代理因限流失败，盘点改为定点阅读，覆盖面比原计划窄。标"推断"的条目只有代码阅读依据，没有运行时证据。标"待核对"的条目要在对应子任务的第一步确认。

## 2. Mole for Windows 的结构

| 标签页 | 截图中的结构 | 能力要点 |
| --- | --- | --- |
| 外壳 | 顶部居中一条胶囊标签（Clean / Apps / Optimize / Analyze / Status，纯文字），左侧圆形 logo 菜单；没有侧栏和页面标题；每个标签页一种底色；仅深色 | Ctrl+1–5 切换；标题栏一枚状态胶囊 |
| Clean | 行星、一个大数字（"90 GB found"）、一行次要信息（"78 GB cleanable · Scan again"）、一个主按钮（"Review results"） | 扫描 → 分组勾选审阅（带页脚）→ 完成页，摘要可展开明细；"单独管理"的行不计入合计 |
| Apps | 分段 Uninstall / Updates / Startup；右侧排序（Name / App size / Installed）、筛选、搜索；行 = 真实图标 + 名称 + 版本 · 容量 + 复选框 | 调用应用自带卸载器，再把残留移入回收站并备份注册表；winget 更新；启动项开关 |
| Optimize | 行星、两行说明、一个主按钮 | 一键执行，随后"Optimize details"列出已执行与已跳过的任务及原因；不改注册表、不停服务 |
| Analyze | 左侧当前目录列表（名称、容量），右侧树图；上方面包屑和卷容量条；底部提示右键操作 | 逐层下钻、多选、最大文件（100 MB 以上）、相对上次扫描的增长、右键"打开 / 回收站" |
| Status | 4×2 指标格（Health、CPU、GPU、Memory、Battery、Free space、Network、Disk activity），每格一个大数字、一条趋势线或条形、一行说明；下方进程表（名称、内存、CPU、PID、行菜单） | 进入即实时；60 秒趋势；进程可固定、解释、结束 |
| 托盘 | 浮动面板：健康读数与机器标签、六个指标小格、电池、前五进程、Awake / Screen 开关、累计 Cleaned / Uninstalled / Optimized | 与主窗口共享数据 |

## 3. DevSweep 现状

### 3.1 CLI 命令树（`crates/devsweep-cli/src/application/cli.rs`）

- `clean`：`scan`（`--root` 可重复、`--scope projects|global|all`、`--rescan-target`）、`plan`、`preview`、`execute`（`--plan --preview-digest --confirm`）、`protect {list,add,remove}`、`rules {list,show}`、`ai {scan,plan,preview,execute,recovery-list,restore-plan}`。
- `software`：`inventory`（`--source all|arp|msi|msix`）、`plan`、`preview`、`uninstall`；`updates {check,plan,preview,execute}`；`startup {list,plan,preview,execute,hive-*,restore-*}`；`leftovers {scan,plan,preview,execute,registry-*}`。
- `optimize`：`list`、`plan --operation`（单个）、`preview`、`run`、`health`。
- `analyze`：`scan`、`roots`、`list`、`explain`、`reveal`、`trash-plan`（`--select` 可多个）、`trash-preview`、`trash-execute`。
- `status`：`snapshot`、`live`（`--interval` 默认 2 秒，NDJSON）、`health`、`process {plan,preview,terminate}`、`keep-awake {run,status,stop}`。
- `history`：`list`、`show`。
- 全局 `--locale en|zh-cn`；快照类命令 `--format human|json`、`--output FILE`。

### 3.2 GPUI 与后端的关系

`desktop-gpui/Cargo.toml:16` 以路径依赖引用 `devsweep-core`，全部在进程内调用；唯一的外部进程是 `explorer.exe`（`analyze_window.rs:242`）。`process_spawn.rs` 记录了从无控制台的窗口程序启动控制台程序会触发终端交接失败（`0x800700E8`），所以桌面端不适合靠派生 `devsweep.exe` 取数据。

### 3.3 CLI 已有、GPUI 没有接上的能力

| 能力 | CLI / core | GPUI |
| --- | --- | --- |
| 主机卸载（当前用户 MSIX） | `software uninstall`；`software/execution/mod.rs` | 拒绝："Host uninstall is refused."（`copy.rs:1590`，`software_review.rs:604-720`） |
| 安装更新 | `software updates execute` | 只检查（`page_workbench.rs:455`，`workbench_review.rs:166`） |
| 启动项开关 | `software startup execute` | 只预览，第二次激活被拒（`software_review.rs:800-818`） |
| 卸载残留 | `software leftovers` 十个子命令 | 不可用（`copy.rs:281`） |
| 结束进程 | `status process terminate`；`status/process_action.rs` | 只读，文案明确不结束进程 |
| 保持唤醒 | `status keep-awake` | 没有引用 `KeepAwakeService` |
| 累计清理量 | core `history::clean_moved_totals()`（`history/mod.rs:301`），CLI 无子命令 | 未使用（Tauri 版已使用） |

## 4. 差距矩阵

"归属"是负责该差距的子任务目录后缀。

### 外壳与设计

| 编号 | 差距与证据 | 归属 |
| --- | --- | --- |
| S1 | 标签已经命名了页面，页内又放图标块、24px 标题和副标题（`view.rs:1358-1404`）。首屏第一眼不是状态、数字或动作。 | airy-glass-shell |
| S2 | `ui.rs`、`view.rs`、`view/*.rs` 里有 19 种字号、13 种圆角、23 种间距取值，没有刻度。 | airy-glass-shell |
| S3 | 场 → 纸（1px 亮边）→ 卡片（1px 描边，`ui.rs:456-465`）→ 行，三层轮廓嵌套。11-06 审计的第 9 门已记为待改进。 | airy-glass-shell |
| S4 | 开启原生模糊时根渐变只有 77% 不透明（`view.rs:712-714`），模糊着色是透明黑（`vendor/gpui_windows/src/window.rs:936`）。浅色主题的场色被桌面冲成灰紫（`after-light-landing.png`），薄荷到淡紫辨认不出。 | airy-glass-shell |
| S5 | 宽度小于 69rem 时只有选中的标签显示文字（`view.rs:627`、`:935`）。归档截图里 1025px 宽的窗口有四个无文字图标标签。 | airy-glass-shell |
| S6 | 工作面随内容定高（`view.rs:736-751`），短页面下方是大片空场（`after-dark-status.png`）。 | airy-glass-shell |
| S7 | 设置只有深色、浅色两项（`page_support.rs:447-470`）。偏好里的 `system` 恒映射为深色（`ui.rs:51-63`），不跟随 Windows。 | airy-glass-shell |
| S8 | 按钮和分段的按压态是给文字加下划线（`ui.rs:429`、`:790`）。 | airy-glass-shell |
| S9 | 路径带 `\\?\` 前缀直接显示（`final-clean-results.jpg`，GPUI 没有剥离函数）；软件行副标题是内部来源 ID（`arp:CurrentUser:Registry64`）；中文界面出现英文诊断句（`final-software-zh-125-light.jpg`）；Analyze 位置卡盘符小写（`after-dark-analyze.png`）。 | airy-glass-shell |
| S10 | GPUI 的设计契约只在归档任务的 `research/` 里。`ui.rs` 的次要文字 `#505F68`、`#52616A` 已与条目的 `#5C6B76` 分叉（对比度更高：6.35、6.15 对 5.28），但没有记录。 | airy-glass-shell |

### Clean

| 编号 | 差距与证据 | 归属 |
| --- | --- | --- |
| C1 | 没有跨范围的合计数字。GPUI 分项目与全局两个范围分别扫描（`page_clean.rs:261`）；CLI 的 `--scope all` 已支持一次扫两类（`cli.rs:86`）。 | clean-review-flow |
| C2 | 审阅列表平铺（`page_clean.rs:734`、`:882`），分组只出现在磁盘图里（`page_clean.rs:552-600`）。没有组小计和按组勾选。 | clean-review-flow |
| C3 | 执行结果是一行结论（`view.rs:1612`）加结果卡（`page_clean.rs:446`）。是否有按类别的已移动、失败、跳过明细：待核对。 | clean-review-flow |
| C4 | Windows 缓存只有 6 个家族（`rules/definitions.rs:5-30`：Chrome、Edge、Firefox、VS Code、Cursor、用户临时文件）。Mole CLI（Windows）的 `caches.ps1`、`cache_guard.ps1` 有 25 个应用缓存档案，另有 Office、OneDrive、Adobe、游戏平台、Windows 日志、内存转储等。DevSweep 已明确排除错误报告、Windows Update 下载、传递优化、Downloads、着色器缓存（`definitions.rs:33-52`）。 | clean-windows-cache-coverage |
| C5 | Mole 清理两周以上的安装包和回收站。DevSweep core 没有对应规则，且把 Downloads 定为个人数据。 | 有意差异 |

### Software

| 编号 | 差距与证据 | 归属 |
| --- | --- | --- |
| W1 | 行首是字母头像，没有真实图标（`final-software-list.jpg`）。`SoftwareEntryV1` 没有图标和安装日期证据（`software/model.rs:247`）。 | software-evidence |
| W2 | 排序只有名称和容量（`page_workbench.rs:126`），没有安装日期。 | software-evidence |
| W3 | 卸载、更新、启动项、残留在本机都不能执行（见 3.3）。ARP 与 MSI 条目是"manual"；core 明确不把厂商卸载命令当作授权依据。 | host-actions |

### Optimize

| 编号 | 差距与证据 | 归属 |
| --- | --- | --- |
| O1 | 一份计划只含一个操作（`optimize/plan.rs:156-165`，`cli.rs:726-734`）。没有"一次审阅、批量运行、逐项结果"。 | optimize-batch |
| O2 | 19 项目录里只有 7 项可执行，3 项是打开系统设置，8 项是指引，1 项是只读观察（`optimize/catalogue.rs:97-188`）。它们混排在同一列表里，页面读起来像文档。 | optimize-batch |

### Analyze

| 编号 | 差距与证据 | 归属 |
| --- | --- | --- |
| A1 | 没有最大文件视图。core 的 analysis 模块没有相应投影，CLI 没有子命令。 | analyze-insights |
| A2 | 树图和列表上下堆叠，截图里树图只有当前目录一层（`redesign-2026-11-03-analyze-detail.png`）。 | analyze-insights |
| A3 | 位置卡没有容量。`AnalyzeRootRecordV1` 没有总量与可用量字段（`analysis/observe.rs:91-96`）。 | analyze-insights |
| A4 | CLI 的 `trash-plan --select` 支持多个节点（`cli.rs:823-825`），界面详情面板只针对一个选中节点。模型里有 `recycle_ids()`（`analyze.rs:81`）：待核对。没有右键菜单。 | analyze-insights |
| A5 | 相对上次扫描的增长。Mole CLI 自己把它列为有意差异（`ref/Mole/SYNC.md`：需要基线和稳定的归属契约）。 | 本轮不做 |

### Status 与托盘

| 编号 | 差距与证据 | 归属 |
| --- | --- | --- |
| T1 | 进入页面不采样，要先点"Refresh snapshot"或"Start live"（`after-dark-status.png`，`shell.rs:2324`）。 | status-dashboard |
| T2 | 没有趋势。GPUI 没有采样环形缓冲（`shell.rs:362` 的 `VecDeque` 只是事件队列）。 | status-dashboard |
| T3 | `StatusSnapshotV1`（`status/mod.rs:195-208`）没有 CPU 型号与频率、开机时间、系统版本、页面文件、GPU 名称、电池健康与循环次数、系统级磁盘活动、网络类型。 | status-dashboard |
| T4 | 健康信息只是一枚徽标。core 的 `explain_health`（`status/health.rs:32`）给的是证据而不是分数，这是既定决定（CLI 测试 `status_health_names_evidence_without_an_opaque_score`）。 | status-dashboard |
| T5 | 进程不能结束，没有保持唤醒（见 3.3）。 | host-actions |
| T6 | 推断：`main.rs:62` 启动时就把托盘标为可见，`StatusPanel::reconcile`（`status_panel.rs:55-66`）按"主面板或托盘可见"判断。托盘并没有面板，所以实时采样一旦开启，离开 Status 页也不会停。 | tray-panel |
| T7 | 托盘只有图标和提示文字：`notify_data` 只设 `NIF_ICON | NIF_TIP`，没有回调消息（`tray.rs:229-252`），也没有菜单。点击没有反应。归档需求 C13-R3 要求一个可选的精简状态面板。Tauri 版已有托盘菜单和 HUD。 | tray-panel |
| T8 | 关闭主窗口即退出（`view.rs:2379`）。 | tray-panel |

## 5. 设计诊断（design-atlas Restyle 的八个维度）

`slop_check.mjs` 对 Rust 源码不产生结果（上次审计为 `files: 0`，本轮无输出），不能当作通过证据。下面是人工判读，属于自检，不是独立复核。

| 维度 | 证据 | 保留 | 改动 |
| --- | --- | --- | --- |
| 字号 | 19 种；页面标题 24px，根字号 15px，说明 11.5–14px；等宽数字只用在指标卡 | rem 随文字缩放；用户选的已安装字体 | 收敛为 32 / 16 / 14 / 12 四档 |
| 留白 | 23 种间距；纸 24px + 卡片 20px + 行 14px 三层内边距；短页面下方空场 | 列表行的紧凑密度 | 只用 8 / 12 / 16 / 24 / 32 / 48；纸占满高度 |
| 层级 | 首屏先看到图标块和标题；顶栏三组胶囊加页内两排按钮互相竞争；三层描边 | 一页一个主动作的原则 | 首行改为状态、数字、动作；内容区用填充不用描边 |
| 颜色 | 绿色承担主动作、选中、焦点；紫色几乎没用上；每行左侧的彩色竖条没有图例 | 警告与危险的独立语义；七个分类色用于树图 | 绿 = 输入与完成，紫 = 结果数字；去掉无图例的竖条 |
| 动效 | 没有动效；进度是静态条纹 | 不请求装饰帧；尊重减少动态效果 | 趋势线按采样节拍重绘，属于数据不属于装饰 |
| 微交互 | 悬停换底色和边框色；按压加下划线；键盘焦点 3px 内描边 | 内描边焦点环（透明标签上外阴影会糊住图标） | 按压改为填充加深 |
| 响应 | 小于 69rem 标签变图标；小于 960px 关闭模糊；小于 36rem 再压缩 | 960px 的实色回退 | 标签在 720px 以上始终带文字 |
| 独特性 | 每页相同的"图标块 + 标题 + 副标题"；等大的描边指标卡；到处是胶囊徽标 | 河狸标识；"下界""未知就是未知"的诚实容量措辞 | 去掉页面标题块；徽标只用于状态 |

设计来源：仓库根的 `design.md` 是历史产品架构文档（Windows 不区分大小写，所以它也以 `DESIGN.md` 出现），`desktop/DESIGN.md` 是 Tauri 版 Clean 的契约。GPUI 没有树内的设计契约。本任务用 `apply style:airy-glass` 生成新文件并写入 GPUI 适配，见 `research/DESIGN.md`；实施第一步把它放到 `desktop-gpui/DESIGN.md`。

## 6. 不照搬 Mole 的地方

| 项 | 依据 |
| --- | --- |
| 健康分数 | DevSweep 只给证据不打分，有测试锁定。 |
| 行星、诗句和首屏大图 | 条目写明没有营销首屏；11-06 已按用户确认移除 Clean 的地球；不得复制 Mole 的素材。 |
| 仅深色 | 用户在 2026-11-04 确认深浅同等交付、首次启动为深色。 |
| 永久删除 | `AGENTS.md`：本构建禁用。Mole 默认永久删除缓存。 |
| 执行厂商卸载命令字符串 | core 不把注册表里的卸载命令当作授权依据。 |
| Docker 清理、Cargo home 清理 | `AGENTS.md` 的安全契约。 |
| 错误报告、Windows Update 下载、传递优化、Downloads | `rules/definitions.rs:33-52` 已逐项说明排除原因。 |
| 风扇控制、增长对比、MFT 扫描 | 需要驱动、基线或新的归属契约，本轮不做。 |

## 7. 需要用户决定的事项

| 编号 | 决定 | 建议 | 影响 |
| --- | --- | --- | --- |
| D1 | 桌面端是否获得主机执行权限（卸载、更新、启动项、残留、结束进程、保持唤醒） | 全部接入，沿用"保存计划 → 实时摘要 → 二次确认"。**用户 2026-11-12 已选定全部接入** | 决定 host-actions 子任务的范围 |
| D2 | 新安装的默认主题 | 维持 2026-11-04 的决定：深色 | 一个枚举默认值 |
| D3 | Status 进入即采样 | 是。只读、有界，离开即停 | 取消现在的"不问不采样" |
| D4 | Windows 缓存扩展到哪些家族 | 先做 Chromium 系浏览器和开发工具；通讯与办公应用缓存另议 | 在该子任务自己的规划关口决定 |
| D5 | 关闭主窗口时驻留托盘 | 默认仍退出，设置里提供开关 | 在 tray-panel 的规划关口决定 |
| D6 | Optimize 默认批次包含哪些操作 | 只含不需要提权且有时限的可执行项 | 在 optimize-batch 的规划关口决定 |
