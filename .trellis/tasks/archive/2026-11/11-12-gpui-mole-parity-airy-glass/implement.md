# 实施计划（父任务）

父任务不直接改产品代码。它的工作是排期、关口和集成评审。

## 启动前

- [x] 用户确认 D1（2026-11-12：全部接入），`11-12-gpui-host-actions` 的三份文档已写完。
- [x] 用户已逐项明确授权启动 airy-glass-shell、status-dashboard、software-evidence、clean-review-flow，并于 2026-11-15 要求继续后续任务、启动 host-actions。各子任务既定评审关口仍生效；P2 决策不因上述授权而自动通过。

## 波次

1. [x] 波次 1：启动并完成 `11-12-gpui-airy-glass-shell`。已完成并归档；前后对照截图与验收记录见归档任务。
2. [x] 波次 0（可与波次 1 并行）：status-dashboard、software-evidence、analyze-insights、optimize-batch、tray-panel 的 core 与 CLI 部分。共用版本结论见本目录 `research/status-version-strategy.md`。
   - [x] status-dashboard 的 core 与 CLI：已完成并归档；版本结论见 `research/status-version-strategy.md`。
   - [x] software-evidence 的 core 与 CLI：V1 增量与双语契约已完成并归档。
3. [x] 波次 2：status-dashboard、software-evidence、clean-review-flow 的界面部分。
   - [x] status-dashboard 的 GPUI：已完成并归档。
   - [x] software-evidence 的 GPUI：已完成并归档；原生验收及 WindowsApps 图标回退限制见[子任务记录](../11-12-gpui-software-evidence/evidence/native-acceptance.md)。
   - [x] clean-review-flow 的 GPUI 已归档。六态、中英文键盘全流程、翻页、三态 UIA 与两次真实夹具回收的 CLI 历史均已核对。见[归档验收](../11-12-gpui-clean-review-flow/evidence/native-acceptance.md)。
4. [x] 波次 3：Host Actions 的 AC1–AC10 已验收归档。七类动作的双语审阅及结果共 28 张原生图。见[归档验收](../11-12-gpui-host-actions/result.md)。
5. [x] 波次 4：Analyze、Optimize、托盘与缓存扩展均已逐项独立复核、验收及归档。见[父任务集成验收](research/integration-review.md)。

每个子任务单独 `task.py start`、实施、检查、归档。一次只启动能独立验证的那一个或互不相干的几个。

2026-11-16 的历史交接：用户当时授权拆分提交全部既有改动，产品与规范已分批入库，任务证据和日志随后单独提交；不推送。提交不改变 Clean 的部分原生验收状态，也不视为 Host Actions 步骤 10 获批。续作入口见[下个 session Prompt](../11-12-gpui-host-actions/research/next-session-prompt.md)。

## 集成评审（2026-11-20 完成）

- [x] 逐条对照 `research/gap-analysis.md` 第 4 节，标注已解决、有意差异或延后，写入 `research/integration-review.md`。
- [x] 按各子任务的"等价 CLI"表核对父任务 AC2。
- [x] 检索 `desktop-gpui/src` 的进程派生点（AC3）。
- [x] 逐条复核 `AGENTS.md` 的安全契约（AC6）。
- [x] 原生截图矩阵（AC8）。
- [x] 更新 `code_map.md` 和受影响的 `.trellis/spec/` 文档。

## 验证命令

```powershell
just ci      # 格式、检查、测试、Clippy
just gpui    # GPUI 检查、测试、release 构建
just gpui-dev
```

## 约束提醒

- 验证清理、卸载、维护类动作只用夹具或测试执行器。不在本机真实执行，除非用户明确要求。
- 不提交 `target/`、`dist/`、`__pycache__/`。只在用户要求时提交。
- 2026-11-15 用户要求后续任务不使用 Computer Use 验收。采用夹具、测试执行器与源码契约检查；尚未取得的原生截图或 UIA 实机证据明确保留为未验证，不以自动化测试冒充原生通过。
