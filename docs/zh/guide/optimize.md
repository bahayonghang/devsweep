# 优化目录与维护

优化模式展示封闭的八项 Windows 维护目录。它不会发明新操作、不会请求管理员权限，也不会把打开 Windows 设置当作已完成维护。请以标准用户运行。

目录是唯一的渲染来源。每一行都有稳定标识，并使用下列徽章之一：

- **在此运行** — `dns.flush` 在此执行原生 System32 `ipconfig.exe /flushdns`，最长 10 秒。不会拼接其他命令，也不会在 `PATH` 中搜索。
- **打开 Windows 设置** — `settings.storage_recommendations`、`settings.search` 与 `settings.energy_recommendations` 打开固定的设置页面。成功启动记录为 **已启动**，绝不是已完成的优化。DevSweep 不会更改该项设置。
- **仅指引** — `guidance.drive_optimize`、`guidance.system_integrity`、`guidance.filesystem_check` 与 `guidance.network_reset` 仅供显示。它们没有预览或运行操作。

存储与搜索设置行需要 Windows 内部版本 22000 或更高；能源建议需要 22624 或更高。DevSweep 使用核心的类型化内部版本查询（`RtlGetVersion`）。若查询失败，设置行不可用；DevSweep 不会猜测版本。

Optimize 与 Clean 使用同一条授权链：列出目录、为一项精确操作保存计划、预览取得实时摘要，然后 `--confirm`。指引项不能进入计划。目录列表文档不是可运行计划。

```powershell
devsweep optimize list --format json --output optimize-list.json
devsweep optimize plan --operation dns.flush --output dns-plan.json
devsweep optimize preview --plan dns-plan.json
```

运行必须同时提供已保存计划、该次预览的实时 `sha256:` 摘要和 `--confirm`。用 `--help` 核对契约；不要把 `optimize run` 当作文档检查命令：

```powershell
devsweep optimize run --help
devsweep optimize run --plan dns-plan.json --preview-digest sha256:DIGEST --confirm
```

派发前取消、失败、超时以及派发后未知是不同的终态。恢复只读取优化审计日志，不会再次派发。日志只写入 `%LOCALAPPDATA%\DevSweep\audit\v1\optimize.jsonl`。不会使用或导入遗留的 `%APPDATA%\devsweep\audit.jsonl`。

TUI 与桌面端使用相同的阶段性状态：检查、就绪、已选择、预览中、预览就绪、确认中、运行中（仅 DNS）或启动中（仅设置），然后进入终态或未知。底部摘要绝不会把设置启动或指引显示称为已完成的优化。
