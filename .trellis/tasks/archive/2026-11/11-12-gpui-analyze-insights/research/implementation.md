# R7 实施与验证

2026-11-20，Windows，集成工作树 `D:/ds-integrate-1120`。

## 交付

- 核心在 `analysis/largest.rs` 校验快照后读取保留节点，用容量为 K 的堆筛选文件。逻辑字节降序、完整路径升序、快照 ID 升序。默认阈值 100 MiB，默认显示 100 行，上限 1,000。未知大小不参加排名，已观测的零和下界保留各自证据。
- `analyze largest` 支持保存的快照、人类可读和 JSON 输出。包括保留范围、匹配总数、遍历完成状态、根证据和各文件证据。`analyze roots` 在 Windows 上读取调用者配额内的总量和可用量；容量探测失败仍可打开有效位置。V1 历史位置记录无需迁移。
- 桌面在扫描工作线程上生成相同的最大文件投影。位置卡显示大写盘符和容量。详情宽窗并排、窄窗堆叠。列表、树图、复选框及区域选择复用节点 ID。固定底栏显示合计、计划预览和显式确认，改选撤销已有计划和摘要。
- 快照保留的完整文件或目录可通过行、树图的菜单加入选择。打开菜单时通过 core 读取当前拒绝原因，生成计划仍通过正式入口。菜单用真实焦点句柄和 GPUI 键绑定处理 Menu、Shift+F10、Up/Down、Tab/Shift+Tab、Enter/Space 和 Esc。Esc 将焦点交还发起控件。解释信息按请求获取，中文错误按本地化消息展示。

## 已运行

| 验证 | 结果 |
| --- | --- |
| 核心 `cargo test -p devsweep-core analysis` | 47 项通过，含正常、配额端真实零、容量不明、旧记录反序列化 |
| 核心 `largest_excludes_unknown_sizes_and_keeps_observed_zero_and_lower_bounds` | 1 项通过，包含重复节点和非法上限 |
| CLI `cli_contract analyze_largest` | 1 项通过，包含中英文、JSON、同大小排序、阈值、上限、空结果、局部快照 |
| CLI `english_and_chinese_references_exactly_match_the_clap_manifest` | 1 项通过 |
| 桌面 `cargo test --locked --target-dir dist/goal-gpui-test analyze` | 12 项通过。三项选择精确绑定一份计划；同焦点点击、范围变化和勾选变化撤回摘要；无摘要确认被拒绝，夹具文件保留 |
| 普通桌面进程（独立 Windows 桌面） | 英文、中文、宽窄布局、最大文件、三项预览和改选完成。首次实测发现全局 Esc 抢走菜单，修复后右键→Esc→菜单键可重开原行菜单；独立复测 Shift+F10→Down→Down→Enter 生效 |

原生记录在本地 `dist/verify-1120/r7-{keys-fixed,shift-post,three-en,narrow-zh,wide-zh,wide-dark-en}/shots/`。用启动进程独立的 LOCALAPPDATA 和 APPDATA，通过保存的 `clean-scan-roots-v1.json` 提供三个同级文件（120、60、30 MiB）。接受根、点击行/树图与预览均走通常页面。首轮失败及复测的 `capture.txt` 记录输入顺序。私有桌面的跨进程 UIA 焦点属性始终为 false；Esc 后未再用鼠标，按菜单键在对应原行重开菜单，验证了焦点恢复。没有回收主机真实文件。

最后新增的生产键盘映射回归 `analyze_menu_shortcuts_win_over_shell_f10_and_escape` 随集成 `just gpui` 核验，由父任务统一登记最终 `just ci`、`just gpui` 和截图归档。
