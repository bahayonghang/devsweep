# 实施计划

在 airy-glass-shell 之后实施。开始前读：`.trellis/spec/backend/desktop-runtime.md`、`clean-advisories.md`、`scan-coverage.md`；本任务 `design.md`。

## 步骤

1. [x] 核对现状并写 `research/current-flow.md`：结果卡与结论行的内容；core 执行结果的字段；GPUI 调用扫描时如何传范围；`recycle`/选择模型里已有的分组信息。
2. [x] `clean_workbench.rs`：分组纯函数与测试（组小计、三态、仅供审阅组不计入建议量、空组不出现）。
3. [x] 范围选择加"全部"，保留全局确认。测试：未确认全局时不扫全局。
4. [x] 首行各状态（`page_bar`），扫描前无数字。
5. [x] 分组审阅列表与组头复选框；键盘与翻页键回归。
   回滚点 A：到此为审阅改动。
6. [x] 固定动作栏。测试：改动选择后摘要失效、确认按钮不可用。
7. [x] 完成页。
8. [x] 中英文文案。

## 验证

```powershell
just gpui
just ci
```

- 用夹具目录走一遍扫描、审阅、预览、确认、完成，英文与中文各截一组，存入 `evidence/`。真实清理只在夹具目录里做，不清理本机的真实缓存。
- 键盘：只用键盘完成同一流程。
- 授权链：没有预览摘要时确认不可用；摘要过期时执行被拒。用现有测试加新增断言覆盖。

## 评审关口

- 步骤 1 发现 core 缺少逐项结果时，先向用户说明缺口再继续步骤 7。

## 当前进度（2026-11-16）

- 已通过 `task.py start` 启动；步骤 1 由 `trellis-implement` 完成，调查记录见 [current-flow.md](research/current-flow.md)，独立检查见 [current-flow-review.md](research/current-flow-review.md)。
- 步骤 1 确认 core 已有逐项执行结果，原有“缺少逐项结果”关口未触发；实现已保留并展示此前 GPUI 丢弃的明细。
- 用户已确认 AC6 边界：完整的本次执行报告与持久化历史证据分别标明；正常落盘时核对同源、同范围结果，取消或审计失败时不承诺重建未记录条目。已同步 PRD 与设计，继续步骤 2。
- 用户已确认回收站边界：Command 目标仅供审阅，不能通过回收站确认执行。这会使当前桌面流程中的 Rust/provider 命令清理不可执行；core 与 CLI 命令能力不变。
- 步骤 2–8 已由 `trellis-implement` 实施、`trellis-check` 检查并完成局部修复；见 `research/implementation.md` 与 `research/check.md`。最终 `just gpui` 完整通过（217 tests + release build），四线程 `just ci` 完整通过（863 passed、3 ignored）；格式、编译与 Clippy 均通过。Python 23 tests 仅有两个已知 `test_gitignore` 旧失败，未记为整套通过。
- 用户选择先完成代码与门禁、稍后再做原生验收，随后明确要求继续后续任务且不使用 Computer Use 验收。已停止界面操作；未完成的原生项保留为未验证，不阻塞可独立推进的 host-actions 研究。阶段证据见 `evidence/native-acceptance.md`；本任务保持进行中，不因产品步骤已勾选而提前归档或更新父任务为完成。
- 2026-11-16 用户另行授权拆分提交全部既有改动；产品已纳入 `2c5b879`，与 Host Actions 共用 shell/审阅队列的修改一起提交。最新组合门禁见 `../11-12-gpui-host-actions/research/final-gates-step9.md`；原生验收状态没有因提交而改变，不归档、不推送。
