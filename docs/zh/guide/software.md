# 软件清单与卸载

软件模式在不把显示名称、发布者文本、注册表卸载字符串或安装路径提升为清理权限的前提下，收集已安装软件证据。请以标准用户运行；此流程不会请求管理员权限。

Software V1 仅允许选择精确标识、状态正常的当前用户 MSIX。所有 MSI 与 ARP 条目仍可查看，但必须手动处理，其中包括计算机级和用户级 MSI 上下文。每一行都会明确标注“报告估计”“实测安装位置”“部分下限”或“未知”大小。由于 V1 没有受支持的精确来源，最近使用时间始终显示为未知。

Software 与 Clean 使用同一条授权链：

保存清单 → 精确选择 → 保存**新**计划 → 实时预览摘要 → `--confirm`。

清单 JSON 不是可运行计划。`software preview` 和 `software uninstall` 需要这份新计划文件。

```powershell
devsweep software inventory --source all --format json --output inventory.json
devsweep software plan --inventory inventory.json --select SOFTWARE_ID --output software-plan.json
devsweep software preview --plan software-plan.json
```

从 V1 清单信封中的 `data.entries[].id` 读取 `SOFTWARE_ID`。只有符合条件的当前用户 MSIX 标识可以进入计划。

卸载不可逆。DevSweep 无法还原或重新安装已移除的软件。清单中的大小是证据，不代表卸载后必然释放同等容量。用 `--help` 核对契约；不要把 uninstall 当作首次运行或文档检查命令：

```powershell
devsweep software uninstall --help
devsweep software uninstall --plan software-plan.json --preview-digest sha256:DIGEST --confirm
```

每个精确标识只会得到一个封闭终态：已移除、需要重启、仍存在、失败或分派后未知；分派前取消单独记录。应用重启后会根据软件审计日志重新查询中断的分派，不会再次执行移除。日志只写入 `%LOCALAPPDATA%\DevSweep\audit\v1\software.jsonl`。不会使用或导入遗留的 `%APPDATA%\devsweep\audit.jsonl`。

在原生构建中验证卸载前，必须使用由用户确认的可丢弃 MSIX 目标。清单与预览是只读操作；不得随意选择已安装软件作为卸载测试目标。
