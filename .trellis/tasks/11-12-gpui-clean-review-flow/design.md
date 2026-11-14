# 设计：Clean 汇总、分组审阅与完成页

预期只改 `desktop-gpui`。授权链不变。

## 等价 CLI

| 桌面动作 | CLI |
| --- | --- |
| 扫描"全部"范围 | `devsweep clean scan --scope all --root <dir>...` |
| 生成计划 | `devsweep clean plan --observation FILE --select ID... --output PLAN` |
| 预览并取得摘要 | `devsweep clean preview --plan PLAN` |
| 移入回收站 | `devsweep clean execute --plan PLAN --preview-digest sha256:... --confirm` |

## 页面状态

| 状态 | 首行（状态 · 数字 · 动作） | 主体 |
| --- | --- | --- |
| 未扫描 | 范围与目录 · 无数字 · 扫描 | 范围选择（项目、全局、全部）与目录选择 |
| 扫描中 | 进度 · 已找到的下界 · 取消 | 已发现的分组 |
| 已扫描 | "找到 X · 建议清理 Y" · 审阅 | 分组概览与磁盘图 |
| 审阅 | 已选 Z · 预览 | 分组列表；底部固定动作栏 |
| 已预览 | 摘要已生成 · 确认移入回收站 | 同上，选择锁定 |
| 完成 | "已移入回收站 ≥ N" · 再次扫描 | 按组的已移动、失败、跳过及原因 |

扫描前不显示 0。数字未知时写未知，下界写"至少"。

## 要点

- **全部范围**：`clean_scope_tabs`（`page_clean.rs:261`）加第三项，对应 core 的 all 范围。全局部分仍要先过 `clean_global_confirm`（`page_clean.rs:322`）；未确认时"全部"只扫项目并在首行说明。
- **分组**：在 `clean_workbench.rs` 加纯函数，把审阅行按生态或目标种类分组，输出组名、组小计、组内行、组的勾选状态（全选、部分、未选）。磁盘图已有的分组依据（`page_clean.rs:552-600`）复用同一函数。
- **仅供审阅的组**：需要单独处理的行归入一组，不计入"建议清理"，默认不勾选。
- **审阅列表**：`clean_rows_card`（`:734`）和 `clean_row`（`:882`）改为组头加组内行。组头是三态复选框，键盘 Space 切换。现有的行级键盘操作和翻页键保留。
- **动作栏**：`clean_action_bar`（`:990`）固定在纸的底部（用 airy-glass-shell 的 `sheet_frame`），显示已选容量、预览、确认。选择变化后旧摘要立即失效，这一行为不变。
- **完成页**：第一步先核对 `clean_result_card`（`:446`）和 `clean_outcome_line`（`view.rs:1612`）现在给出的内容，以及 core 的执行结果里是否带逐项的失败与跳过原因。有就直接呈现；没有就只呈现已有的，并把缺口记到父任务，不在本任务改 core。
- **路径**：用 `ui::display_path`。

## 兼容与回滚

- 元素 ID、无障碍名称和现有测试的文本投影尽量不变；分组引入的新 ID 另起前缀。
- 回滚只涉及 `page_clean.rs`、`clean_workbench.rs`、`copy.rs`。

## 风险

- 行数很大时分组不能每帧重算。分组结果随扫描结果缓存。
- 三态复选框的无障碍状态要报"部分选中"，在 UIA 构建里验证。
