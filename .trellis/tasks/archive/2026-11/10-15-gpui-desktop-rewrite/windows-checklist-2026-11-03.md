# Windows 原生执行清单（2026-11-03）

这份清单对应 `next-session-prompt.md` 的第 5 份（软件验证），并加上本轮新增功能的原生核对。
顺序是固定的：先 C11，再 C13，再 C06-AC4，再本轮新增功能，最后是 Software。

本清单不授权以下事情：打包、签名、安装、退役 Tauri、归档任务、勾选验收框。

## 0. 规则（全程适用）

- 不卸载真实软件，不改真实启动项。
- 不对本机运行 defrag、`dism /Online`、`/RestoreHealth` 或 `w32tm /resync`。
- 显示的 PID 只是信息，不要用它结束任何进程。
- 失败就记失败。不放宽 16.7 ms、100 ms 或 100 次样本的预算。样本数不够就记"未满足"。
- 只用本节第 1 步构建出的那个哈希做测量。中途重新构建过，就从第 1 步重来。
- 脚本因界面文字变化而找不到控件时，记 `NOT_RUN（harness 不匹配）`，并把日志发我。
  我只改脚本里用来匹配的字符串，不改预算。

所有命令都在仓库根目录执行：

```powershell
cd D:\Documents\Code\Rust\Exp\devsweep
git pull --ff-only origin codex/gpui-desktop-rewrite
git rev-parse HEAD
git status --short
Get-Process devsweep-gpui-spike -ErrorAction SilentlyContinue | Stop-Process -Force
```

记下 `HEAD`。`git status` 应为空。

## 1. 测试门和 release 构建

```powershell
cd desktop-gpui
cargo test --locked --target-dir dist/goal-gpui-test; "test_exit=$LASTEXITCODE"
$env:RUSTFLAGS = '-C link-arg=/Brepro'
cargo build --locked --release --target-dir dist/goal-gpui-release; "build_exit=$LASTEXITCODE"
Remove-Item Env:RUSTFLAGS
$bin = 'dist\goal-gpui-release\release\devsweep-gpui-spike.exe'
"length=$((Get-Item $bin).Length)"
"sha256=$((Get-FileHash $bin -Algorithm SHA256).Hash)"
cd ..
```

记录 `test_exit`、`build_exit`、字节数和 SHA-256。
另外注明：用的是 rustc 1.99.0，这不算 MSRV 1.88.0 通过。
`6E827733`、`5BFE59AF`、`23F7D614` 这几个旧哈希都不能复用。

## 2. C11：10k 行、帧间隔、取消和焦点

需要的脚本已经准备好：

- `c11-r11-perf.ps1` 和 `c11-r11-select-frames.ps1`：复制自 r10，只改了输出文件名和 fixture 路径。
  另外 perf 的取消循环改成 105 次，前 5 次作预热丢弃，保留 100 个样本。
  r10 的证据不会被覆盖。
- `windows-focus-probe.ps1`：新写的脚本，放在 `10-15-gpui-desktop-rewrite/evidence/`。

### 2.1 建 10k fixture

r11 脚本固定读取这个路径。

