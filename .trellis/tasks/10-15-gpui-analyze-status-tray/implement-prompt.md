# C13 实施 Prompt

这是顺序第 2 份。只补还没接到窗口的分析、状态和托盘行为。
不要做原生验证，不要做软件验证。

任务：`10-15-gpui-analyze-status-tray`。分支：`codex/gpui-desktop-rewrite`。
产品接线已经在 `c7846e4`。证据在 `deba8d1`。父顺序见
`../10-15-gpui-desktop-rewrite/next-session-prompt.md`。

## 本份要做

先读 `.trellis/spec` 里 GPUI 层的 Pre-Development Checklist，再读本任务 `prd.md`。
对照 `c7846e4`，只修仍不能操作的需求：

- Analyze 可以选择隔离根或窗口列出的驱动器，不靠手写 `DEVSWEEP_ANALYZE_ROOT`。
- 列表和 treemap 共用稳定节点 id。空、不可读、存在必须能区分。
- 回收使用和 CLI 相同的预览摘要。缺摘要或过期摘要就拒绝，不在界面里另写删除。
- Status 的 Landing 和 Live 显示健康、CPU、GPU、内存、电池、空间、网络、温度和进程。
  缺传感器保持 unavailable。主窗口和托盘共用一个采样订阅。
- 显示的 PID 不能成为结束进程的依据。
- Keep Screen On 不调用 `powercfg`。Clean Screen 的 Escape 或 F11 必须恢复界面，
  不锁输入。

这些如果已经接到窗口，就停。不要重做 `analyze_window.rs` 或 `status_panel.rs`。

## 本份明确不做

留给最后的软件验证 Prompt：

- 在新哈希上重跑 20 次创建、最小化、恢复和 `WM_CLOSE`。
  `6E827733` 的 20 次不能套到 `c7846e4`。
- 大快照渲染。
- OS 崩溃释放。做不到就保持 NOT_RUN，不要杀进程来冒充。
- FocusPattern。不要放宽，不要分叉 AccessKit。
- 勾选 C13-AC3 或 C13-AC4。C13-AC1 和 C13-AC2 的现有勾选只覆盖 `6E827733`，
  不要写成也覆盖 `c7846e4`。
- 归档。

本地测试可以跑，编译失败就修。测试通过不是原生验收。
本 Prompt 不授权提交、推送、归档或删除 Tauri。不要编辑 `desktop/`。
