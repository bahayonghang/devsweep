# 真实取消：token 贯穿、job 状态机接线与退出治理

> 父任务：07-28-audit-remediation ｜ 审计条目：F-03（真取消部分） ｜ 对应审计 Milestone M2 / PR9
> 二轮评审 ARCH-004 拆分：runner core / 进程树归 07-28-process-runner-cancellation；UI 状态机约定由 07-28-tui-race-hardening 先立。
> **本任务是 cancellation token 接口的 owner**（scanner/fs_size/providers/executor 消费）。

## Goal

把 cancellation token 贯穿 scan、size、provider probe 与 executor，使 TUI 的 `Cancelling → Canceled` 成为 worker 确认停止后的真实终态；治理 TUI 退出时 detached worker 与子进程的生命周期。

## 背景与证据（当前 HEAD）

1. `Effect::CancelJob` 只 `send(JobCanceled)`（`src/tui/runtime.rs:136-138`）；无 token、无 control channel、无 JoinHandle、无 child handle——UI 显示 Canceled 时 worker 与已启动命令仍在执行。
2. `Effect::Quit` 为 no-op（`runtime.rs:139`），事件循环退出后 detached worker 继续；用户退出 TUI、终端恢复后，孤儿命令可能仍在删除。
3. tui-race-hardening 已（将）落地状态机约定 `Running → Cancelling → 终态` 与"终态不可复活"；本任务补上真实的 worker 侧确认。

## Requirements

1. cancellation token 接口定型（owner）：scan walker、size 估算、provider probe、executor 每个 target 前后为检查点；ProcessRunner 的 no-op token 参数接入真实 token。
2. worker 确认协议：`Cancelling` 只有在 worker 确认停止（当前 action 完成或被安全终止、不再启动下一 action）后才迁移到 `Canceled`；不可安全中断的 action 显示 `cancel_pending`，在 action 边界停止。
3. executor 取消语义：token 触发后，当前 target 按其可中断性处理（Trash 等待完成；command 经 ProcessRunner 终止进程树），后续 target 一律不启动，report 标记 canceled/partial。
4. TUI 退出治理（决策 D9）：`q`/Ctrl-C 在 mutation job 运行时要求用户选择"继续等待"或"请求取消并等待 worker 确认"；不提供默认或隐式 detach，未达终态不得退出，不得留下孤儿删除进程。
5. 恢复 `x` 取消快捷键的完整语义（race-hardening 止血阶段的降级文案由本任务替换为真实行为）。

## Acceptance Criteria

- [ ] 回归测试：第 N 个 fake action 阻塞时取消，worker 确认停止后第 N+1 个 runner 调用次数为 0；report 标记 canceled
- [ ] scan/size 取消：大 fixture 上取消后 walker 在检查点退出（目标 SLO：walker < 250 ms、target 边界 < 100 ms，实测值记录）
- [ ] provider probe 取消：token 触发后 probe 经 ProcessRunner 终止，scan 返回 partial
- [ ] 迟到 worker 事件不能把 `Canceled` 改回任何状态（衔接 race-hardening 状态机测试）
- [ ] TUI 退出：mutation 运行中 `q` 弹出等待/取消选择；选择取消后进程树无孤儿（平台 fixture 验证）
- [ ] `just ci` 全绿

## 约束与依赖

- 硬依赖：07-28-tui-race-hardening（状态机约定）、07-28-process-runner-cancellation（runner/进程树/Canceled 变体）。
- implement.md 验证矩阵：Windows Job Object 与 Unix process group 的取消动态验证、无法验证平台的 No-Go 条件。
- 审计预估 3–5 日（F-03 长期部分）。