```powershell
$root = 'C:\Users\lyh\AppData\Local\Temp\devsweep-gpui-c11-r11-perf\scan-root'
if (Test-Path $root) { Remove-Item $root -Recurse -Force }
for ($i = 0; $i -lt 10000; $i++) {
    $p = Join-Path $root ('p{0:D5}' -f $i)
    $c = Join-Path $p '.pytest_cache\v\cache'
    [void][System.IO.Directory]::CreateDirectory($c)
    [System.IO.File]::WriteAllText((Join-Path $p 'pyproject.toml'), "[project]`nname = `"p$i`"`n")
    [System.IO.File]::WriteAllText((Join-Path $c 'lastfailed'), '{}')
}
(Get-ChildItem $root -Directory).Count
```

最后一行应输出 `10000`。

### 2.2 运行

```powershell
$c11 = '.trellis\tasks\10-15-gpui-clean-workbench\evidence'
powershell -ExecutionPolicy Bypass -File "$c11\c11-r11-perf.ps1"
powershell -ExecutionPolicy Bypass -File "$c11\c11-r11-select-frames.ps1"
powershell -ExecutionPolicy Bypass -File .trellis\tasks\10-15-gpui-desktop-rewrite\evidence\windows-focus-probe.ps1 -Round r11
```

每个脚本运行时都不要动鼠标和键盘，它们会把窗口切到前台并模拟按键。

从输出里抄下这些行：

| 来源 | 要抄的行 |
| --- | --- |
| `c11-r11-perf.txt` | `binary_sha256`、`row_count`、`interaction_n` / `INTERACTION_RESULT`、`cancel_n`、`CANCEL_SAMPLE_COUNT`、`CANCEL_ACK_BUDGET`、`frame_n` / `FRAME_RESULT` |
| `c11-r11-select-frames-log.txt` | `frame_burst_n`、`nearest_rank_p95_us`、`FRAME_BURST_RESULT`。不轮询 UIA 的 PageDown 连按，这是正式的帧样本 |
| `r11-focus-probe.txt` | 每一步的 `step=` 行、`distinct_ids`、`FOCUS_TRAVERSAL` |

### 2.3 先核对一件事：`FocusPattern`

旧的 r10 脚本用 `[System.Windows.Automation.FocusPattern]` 来检测焦点。
我认为 .NET 的 UIA 客户端里没有这个类型：UIA 用 `FocusedElement` 和 `HasKeyboardFocus` 表达焦点，不是一个控件模式。
旧脚本用 `try {} catch {}` 包住了这次调用，所以类型不存在时也只会记成"0 个暴露"。
请先在 PowerShell 里验证：

```powershell
Add-Type -AssemblyName UIAutomationClient
[System.Windows.Automation.FocusPattern]
```

- 如果报 `Unable to find type`，说明以前的 `FocusPattern NOT_PASS` 测的是一个不存在的类型。
  那它就不是应用缺陷的证据。改用 `windows-focus-probe.ps1` 的结果判断焦点。
  旧记录要不要改，由你决定。我不会自己改写旧结论。
- 如果类型存在，两种结果都记下来。

## 3. C13：生命周期和大快照

```powershell
powershell -ExecutionPolicy Bypass -File .trellis\tasks\10-15-gpui-analyze-status-tray\evidence\c13-r4-cycles.ps1
```

在 `c13-r4-cycles.txt` 里抄 `binary_sha256`、`clean_exits`、`forced`，以及每轮的 `leftover`。
合格结果是 `clean_exits=20`、`forced=0`、每轮 `leftover=0`。

大快照可以复用第 2 步的 10k fixture，它大约有 4 万个节点：

1. 打开 Analyze，选择文件夹 `...\devsweep-gpui-c11-r11-perf\scan-root`。
2. 记录从开始扫描到 treemap 和列表出现大约用了多少秒。
3. 在 treemap 中点开一个子目录，再按 Up 返回。确认没有卡死，列表和 treemap 的选中项一致。
4. 不要点"移到回收站"。

这一步做不了就记 `NOT_RUN`。OS 崩溃释放保持 `NOT_RUN`，不要用杀进程来冒充。

## 4. C06-AC4

本机的主机卷和系统映像都不是可丢弃目标，所以记 `NOT_RUN`，原因写"无可丢弃卷或映像"。
不要运行 `dism /Online` 或 `/RestoreHealth`。

如果你想专门建一个可丢弃的 VHDX 来测 defrag，这需要你单独决定。本清单不包含这一步。

## 5. 本轮新增功能的原生核对

这些功能只在 Linux Xvfb 上预览过，Windows 上都还没跑过。
请打开讲述人（Ctrl+Win+Enter）或 Accessibility Insights，确保无障碍处于活动状态。
用 release 二进制和隔离的 `LOCALAPPDATA` 启动：

```powershell
$t = Join-Path $env:TEMP 'devsweep-gpui-r11-manual'
New-Item -ItemType Directory -Force "$t\local", "$t\roaming" | Out-Null
$env:LOCALAPPDATA = "$t\local"; $env:APPDATA = "$t\roaming"
& .\desktop-gpui\dist\goal-gpui-release\release\devsweep-gpui-spike.exe
```

这个 PowerShell 窗口只用于本节，做完就关掉，因为它改了 `LOCALAPPDATA`。

### 5.1 键盘焦点

| 检查 | 期望 |
| --- | --- |
| 连按 Tab | 焦点依次落在标签、按钮、卡片上，每个焦点都有蓝色外环 |
| Shift+Tab | 反向移动 |
| 焦点在某个模式标签上时按 Enter 或空格 | 切换到该模式 |
| 用鼠标点击已选中的标签，再依次点其他四个 | 不崩溃（对应 `982d936` 的修复） |
| 用鼠标点击后 | 焦点外环不显示，只在键盘操作后显示 |

### 5.2 自绘标题栏

| 检查 | 期望 |
| --- | --- |
| 拖动顶栏空白处 | 窗口移动 |
| 双击顶栏空白处 | 最大化，再双击还原 |
| 拖到屏幕上沿或左右边缘 | 贴靠 |
| Win11：鼠标悬停在最大化按钮上 | 出现贴靠布局菜单 |
| 最小化、最大化/还原、关闭三个按钮 | 都能用。关闭按钮悬停时为红底白图标 |
| 点关闭 | 进程正常退出 |
| 顶栏里的标签、主题按钮、菜单 | 能点击，不会触发拖动 |

### 5.3 Analyze：在窗口里移入回收站

先建一个隔离目录：

```powershell
$a = Join-Path $env:TEMP 'devsweep-gpui-r11-analyze'
if (Test-Path $a) { Remove-Item $a -Recurse -Force }
New-Item -ItemType Directory -Force "$a\nested" | Out-Null
Set-Content "$a\keep.bin" 'keep'
Set-Content "$a\nested\move-me.bin" ('x' * 4096)
Set-Content "$a\nested\change-me.bin" ('y' * 4096)
$bin = (New-Object -ComObject Shell.Application).NameSpace(10)
"recycle_before=$($bin.Items().Count)"
```

| 步骤 | 期望 |
| --- | --- |
| 在 Analyze 里选择上面的 `$a` 目录 | 显示 treemap 和列表 |
| 进入 `nested`，再点 Up | 回到上级目录，不会重新扫描 |
| 选中 `move-me.bin`，点"在资源管理器中显示" | 资源管理器打开并选中该文件 |
| 点"预览移入回收站" | 显示 `sha256:` 摘要 |
| 点确认移入 | 只移动 `move-me.bin`。在 PowerShell 中再次读取 `$bin.Items().Count`，应比之前多 1 |
| 选中 `change-me.bin`，点预览，**然后**在 PowerShell 执行 `Add-Content "$a\nested\change-me.bin" 'more'`，再点确认 | 拒绝执行，文件还在原处 |
| 结束后 | 手动从回收站还原或清空 `move-me.bin` |

### 5.4 Status：进程解释

| 步骤 | 期望 |
| --- | --- |
| 进入 Status，点"刷新快照" | 显示真实数据。缺失的传感器显示"不可用"，不会显示为 0 |
| 点击一个进程行 | 解释在该行下方展开，包含类别、CPU（逻辑核心数）、私有内存占比、磁盘 I/O，以及"DevSweep 不从此列表结束进程" |
| 点击 `explorer.exe` 或 `svchost.exe` | 类别显示为"Windows 共享进程" |
| Tab 到某一行后按 Enter | 打开解释。再按 Enter 关闭 |
| 讲述人朗读 | 能读出解释区域和其中的文字 |
| 在搜索框输入内容，让已选中的行被过滤掉 | 解释移到表格上方 |
| 开始实时采样，然后关闭被选中的那个程序（正常关闭，不要用 PID 结束它） | 解释提示该进程已不在最新采样中 |
| 整个页面 | 没有任何"结束进程"按钮 |

## 6. Software 工作台（最后执行）

前面各节都记录完之后，再开始这一节。

### 6.1 显示核对（使用仓库里的 fixture）

不读主机的软件清单：

```powershell
$fx = (Resolve-Path '.trellis\tasks\10-15-gpui-desktop-rewrite\evidence\fixtures').Path
$s = Join-Path $env:TEMP 'devsweep-gpui-r11-software'
if (Test-Path $s) { Remove-Item $s -Recurse -Force }
New-Item -ItemType Directory -Force "$s\local", "$s\roaming" | Out-Null
Copy-Item "$fx\software-*.json" $s
$before = (Get-FileHash "$s\software-startup.json").Hash
$env:LOCALAPPDATA = "$s\local"; $env:APPDATA = "$s\roaming"
$env:DEVSWEEP_SOFTWARE_INVENTORY = "$s\software-inventory.json"
$env:DEVSWEEP_SOFTWARE_UPDATES_FILE = "$s\software-updates.json"
$env:DEVSWEEP_SOFTWARE_STARTUP_HIVE = "$s\software-startup.json"
& .\desktop-gpui\dist\goal-gpui-release\release\devsweep-gpui-spike.exe
```

| 分段 | 期望 |
| --- | --- |
| 卸载 | 6 行。大小列有 `~… reported`、`… measured`、`≥ … partial`，Python 一行没有大小。"排序：大小"时从大到小，未知大小排最后。计数显示 `6 of 6`。Docker Desktop 不可选。**不要点确认卸载**：这些 fixture 里的包名是假的，清单也已过期 |
| 更新 | 3 行，格式为 `已装 → 可用 · winget`，带"有可用更新"标记 |
| 启动项 | OneDrive 已启用；Teams 已禁用；Defender 和 VPN Client 状态未知，并注明"受保护，DevSweep 不改动"。位置显示为可读名称 |

用英文和中文各看一遍（在设置里切换语言）。
关闭窗口后执行 `(Get-FileHash "$s\software-startup.json").Hash -eq $before`，应为 `True`，表示只看不改时 hive 没有变化。
这个 PowerShell 窗口用完就关掉。

### 6.2 确认流程（复用 r3/r4 脚本的副本）

```powershell
$c12 = '.trellis\tasks\10-15-gpui-software-optimize-workbenches\evidence'
powershell -ExecutionPolicy Bypass -File "$c12\c12-r5-review.ps1"
powershell -ExecutionPolicy Bypass -File "$c12\c12-r6-error.ps1"
```

这两个脚本只用隔离 hive 和一个不存在的更新提供器，不会调用真实卸载。
在输出里核对以下内容：

- `confirmation_required`、`probe_failed`、`stale_confirmation`。过期摘要被拒绝时，hive 不变。
- 实时 `sha256:` 预览，以及 `protected`、`requires_administrator`。
- 再次确认后，只有隔离 hive 文件发生变化。
- Optimize 页的主机行仍为"未运行"，guidance 行显示"不可调度"。
- 中英文两种语言都有记录。

### 6.3 Optimize

主机行必须仍是"未运行"。不要把 C06-AC4 的 `NOT_RUN` 写成成功。

## 7. 回传格式

把下面这段填好发我。另外把各脚本生成的 `.txt` 文件提交到仓库，或直接贴给我。我会据此更新父账本和各任务证据。我不会勾选验收框，也不会归档任务。

```text
HEAD=
binary length= sha256=
cargo test exit=
FocusPattern 类型存在？ 是/否
C11  perf: row_count= interaction_p95= cancel_n= cancel_p95_ms= frame_p95_us= result=
     select-frames: frame_burst_n= p95_us= FRAME_BURST_RESULT=
     focus-probe: distinct_ids= FOCUS_TRAVERSAL=
C13  cycles: clean_exits= forced= leftover_max=
     大快照: 秒数= / NOT_RUN 原因
     OS 崩溃释放: NOT_RUN
C06-AC4: NOT_RUN（无可丢弃卷或映像）
5.1 焦点: PASS/FAIL + 备注
5.2 标题栏: PASS/FAIL + 备注
5.3 Analyze 回收: recycle_before= after= 改写拒绝=是/否
5.4 Status 解释: PASS/FAIL + 备注
6.1 Software 显示 EN: / zh-CN: / hive 未变=
6.2 c12-r5 / c12-r6 关键行:
6.3 Optimize 主机行仍未运行: 是/否
```
