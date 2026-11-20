# Windows 托盘验收

2026-11-20，在独立非输入桌面捕获，系统窗口 DPI 为 120（125%）。每个语言和主题组合各一份 JSON，内含 Status、面板及菜单 PNG 的 SHA-256、原生操作结果和夹具采样记录。四组包含英文、简体中文，以及浅色和深色。菜单使用 Windows 原生样式。额外的 `awake-review.json` 记录了从托盘进入既有保持唤醒审阅，UIA 读到待确认控件，菜单退出码为 0。

- **同一读数**　测试窗口打开实际 Status 页和托盘面板。二者共享 `snapshot-fixture`。CPU 为 43%，GPU 为 12%，内存为 50%，供电显示无电池。面板还展示机器信息、按 CPU 排序的前五个测试进程和累计统计。
- **生命周期**　两处同时可见时只启动一次订阅。按 Escape 关闭面板，再最小化主窗口，停止计数加一。随后 800 ms 内没有新样本。键盘选择通知图标在最小化状态仍可打开面板。英文浅色还验证了面板的失焦消息。
- **关窗偏好**　三组通过保存的驻留偏好验证了关窗后进程仍在、面板能打开和菜单恢复主窗口。中文深色使用默认偏好，并由关闭主窗口结束进程。每次退出均删除通知图标。
- **通知图标**　夹具向实际顶层接收窗口投递 NIN_SELECT、NIN_KEYSELECT、WM_CONTEXTMENU 和已注册的 TaskbarCreated。测试 shell 观察到启动与重建各一组 NIM_ADD、NIM_SETVERSION。菜单是实际的 Win32 弹出菜单，使用字符快捷键选择打开及退出。本机 Explorer 全程运行。此项采用用户确认的消息复验口径。
- **数据来源**　采样控制器和所有窗口走生产入口。测试采样通道只送入固定快照，进程列表为 fixture-1 至 fixture-6，累计栏用固定测试数值，保持唤醒使用 RecordingBackend。实际 V1 历史读写边界由独立 core 和 CLI 测试验证。

## 复现

在仓库根目录启动 Windows PowerShell。运行环境需要 Python、pywin32、Pillow 和 Windows PowerShell 内置 UIAutomationClient。将 `$proof` 指向含本说明的目录。将 `$bin` 设为 Cargo 输出中 `src/main.rs` 的测试程序路径。输出目录使用新的临时目录。

```powershell
$env:RUSTFLAGS = '-C link-arg=/Brepro'
cargo test --locked --manifest-path desktop-gpui/Cargo.toml --bin devsweep-gpui-spike --no-run
$env:DEVSWEEP_TRAY_RESIDENT = '1'
$env:DEVSWEEP_TRAY_AWAKE_REVIEW = '1'
$out = Join-Path $env:TEMP ('devsweep-tray-' + [guid]::NewGuid())
python "$proof/private_desktop.py" python "$proof/tray-driver.py" $bin en:light $out
```

可将场景改为 `en:dark`、`zh:light` 或 `zh:dark`。验证默认关闭行为时，将 `DEVSWEEP_TRAY_RESIDENT` 设为 0，`DEVSWEEP_TRAY_DEFAULT_EXIT` 设为 1。探针只操作夹具进程拥有的窗口，并在结束时关闭启动的进程。
