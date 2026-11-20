# GPUI 对标 Mole：功能补全与轻盈毛玻璃重构

## 目标

让 GPUI 版 DevSweep 在界面和功能上达到用户点名参照的 Mole for Windows 的完成度，同时保留 DevSweep 自己的安全契约。功能缺口先在 core 和 CLI 补上，桌面端再调用。主题、样式和排版按 design-atlas 的 `style:airy-glass`（轻盈毛玻璃）重构。

本任务是父任务：持有需求来源、子任务地图、跨子任务验收和最终集成评审，自身不直接改产品代码。

## 背景

- 对标分析、差距矩阵（S1–S10、C1–C5、W1–W3、O1–O2、A1–A5、T1–T8）和证据边界见 `research/gap-analysis.md`。
- 设计契约见 `research/DESIGN.md`：`style:airy-glass`，颜色方案 `mint`（回退选择，与 `ui.rs` 现有浅色值一致），带 GPUI 原生适配。
- GPUI 以路径依赖在进程内调用 `devsweep-core`（`desktop-gpui/Cargo.toml:16`），不派生 `devsweep.exe`。
- 最大的功能差距是授权而不是能力：卸载、更新、启动项、残留、结束进程、保持唤醒在 CLI 里都有，GPUI 有意拒绝执行。

## 需求与子任务

| 编号 | 需求 | 子任务 | 优先级 | 差距 |
| --- | --- | --- | --- | --- |
| R1 | 设计契约、Token 刻度、材质、外壳排版、主题跟随系统、文案规范 | `11-12-gpui-airy-glass-shell` | P1 | S1–S10 |
| R2 | Status 指标补全（core、CLI）与进入即实时的仪表盘 | `11-12-gpui-status-dashboard` | P1 | T1–T4 |
| R3 | 软件清单的图标与安装日期证据（core、CLI），列表排序筛选 | `11-12-gpui-software-evidence` | P1 | W1、W2 |
| R4 | 桌面端接入主机动作（全部接入），并新增 MSI 卸载（core、CLI） | `11-12-gpui-host-actions` | P1 | W3、T5 |
| R5 | Clean 汇总数字、分组审阅、固定动作栏、完成页 | `11-12-gpui-clean-review-flow` | P1 | C1–C3 |
| R6 | Windows 缓存家族扩展（core、CLI） | `11-12-clean-windows-cache-coverage` | P2 | C4 |
| R7 | Analyze 最大文件与卷容量（core、CLI），分栏、多选、上下文菜单 | `11-12-gpui-analyze-insights` | P2 | A1–A4 |
| R8 | Optimize 多操作计划与逐项结果（core、CLI），批量界面 | `11-12-gpui-optimize-batch` | P2 | O1、O2 |
| R9 | 累计统计（CLI），托盘菜单与面板，关闭行为 | `11-12-gpui-tray-panel` | P2 | T6–T8 |

### 贯穿所有子任务的约束

- **CLI 先行**：每项新能力先有 core 函数、CLI 子命令、JSON 契约和 CLI 契约测试，桌面端再在进程内调用同一个 core 函数。每个子任务的 `design.md` 有"等价 CLI"表。
- **授权链不变**：保存的计划、实时 `sha256:` 预览摘要、`--confirm`（界面上是二次确认）。只进回收站，没有永久删除。
- **扫描与模型层只产生计划**。命令型动作的程序与参数分离，不拼接命令行字符串。
- **Cargo home 只读检查。Docker 清理不做。**
- **诚实的数字**：未知就写未知，下界写"至少"，不用 0 代替缺失的读数。不引入健康分数。
- **中英文同等交付**，深色与浅色同等交付。

## 跨子任务验收

- AC1：九个子任务各自的验收全部通过并归档。
- AC2：GPUI 里每个会改变主机状态的动作，都能在对应子任务的"等价 CLI"表里找到同一 core 路径的 CLI 命令，且该命令有契约测试。
- AC3：`desktop-gpui` 源码里没有新增的外部进程派生点（`explorer.exe` 除外）；有检索证据。
- AC4：`desktop-gpui/DESIGN.md` 与 `ui.rs` 的 Token 一致；源码守卫测试通过。
- AC5：CLI 的 JSON 新增字段保持 V1 增量，CLI 契约测试通过。（Tauri 桌面已于 2026-11-14 移除）
- AC6：`AGENTS.md` 的安全契约逐条复核，没有新增 `--execute`、永久删除、Docker 清理或 Cargo home 清理动作。
- AC7：`just ci` 与 `just gpui` 在集成后的分支上通过。
- AC8：五个模式加托盘面板的原生截图矩阵（英文与中文、浅色与深色），与 `research/gap-analysis.md` 第 4 节逐条对照，每个差距标为已解决、有意差异或延后。

## 不在范围

健康分数。行星、插画、诗句和首屏大图。仅深色。永久删除。执行注册表里的厂商卸载命令字符串。Docker 与 Cargo home 清理。错误报告、Windows Update 下载、传递优化、Downloads 的清理。风扇控制、扫描增长对比、MFT 扫描。TUI 的界面改版。复制 Mole 的素材、名称或文案。

## 决定

| 编号 | 决定 | 状态 |
| --- | --- | --- |
| D1 | 桌面端获得主机执行权限，范围为全部接入：MSIX 与 MSI 卸载、winget 更新、启动项开关、卸载后的残留、结束进程、保持唤醒。ARP 条目交给 Windows 设置，不执行厂商卸载命令 | 用户 2026-11-12 选定 |
| D2 | 新安装默认主题为深色 | 沿用用户 2026-11-04 的决定 |
| D3 | Status 页和托盘面板共享一个订阅，首次可见启动，两处均不可见后停止 | R2 与 R9 原生验收 |
| D4 | Chrome Beta、Chrome Canary、Edge Beta、Edge Dev 已登记配置文件的 HTTP Cache 与 Code Cache | 用户 2026-11-20 确认 |
| D5 | 关闭默认退出，在设置中可改为托盘驻留 | 用户 2026-11-20 确认 |
| D6 | Optimize 默认全不勾，由用户选择 | 用户 2026-11-20 确认 |
| D7 | "CLI 先行"的实现方式是进程内调用 core，而不是派生 CLI 进程 | 本次用户交付约束和各域的同源 CLI 契约 |

## 风险与延后

- 本轮没有运行 GPUI、CLI 和本机的 Mole；界面依据是 Mole 官网同版本截图与归档截图。标"推断""待核对"的条目在各子任务第一步确认。
- GPUI 固定版本的绘制、弹出窗口和位图接口尚未核实，各相关子任务都留了回退方案。
- `slop_check.mjs` 不覆盖 Rust 源码；设计判读是自检，不是独立复核。
- 延后：扫描增长对比（A5）、旧安装包与回收站清理（C5，属有意差异）。
