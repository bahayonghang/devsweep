# Desktop Clean 清理入口与流程可发现性

## Goal

在 Clean 首屏提供直接的清理入口，缩短 扫描→演练→确认 路径，并修正首屏数字与文案，保留演练摘要与二次确认安全契约。

## Background（现状分析）

用户截图：扫描完成后首屏只显示「本次扫描发现 63.6 GiB」、主按钮「复核目标」、次链接「扫描」。首屏没有任何「清理」字样，用户判断为「只有扫描功能」。

清理功能实际存在，但位于第 4 次点击之后：

1. 首屏 `StageResult` 主按钮「复核目标」→ 切换到 `DetailView`（`desktop/src/modes/clean/CleanWorkbench.tsx:293`）。
2. `ReviewPage` 汇总卡片中的按钮「复核演练」→ 调用 `plan_dry_run`（`desktop/src/pages/ReviewPage.tsx:95`）。
3. `ExecutePage` 演练预览中的按钮「确认清理」→ 打开 `ConfirmDialog`（`desktop/src/pages/ExecutePage.tsx:29`）。
4. `ConfirmDialog` 确认 → 调用 `plan_execute`。

已确认的问题：

- P1 可发现性：首屏主操作只有「复核目标」。「清理」一词第一次出现在第 3 步。
- P2 文案不透明：`clean.v1.action.preview` 中文为「复核演练」，没有说明该按钮是清理的第一步。
- P3 首屏数字含义不符：首屏大数字取 `health.totals`（全部发现目标，含 Inspect Only 与未默认选中目标）。用户看不到默认选中项的 Estimated Recoverable，也看不到选中数量。
- P4 格式缺陷：`clean.v1.scan.complete` 以「。」结尾，`MetaLine` 再用「 · 」连接，显示为「6 个目标。 · 5.2 GiB」。
- P5 演练预览与结果表只显示 `target_id`（例如 `cargo.target:C:/work/app/target`），没有显示路径与类别标签（`desktop/src/pages/ExecutePage.tsx:21`）。
- P6 演练预览没有返回首屏的路径；执行结果后「返回」按钮文案为「取消」（`clean.v1.action.cancel`），语义不对。

## Requirements

- R1 首屏清理入口：扫描完成且默认选中非空时，首屏主操作为清理入口，文案包含选中项的 Estimated Recoverable（例如「清理 5.2 GiB」）。点击后直接对当前选中项运行 dry run，并进入演练预览。
- R2 复核入口保留：「复核目标」降为首屏次操作。「扫描」（重新扫描）保留为第三级链接或并入次操作行。
- R3 选中为空：默认选中为空（全部为 Inspect Only 或未默认选中）时，首屏主操作回退为「复核目标」，并显示一行原因说明。
- R4 首屏数字：大数字显示当前选中项的 Estimated Recoverable（verified + partial lower bound）。全部发现总量与目标数进入次要行。partial 与 unknown 语义保持现有 catalogue 文案。
- R5 文案：`clean.v1.action.preview` 改为直接说明下一步（中文候选「预览清理」，英文 `Review dry run` 保持），审阅列表底部/汇总栏主按钮与首屏主按钮使用同一动词体系。修正 P4 的重复标点。
- R6 演练预览可读性：演练预览与结果表每行显示目标路径（有路径时）和类别标签，`target_id` 退为次要文本或 title。
- R7 返回路径：演练预览「返回复核」与执行结果「返回概览」使用正确文案，不复用「取消」。
- R8 审阅页操作栏：`ReviewPage` 的汇总与「预览清理」按钮在长列表滚动时保持可见（固定于 DetailView 底部或顶部），满足 component-guidelines「persistent summary/action bar」。

## Constraints（不可改变的安全契约）

- 执行前必须存在当前选中集合的 dry run 摘要；摘要只来自 `plan_dry_run` 响应，前端不计算、不编辑摘要。
- 执行必须经过 `ConfirmDialog` 二次确认。首屏入口不得跳过 dry run 或确认对话框。
- 选中集合任何变化同步清除 dry run、确认与执行状态（`state-management.md` Invariants）。
- Inspect Only 目标不可选；Scan Preview 观测数据不得进入选中或 dry run。
- 不引入新的生产依赖；不新增或修改 Tauri 命令名与 IPC 契约。
- 文案不得声称空间已释放；回收站结果沿用「已移至回收站」语义。
- 所有新增或修改的文案通过 `resources/i18n/{en,zh-CN}.json` catalogue，React 不自定义键或单位。

## Acceptance Criteria

- [ ] 扫描完成、默认选中非空：首屏主按钮文案包含「清理」和选中 Estimated Recoverable；点击后发起一次 `planDryRun(plan, selectedIds)`，成功后显示演练预览；「确认清理」打开 `ConfirmDialog`；确认后调用 `planExecute` 并显示结果首屏。（R1）
- [ ] 首屏存在「复核目标」次操作，点击进入 `ReviewPage`；「扫描」重新扫描仍可用。（R2）
- [ ] 默认选中为空：首屏主按钮为「复核目标」，并显示原因行；无清理按钮。（R3）
- [ ] 首屏大数字等于 `selectedTotals(state)` 的 verified + partial lower bound；全部发现总量与目标数出现在次要行。（R4）
- [ ] 中英文 catalogue 更新；首屏次要行不再出现「。 ·」。（R5）
- [ ] 演练预览与结果每行显示路径或类别标签；`target_id` 仍可复制或在 title 中可见。（R6）
- [ ] 演练预览与结果页的返回按钮文案分别为「返回复核」「返回概览」（或 catalogue 等价键），不再显示「取消」。（R7）
- [ ] `ReviewPage` 列表滚动到底时，汇总与「预览清理」按钮仍可见。（R8）
- [ ] 首屏入口发起 dry run 期间，按钮 disabled 且 `aria-busy`；dry run 失败时首屏显示 `ErrorBanner`，不进入确认。
- [ ] 现有 `CleanWorkbench.test.tsx`、`skip-protect.test.ts`、`app-state.test.ts` 全部通过；新增测试覆盖 R1、R3、R4、R7。
- [ ] `npm run lint`、`typecheck`、`test`、`build` 通过；`just ci` 通过。
- [ ] 在 800 与 1440 CSS 像素宽度、中英文下截图首屏、审阅、演练预览、结果四个状态。

## Out of Scope

- 后端扫描、规划、执行逻辑与 IPC。
- 永久删除（本构建禁用）。
- Software、Optimize、Analyze、Status 模式。
- 行星渲染与主题。

## Decisions

- D1（2026-09-29，用户确认）：首屏清理入口直接对当前选中项运行 dry run 并进入演练预览。到 `ConfirmDialog` 共 2 次点击。
