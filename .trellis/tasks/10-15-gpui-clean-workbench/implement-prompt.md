# C11 实施 Prompt

这是顺序第 1 份。只补还没接到窗口的清理行为。不要做原生验证，不要做软件验证。

任务：`10-15-gpui-clean-workbench`。分支：`codex/gpui-desktop-rewrite`。
产品接线已经在 `c7846e4`。证据在 `8d050f8`。父顺序见
`../10-15-gpui-desktop-rewrite/next-session-prompt.md`。

## 本份要做

先读 `.trellis/spec` 里 GPUI 层的 Pre-Development Checklist，再读本任务
`prd.md` 和 `design.md`。对照 `c7846e4` 的窗口，只修仍不能操作的需求：

- 独立的全局缓存扫描和项目产物扫描，以及首次使用时的根选择。
- 扫描有行时的只读磁盘地图：按估算字节分块，按父目录嵌套，不发明空闲空间，
  不绕过预览和确认。
- 选择、实时预览、显式确认、执行结果和历史仍走已接受的 core/CLI。
- AI 审阅仍只用显式临时 home。没有隔离 home 时不读真实 `.codex`、`.claude` 或 Cursor。

如果这些都已经能从窗口操作，就停。不要为了“看起来更完整”重写 `clean_workbench.rs`、
`clean_map.rs` 或 `clean_ai.rs`。

## 本份明确不做

这些留给最后的软件验证 Prompt，不要在这里停下来做，也不要写成 PASS：

- 16.7 ms PageDown。不要放宽预算，不要分叉 AccessKit。
- 100 次取消样本。现有 6 次样本不够。
- UIA FocusPattern 和完整键盘加 UIA 审阅流。
- 重跑 10k 帧间隔、五模式落地页或新的 release 哈希。
- 勾选 C11-AC1 到 C11-AC4。它们保持未勾选。
- 归档。父账本状态是 NOT_ACCEPTED。

本地门可以跑 `desktop-gpui` 下
`cargo test --locked --target-dir dist/goal-gpui-test`。编译失败就修。
这不是原生验收。

本 Prompt 不授权提交、推送、归档、安装、签名、发布或删除 Tauri。
不要编辑 `desktop/`。不要对这台机器执行真实用户缓存清理。
