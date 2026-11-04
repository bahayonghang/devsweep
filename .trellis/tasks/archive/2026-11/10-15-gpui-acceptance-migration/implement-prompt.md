# C14 实施 Prompt

这是顺序第 4 份。它只用来挡住过早的验收和迁移。不要在软件验证之前做测量。

任务：`10-15-gpui-acceptance-migration`。状态仍是 `planning`。
不要运行 `task.py start`。不要归档。

## 本份要做

读父账本 `../10-15-gpui-desktop-rewrite/acceptance-status.md` 的 C14 行，确认：

- C14-AC1、C14-AC2、C14-AC3 仍开放。
- C14-AC4 的勾选只表示没有删除 Tauri，不是迁移完成。
- `desktop/` 仍在。`just tdev` 仍指向 Tauri。
- 旧的单窗口 spike 测量不能代表 `c7846e4` 的五模式窗口。

然后停。不要补测，不要写新的性能样本。

## 本份明确不做

- 不打包、不签名、不发布、不安装。
- 不删除 Tauri，不改 `desktop/`。
- 不重跑 20 次开关、空闲 CPU、相对 Tauri 基线或五模式性能协议。
- 这些测量依赖最后一份软件验证 Prompt 产生的新 release 哈希。
  没有那个哈希时，C14 保持阻塞。
- 不宣称重写完成，不把 NOT_RUN 改成 PASS。

本 Prompt 不授权提交、推送、归档或退役。
