# Design: Desktop Clean 清理入口与流程可发现性

## Boundary

只改 `desktop/src/` 的 Clean 模式呈现层与 `resources/i18n/{en,zh-CN}.json` 中的新增键。reducer（`desktop/src/state/app-state.ts`）、IPC、Rust 后端不变。

## Flow

```text
Stage(found) --[清理 {selected}]--> dryRun() --success--> view=detail, ExecutePage(dry run)
                                                      --[确认清理]--> ConfirmDialog --> execute() --> Stage(result)
Stage(found) --[复核目标]--> view=detail, ReviewPage --[预览清理]--> dryRun() --> ExecutePage(dry run)
ExecutePage(dry run) --[返回复核]--> review_requested --> ReviewPage
Stage(result) --[详情]--> ExecutePage(final) --[返回概览]--> view=stage
```

reducer 状态序列不变：`reviewed -> (pending dry_run) -> dry_run -> confirming -> executing -> reported`。首屏入口复用现有 `dryRun()`，只在成功分支追加 `setView("detail")`。

## Component changes

### `CleanWorkbench.tsx`

- found 分支：
  - `selected = selectedTotals(state)`；大数字 = `selected.verified_bytes + selected.partial_lower_bound_bytes`；caption 用新键 `clean.v1.stage.selected_caption`。
  - 次要行：新键 `clean.v1.stage.found_meta`（`{count}`、`{bytes}`，全部发现总量）+ 选中数量 `clean.v1.preview.selected`。不再拼接 `clean.v1.scan.complete`，因此不出现「。 ·」。
  - `state.selectedIds.size > 0`：主按钮 = `clean.v1.action.clean_selected`（`{bytes}` = 选中 Estimated Recoverable 格式化值），`onClick={() => void dryRun()}`，`disabled={busy}`；次操作行 = 「复核目标」按钮 + 「扫描」链接。
  - `state.selectedIds.size === 0`：主按钮 = 「复核目标」；次要行追加 `clean.v1.stage.none_selected`；次操作 = 「扫描」链接。
  - `busy` 为真时 `StageResult` 传 `busy`，Stage 设置 `aria-busy`。
- `dryRun()` 成功分支：`dispatch(dry_run_succeeded)` 后 `setView("detail")`。reducer 拒绝不匹配的 outcome 时，`showDryRun` 为假，detail 显示 `ReviewPage`，不会进入确认。
- dry run 失败：`command_failed` 设置 `state.error`；首屏已渲染 `errorBanner`，无需改动。
- 结果首屏「详情」进入 detail 后，`ExecutePage(final)` 的 `onReturn` 改为 `setView("stage")`（当前为 `review_requested`，会清除执行结果）。

### `StageResult.tsx`

- 新增可选 `busy?: boolean`，透传给 `Stage`。

### `ExecutePage.tsx`

- 新增可选 prop `targets?: readonly UntrustedTarget[]`，按 `target_id` 查找。行主文本 = `target.path ?? kindLabel(target.kind)`；次文本 = 现有动作标签；`target_id` 放入 `title` 属性并保留为可选中文本（`.secondary` 小字）。
- 返回按钮文案：`final ? clean.v1.action.back_overview : clean.v1.action.back_review`。

### `ReviewPage.tsx`

- `.review-summary` 在 DetailView 内 `position: sticky; top: 0`（`desktop/src/pages` 所用样式文件中），背景用 raised card token，保证滚动时汇总与「预览清理」可见。现有结构不变。

## Catalogue changes

`resources/i18n` 与 CLI 共用。只新增 desktop 使用的键，不修改 CLI 使用的 `clean.v1.scan.complete`。

| key                               | en                                                            | zh-CN                                    |
| --------------------------------- | ------------------------------------------------------------- | ---------------------------------------- |
| `clean.v1.action.clean_selected`  | `Clean {bytes}`                                               | `清理 {bytes}`                           |
| `clean.v1.stage.selected_caption` | `Selected, estimated recoverable`                             | `已选中，估计可回收`                     |
| `clean.v1.stage.found_meta`       | `Found {count} targets, {bytes} in total`                     | `共发现 {count} 个目标，合计 {bytes}`    |
| `clean.v1.stage.none_selected`    | `No target is selected by default. Review targets to choose.` | `没有默认选中的目标。请复核目标后选择。` |
| `clean.v1.action.back_review`     | `Back to review`                                              | `返回复核`                               |
| `clean.v1.action.back_overview`   | `Back to overview`                                            | `返回概览`                               |

修改：`clean.v1.action.preview` zh-CN 由「复核演练」改为「预览清理」；en 保持 `Review dry run`（component-guidelines 规定的命令标签）。

新增键需满足 catalogue 结构（`forms`、`placeholders`、`count`、`accelerator`、`group`、`truncation`）与现有 en/zh 键集合一致性测试。`found_meta` 的 `{bytes}` 在 partial 时传入 `capacityParts` 已有的本地化片段组合，保持 partial/unknown 语义。

## Compatibility and safety

- 首屏入口调用的是同一个 `dryRun()` 与同一个 `coordinator.start({ kind: "clean.dry-run" })`，单飞与取消语义不变。
- 执行仍只能从 `confirming` 态经 `ConfirmDialog` 触发；摘要仍只取自 `state.dryRun.digest`。
- 首屏按钮的 `{bytes}` 只是展示值，不参与任何授权判断。

## Rollback

全部改动在 desktop 前端与两个 catalogue 文件。回滚 = revert 本任务提交。

## Implementation notes（实现偏差）

- 复用已有键：`stage.v1.caption.estimated`（代替 `clean.v1.stage.selected_caption`）与 `stage.v1.action.back`（代替 `clean.v1.action.back_overview`）。实际新增 4 个键：`clean.v1.action.clean_selected`、`clean.v1.action.back_review`、`clean.v1.stage.found_meta`、`clean.v1.stage.none_selected`。
- `clean.v1.stage.found_meta` 只带 `{count}`；容量沿用 `capacityParts` 的 verified/partial/unknown 片段，保持 partial 语义。stopped 首屏同样改用该键。
- 执行结果详情页不再渲染底部返回按钮。`DetailView` 的「返回概览」已提供同一动作，保留两个同名按钮会重复。演练预览底部保留「返回复核」。
- 审阅汇总栏 sticky 只在视口高度 ≥ 800px 时启用，`top: 41px` 让出 `.window-chrome`。
