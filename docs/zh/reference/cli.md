# 命令行参考

从本仓库运行主程序时必须显式选择它：

```powershell
cargo run --locked --bin devsweep -- <COMMAND>
```

## 命令

| 命令 | 用途 | 主要选项 |
| --- | --- | --- |
| `tui` | 打开交互式终端界面。 | 无。 |
| `scan [ROOT]...` | 发现清理候选项并输出报告。 | `--json`、`--global`、`--projects`、`--rescan-target <TARGET_ID>`。 |
| `inventory [ROOT]` | 仅检查容量，不创建清理目标。 | `--json`。 |
| `clean` | 演练或明确执行保存的计划/报告。 | `--plan <PATH>`、`--execute`、`--audit-log <PATH>`。 |
| `protect add <PATH>` | 将现有路径加入持久保护列表。 | 无。 |
| `protect remove <PATH>` | 从该列表移除路径。 | 无。 |
| `protect list` | 列出受保护路径。 | 无。 |
| `rules` | 打印当前项目和全局规则目录。 | 无。 |

## 全局选项

`--help` 输出命令帮助。`--version` 输出 DevSweep 版本。

## `scan` 的范围行为

未提供 `--global` 和 `--projects` 时，`scan` 会同时包含两者。仅提供 `--global`
会排除项目扫描；仅提供 `--projects` 会排除全局提供方扫描。

## 示例

```powershell
# 在扫描前检查已知清理规则。
cargo run --locked --bin devsweep -- rules

# 保存仅全局范围的 JSON 扫描报告。
cargo run --locked --bin devsweep -- scan --global --json > global-scan.json

# 演练已复核的计划。
cargo run --locked --bin devsweep -- clean --plan global-scan.json
```

`clean` 可接受的文档格式见[计划与报告](/zh/reference/plan-and-report)。
