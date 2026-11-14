# 设计：多操作维护计划

## 等价 CLI

| 桌面动作 | CLI |
| --- | --- |
| 列出目录 | `devsweep optimize list --format json` |
| 生成计划 | `devsweep optimize plan --operation A --operation B --output PLAN` |
| 预览并取得摘要 | `devsweep optimize preview --plan PLAN` |
| 运行 | `devsweep optimize run --plan PLAN --preview-digest sha256:... --confirm` |

## core（`crates/devsweep-core/src/optimize/`）

- 新增 `MaintenancePlanV2 { version, catalogue_version, operations: [{ operation_id, action_class }] }`。`MaintenancePlanV1`（`plan.rs:156-165`）保留，读取时视为单元素的 V2。
- 计划构造时校验：非空、无重复、每项的 `action_class` 为可执行、满足 `build_floor`。
- 摘要覆盖整份计划的规范化内容（含顺序）。预览对每项各自求实时状态，再合成一个摘要。
- 执行：按计划顺序逐项调用现有的单操作执行路径。每项产生一条结果（已执行、已跳过及原因、失败及原因）。一项失败不终止批次。项与项之间检查取消标志。
- 命令仍是程序与参数分离，不拼接命令行字符串。
- 历史：第一步确认现有历史记录是按操作还是按计划写的；批次沿用每项一条记录，并带同一个批次标识。

## CLI

- `--operation` 改为可重复（`cli.rs:726-734`）。单个时行为与现在一致。
- `optimize run` 的 JSON 输出增加逐项结果数组；人类可读输出逐行列出，中英文。
- 退出码：全部成功或跳过为 0；有失败为非 0。具体取值沿用现有约定，在契约测试里固定。

## GPUI

- `optimize_page`（`page_workbench.rs:488`）分两区。上区列出可执行项，行带复选框、说明、是否需要提权；下区折叠，列出系统设置跳转和指引。只读观察项放在首行的状态里。
- 审阅与确认沿用现有的单操作审阅卡，改为列出多项。
- 运行中：进度按"第几项 / 共几项"显示，不估算时间。
- 结果：每项一行，状态用文字加图标，不只靠颜色。

## 兼容与回滚

- V1 计划文件和 TUI 的单操作调用不变。
- 回滚 GPUI 后 CLI 的批量能力仍可用。

## 风险

- 个别操作耗时长（映像扫描与修复）。默认勾选不含它们；运行时允许在项间取消，并说明正在运行的一项不会被中断。
- 批次里前一项改变了后一项的前提。预览摘要只证明预览时刻的状态；执行时每项仍做自己的前置检查，不满足则跳过并说明。
