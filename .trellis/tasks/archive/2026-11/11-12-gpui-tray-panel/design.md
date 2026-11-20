# 设计：托盘面板与累计统计

## 等价 CLI

| 桌面内容 | core | CLI |
| --- | --- | --- |
| 累计统计 | `history::clean_moved_totals()`（`history/mod.rs:301`）加软件、优化的成功计数 | `devsweep history totals`（新增） |
| 面板读数 | 与 Status 页相同的采样 | `devsweep status live` |

## core 与 CLI

- `clean_moved_totals` 已返回 `CleanMovedTotalsV1 { known_bytes, unknown_records, lower_bound }`。新增一个合并视图，加上软件与优化两个域的成功记录数。只读历史，不写。
- CLI 加 `HistoryCommand::Totals`。

## GPUI

- **托盘消息**：`notify_data`（`tray.rs:229-252`）现在只设 `NIF_ICON | NIF_TIP`。加上回调消息，在消息窗口里处理左键、右键和键盘激活。实现前可参考已移除的 Tauri 版先例（提交 `0d32939` 及更早版本中的 `desktop/src-tauri/src/tray.rs`、`hud.rs`，用 `git show` 查看）和 vendor 的 `gpui_windows` 是否已有消息窗口可挂接。
- **菜单**：原生弹出菜单，两项。显示前把消息窗口设为前台，这是 Windows 托盘菜单能正常关闭的前提。
- **面板**：一个无边框的小窗口，定位在托盘图标附近并限制在工作区内。第一步确认固定版本的 GPUI 支持的窗口种类（弹出窗口、置顶、不进任务栏）；不支持时退为一个普通的小窗口。失去焦点或按 Escape 关闭。内容复用 status-dashboard 的指标组件的紧凑形态。
- **可见性**：`session.status_set_tray_visible`（`main.rs:62`）只在面板打开时为真。`StatusPanel::reconcile`（`status_panel.rs:55-66`）的逻辑不用改。status-dashboard 已把启动时的置位改为假。
- **关闭行为**：偏好增加一个枚举（退出、驻留托盘），默认退出。按 `.trellis/spec/backend/desktop-preferences.md` 的规则加字段，保证旧文件可读、未知字段保留。`view.rs:2379` 的 `cx.quit()` 按偏好分支。驻留时主窗口隐藏，采样随可见性停止。
- **保持唤醒**：只留一个由能力开关控制的位置，本任务不实现。

## 兼容与回滚

- 托盘初始化失败时应用照常运行（现有行为）。面板创建失败时退回为"左键打开主窗口"。
- 回滚 GPUI 不影响 `history totals`。

## 风险

- 托盘消息处理在 vendor 补丁里的话，升级 GPUI 固定版本时要重新核对。改动尽量放在 `desktop-gpui/src/tray.rs`。
- 多显示器和不同缩放下的面板定位。验收里含一次 125% 缩放的截图。
- 驻留托盘后用户可能以为程序已退出。首次驻留时在托盘提示文字里说明，不弹通知。
