# 设计：Analyze 大文件、多选与分栏

## 等价 CLI

| 桌面动作 | core | CLI |
| --- | --- | --- |
| 最大文件 | 新增的快照投影 | `devsweep analyze largest --snapshot FILE`（新增） |
| 位置与容量 | 位置记录（`analysis/observe.rs:91-96`） | `devsweep analyze roots` |
| 在资源管理器中显示 | `reveal_analyze_node`（`analysis/actions.rs:211`） | `devsweep analyze reveal` |
| 多选回收站计划 | 计划、`preview_analyze_trash`（`:221`）、`execute_analyze_trash`（`:236`） | `analyze trash-plan --select A --select B`、`trash-preview`、`trash-execute` |
| 解释 | 现有解释接口 | `devsweep analyze explain` |

## core 与 CLI

- **最大文件**：对已有快照做纯投影，筛出 `AnalyzeNodeKind::File`（`analysis/model.rs:207`）节点，按容量降序、路径升序，截到上限。默认阈值和上限在实施时定并写进帮助文本。不重新遍历磁盘。
- 第一步确认遍历器的保留上限：快照若只保留每个目录的前若干项，投影就只是"快照中最大的文件"。输出里带一个字段说明是否完整。
- **容量**：`AnalyzeRootRecordV1` 增加可选的卷总量与可用量，沿用现有 `error` 字段的不可用表达。版本策略同 status-dashboard。
- CLI 加 `AnalyzeCommand::Largest`，JSON 与人类可读输出，中英文。

## GPUI

- **分栏**：`analyze_detail`（`page_analyze.rs:202`）改为左右两栏，比例约 2:3；宽度不足时堆叠。树图布局沿用 `layout_node`（`analyze.rs:188`）。
- **多选**：第一步核对 `recycle_ids()`（`analyze.rs:81`）现在是否已支持多个。列表行加复选框，Ctrl 与 Shift 点击扩展选择。底部固定动作栏显示已选数量和容量、预览、确认。
- **上下文菜单**：实施第一步确认固定版本的 GPUI 是否有可用的弹出菜单；没有就用锚定在行上的浮层，带焦点陷阱和 Escape 关闭。菜单项只调用上表里已有的操作。
- **最大文件分段**：与目录视图并列的一个分段，行可多选并加入同一份计划。
- **位置卡**：`analyze_landing`（`:117`）显示大写盘符、总量、可用量和条形。

## 兼容与回滚

- 旧快照文件仍可读。`analyze largest` 是新增子命令，不改现有子命令。
- 回滚 GPUI 不影响 CLI。

## 风险

- 快照很大时投影要有界：单次线性扫描加定长堆，不排序全量。
- 受保护或重解析点节点不能进入回收站计划，沿用 core 现有的拒绝逻辑，界面上禁用对应菜单项并给出原因。
