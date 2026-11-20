# 托盘回调、窗口和累计口径调研

- 问题：托盘能否同时处理鼠标/键盘/Explorer 重建，怎样保持同一采样订阅，累计栏应计什么？
- 范围：所检 Windows GPUI 后端、core V1 历史、Tauri 历史先例、微软 Win32 API。
- 日期：2026-11-20。

## D5 和窗口

建议保留当前“关闭即退出”作为默认值，在 Settings 显式保存后允许关闭驻留。另一选择为“默认驻留”，这会改变旧用户关窗后进程是否继续的行为；托盘不可用时应始终保留可恢复的窗口或退出路径。关闭设置字段沿用 core 严格校验与锁内合并补丁规则。旧 V2 缺新增字段时按确定的默认值解释；保留其他合法字段。已有规范对未知字段/未来版本要求拒读及保留原文件，不能为新增设置而把未知内容吞掉重写。`.trellis/spec/backend/desktop-preferences.md`。

- `desktop-gpui/src/tray.rs:154–194,208–252` 通过 STATIC/HWND_MESSAGE 及 NIF_TIP|NIF_ICON 添加图标，退出删除。没有 NIF_MESSAGE、版本协商、菜单和 TaskbarCreated 接收；AC6 的恢复行为需开发。建议用任务拥有的隐藏顶层接收窗口与轻量回调把事件送到 GPUI 主线程，对每次 NIM_ADD 配套 NIM_SETVERSION；处理 NIN_SELECT、NIN_KEYSELECT、WM_CONTEXTMENU 时检查版本和图标 ID。TaskbarCreated 发送给顶层窗口，恢复后重新提交图标及版本。避免同时响应鼠标点击和等价合成选择从而打开两次。[任务栏](https://learn.microsoft.com/en-us/windows/win32/shell/taskbar) · [通知数据](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-notifyicondataw)
- 右键显示“打开 DevSweep / 退出”。原生菜单开前让接收窗口获得前台，按系统惯例返回焦点和投递空消息；正常操作仅响应用户主动激活托盘。按图标屏幕矩形和显示器工作区计算面板坐标，考虑负坐标和 DPI。固定 GPUI 的 WindowKind::PopUp 在 Windows 实现为 toolwindow/topmost，原生 AnchoredPopup 会返回不支持。可用 PopUp 配独立定位；失焦/Escape/失败回收面板句柄及可见标记。`desktop-gpui/vendor/gpui_windows/src/window.rs:469–473,519–521`。[菜单 API](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-trackpopupmenu) · [图标矩形](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shell_notifyicongetrect)
- `desktop-gpui/src/status_panel.rs:294–333` 初始化 tray_visible=false，`reconcile` 已按可见性去重。共享会话持有一个采样器，面板只读同帧已发布的样本。页签为 Status 但窗口最小化/关闭驻留时须撤销主面板可见性；popup 销毁、失焦、退出都撤销 tray 可见性。分别测两者同时可见只 start 一次；最后一处隐藏，stop 并检查无后续样本。明确主面板 pause 和面板仍可见时的订阅需求。避免用托盘图标存在代替面板真正可见。
- 主会话核对的旧版 `0d32939`：`desktop/src-tauri/src/tray.rs:199–232` 使用独立菜单事件和左键释放切换 HUD；窗口事件 233 起，打开主窗口 282、切换 HUD 290、创建/显示 321/336、隐藏 359、布置和位置 397/441。`hud.rs:46,91,111` 启停并在析构 join，185/218/229 测停止、等待结束及防重复。借鉴行为与回归用例，采样归当前 Session 统一管理。

## 历史合计

- **已移入回收站**：复用 `crates/devsweep-core/src/history/mod.rs:317–351`，它只加成功的 MoveToTrash 记录估算字节；未知容量增加 unknown_records，部分容量与异常历史标记 lower_bound，显示“至少”。不要从清理计划或当前磁盘空闲量反推。
- **已卸载**：只计主 Software 执行历史的 `Terminal { Removed }`，按 operation_id 和软件身份去重；重启待确认、仍安装、未知、开始、重查中及 software-support 的更新、启动项、残留均不计。不要对 `is_success` 或所有 Software 红绿标签计数，前者包括 RebootRequired。一个批次中不同软件分别统计。参见 `software/execution/audit.rs:82–116`、`software/execution/mod.rs:48–66`、`history/mod.rs:426–448`。
- **已优化**：只计 Execute 的 `Terminal { Succeeded }`，按运行与目录项身份计算。已打开 Settings 的 Launched、AdapterCompleted、可见过的目录项或 started-only 排除。将来 R8 批次以逐项终态为单位。`optimize/audit.rs:54–91`、`optimize/execution.rs:45–67,437–469`。
- 给每个域分别提供读取完整性。空历史是明确的零，受损或未知版本只可声称读到的下界；不可读不能显示为精确零。重复、部分、错误或互相矛盾的终态不能反复记功；可用有效历史与审计验证，不借助 UI 本地成功提示。不要以带分页和 limit 的 list 接口作累计。累计读取在后台，避免每次采样/渲染重新读所有文件。`history/mod.rs:450–545`。

## 证据边界

尚未运行原生托盘、资源管理器重启或 125% DPI。可模拟向任务拥有窗口投递已注册消息并检查图标重加，但需要把实际 Explorer 重启单独列明；共享主机不为截图重启 Explorer。参考规范为 `.trellis/spec/backend/desktop-runtime.md`、`desktop-preferences.md`、`database-guidelines.md` 与子任务验收表。
