# ProcessRunner core：超时、输出上限与进程树治理

> 父任务：07-28-audit-remediation ｜ 审计条目：F-06、F-13（进程上下文部分） ｜ 对应审计 Milestone M2 / PR8
> 二轮评审 ARCH-004 拆分：本任务只做 runner core；cancellation token 贯穿与状态机接线归 07-28-true-cancellation。
> **本任务是 `ProcessRunner` port 与 typed 进程诊断契约的 owner**（消费方：true-cancellation、durable-audit、rule-registry-providers）。

## Goal

为所有外部命令建立统一的 `ProcessRunner`：deadline、输出上限、进程树终止、typed diagnostics、neutral cwd、输出净化。使挂死的 PATH shim、无限输出的 wrapper、残留孙进程从"必然事故"变成"有界的 typed 结果"。

## 背景与证据（当前 HEAD）

1. **provider 探测无预算（F-06）**：`SystemProviderProbe::command_output` 阻塞式 `Command::output()`，无 deadline、无输出上限；not-found/non-zero/parse 错误全部压成 `None`（`src/providers.rs:107-113`）。PATH shim 挂死 → scan worker 永久挂住而 UI 仍在刷新；global scan 静默缺项。
2. **executor 同样无预算（F-06）**：`ProcessCommandRunner::run` 阻塞 `output()`，stdout/stderr 全量进内存，完整 stderr 拼进错误消息（`src/executor.rs:92-118`）——无限输出耗尽内存，stderr 可携控制字符/敏感 URL 进 UI 与 audit。
3. **provider 继承项目 cwd（F-13 部分）**：探测命令在当前目录执行，project-local 配置（如 `.npmrc`）可改变"global"路径判定。

## Requirements

1. 统一 `ProcessRunner` port：per-command timeout + whole-job deadline；stdout/stderr 有界 ring buffer（默认每 stream 1 MiB，保留 tail，可配置）；typed 结果 `NotFound / Timeout / Exit { code } / InvalidOutput / Canceled`（`Canceled` 变体本任务定义、true-cancellation 接线）。
2. 进程树终止：Windows Job Object / Unix 进程组；超时或终止时孙进程一并回收。
3. neutral cwd：runner 默认在中性目录执行（如临时目录或用户 home），调用方显式声明需要的 cwd。
4. 输出净化契约：stderr/stdout 进入 UI 或 audit 前 sanitize（剥控制字符）+ truncate，净化函数由本任务提供（durable-audit 消费）。
5. executor 的 `ProcessCommandRunner` 与 providers 的 `command_output` 全部迁移到该 runner；provider 探测语义本身不改（归 rule-registry-providers）。
6. runner 接受外部 cancellation token 参数（本任务以 no-op token 预留接口，真实 token 由 true-cancellation 提供）。

## Acceptance Criteria

- [ ] fixture：不退出的 child —— 在 timeout 内返回 `Timeout`，进程树（含孙进程）被终止
- [ ] fixture：持续输出的 child —— 输出被截断至上限，RSS 有界，保留 tail
- [ ] fixture：non-UTF8 输出、非零 exit —— 返回确定的 typed 状态，无 panic
- [ ] provider probe 注入挂死命令 —— deadline 内返回 `Timeout`，global scan 其余 provider 不受影响
- [ ] stderr 含控制字符/超长输出 —— sanitize/truncate 后进入错误消息
- [ ] provider 探测在含 `.npmrc` 的项目目录下运行 —— cwd 中性，探测结果不受项目配置污染（fixture 验证）
- [ ] 审计 §7.3 SLO 初值可配置：provider probe 默认 3–10 s/项、global phase 硬上限 30 s、cleanup per-provider policy 有 hard cap
- [ ] `just ci` 全绿

## 约束与依赖

- 无上游依赖，可与 tui-race-hardening / plan-validation 并行。
- Windows Job Object 与 Unix process group 属平台特定行为：implement.md 验证矩阵必须包含两平台动态验证项及取不到证据时的 No-Go 条件（CI 至少覆盖 Windows + Linux；macOS 行为标注需动态验证）。
- 审计预估 3–5 日。
