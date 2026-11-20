# 步骤 9 前评审：当前交付与下一阶段

本任务的既定关口。以下边界已于 2026-11-16 获用户确认，授权仅继续步骤 9；步骤 10 前仍须停下。用户明确要求实施与检查交给 `trellis-implement`、`trellis-check`，只用夹具，不用 Computer Use，不提交或推送。

## 步骤 3–8 的范围

| 部分 | 已实施的行为 | 检查记录 |
| --- | --- | --- |
| core/CLI MSI | 正向资格、系统工具绑定、固定参数、实时摘要、专用退出码/重启证据、审计与旧文件兼容；仅严格 Removed 可进入后续残留流程 | [MSI 检查](msi-check.md) |
| 保持唤醒 | core 共用计划/摘要；同线程获取/清除、有限租期；显式 CLI 会话复用；退出后旧端点不可达保持未知 | [唤醒检查](keep-awake-check.md) |
| 启动项 | 计划、实时预览、二次确认、结果；恢复重新生成反向计划与预览 | [GPUI 检查](gpui-3-8-check.md) |
| 更新 | 实际工具/能力/官方源与用户范围绑定；目标派发前可取消；逐项结果、异常观察保持未知、持久化历史 | [GPUI 检查](gpui-3-8-check.md) |
| 共用审阅 | 固定动作栏、准确确认文案、失效与 single-flight、过期后台结果/轮询拒绝、关窗清理、中英文投影 | [GPUI 检查](gpui-3-8-check.md) |

最终串行门禁与精确计数见 [门禁记录](final-gates-3-8.md)。未取得的原生截图、真实第三方安装器/UAC/重启、完整真实 MSI 资格、WindowsApps 权限与 OS 唤醒效果仍未验收。用户禁止 Computer Use；不以文本投影、测试执行器或临时 MSI 摘要 API 夹具冒充上述原生证据。

## 已确认的步骤 9 边界

- 只接入 Status 进程行的“结束进程”审阅与确认。MSIX/MSI 卸载、残留执行和步骤 13 新跳转仍不开放。
- 复用 core `ProcessTerminationPlanV1`、`bind_process_termination_plan`、`preview_bound_process`、`terminate_bound_process`；绑定 PID 与 Windows creation time，审阅显示进程名/PID及丢失未保存数据的提示。
- 使用 core 的保护结果，UI 不开放受保护条目动作；core 再拒绝。进程消失、身份改变或 PID 复用后必须拒绝旧摘要，不以名字或 PID 单独代替身份。
- 共用审阅、后台 single-flight 和过期投递守卫继续适用。没有当前有效摘要或没有第二次确认不调用终止执行器，不请求提权。
- 在 GUI 接入前补齐该动作所需的 core/CLI 历史合同；不把无审计的现有 helper 直接当成完整 R12 实现，不复用保持唤醒记录伪装进程动作。
- 验证只使用 fake probe/killer 与任务拥有的进程夹具；不结束本机用户实际进程，不使用 Computer Use，不提交或推送。
- 步骤 9 完成并检查后，停在步骤 10 前，再单独评审 MSIX 卸载。

这些是下一步的实施边界，不是对本机真实进程操作的授权。整个 host-actions 任务尚未完成，保持 `in_progress`，不归档。
