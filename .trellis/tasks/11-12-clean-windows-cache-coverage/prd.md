# Clean：Windows 缓存规则覆盖扩展（core/CLI）

父任务：[11-12-gpui-mole-parity-airy-glass](../11-12-gpui-mole-parity-airy-glass/prd.md)。差距编号 C4 见父任务 `research/gap-analysis.md`。

## 目标

让 Clean 的全局范围认出更多可再生的 Windows 应用缓存。规则只加在 core 的封闭目录里，CLI 与桌面端自动呈现。

## 背景

现有 6 个家族（`rules/definitions.rs:5-30`）：Chrome、Edge、Firefox、VS Code、Cursor、用户临时文件。每个家族有封闭的目录列表和"所有者正在运行则跳过"的保护（`rules/windows_caches.rs`）。

## 需求

| 编号 | 需求 |
| --- | --- |
| R1 | 为选定的家族各加一条规则：封闭的缓存子目录列表、所有者进程名、可再生的依据。 |
| R2 | 所有者进程在运行时不生成清理目标，并说明"关闭后重新扫描"。 |
| R3 | `clean rules list` 与 `clean rules show` 列出新规则，中英文。 |
| R4 | 已排除项保持排除：错误报告、Windows Update 下载、传递优化、Downloads、着色器缓存、AI 会话、Docker、Cargo home。 |
| R5 | 规则只产生清理计划，不删除、不移动、不执行外部命令。 |

## 验收

- AC1：每个新家族有夹具测试：命中预期目录；不命中配置、登录状态、历史、扩展、会话等非缓存目录；所有者运行时无目标。
- AC2：`clean rules list --format json` 含新规则 ID；契约测试更新。
- AC3：覆盖遗漏表（`definitions.rs:33-60`）里的排除项仍在，且有测试断言。
- AC4：GPUI 与 TUI 不改代码即可在全局范围看到新目标（用夹具验证）。
- AC5：`just ci` 通过。

## 不在范围

通讯、办公、网盘、会议、游戏平台应用的缓存。Windows 日志与内存转储。回收站与旧安装包。任何需要提权的路径。

## 待定（实施前在本任务的规划关口由用户决定）

纳入哪些家族。候选：Chrome Beta 与 Canary、Edge Beta 与 Dev、Brave、Vivaldi；开发工具 Trae、Qoder、Windsurf、Zed。每个候选在调研里给出目录证据后再定。未获确认的家族不实现。
