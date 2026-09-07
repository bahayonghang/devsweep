# 盘点

磁盘容量观察使用 `analyze scan`。旧的根命令 `inventory` 会被拒绝（退出码 2）。
该命令不会创建清理目标，输出也不能作为计划传给 `clean`。

```powershell
devsweep analyze scan --root PATH
```

需要供其他工具读取的报告时，使用 `--format json` 和可选的只创建新文件的
`--output`：

```powershell
devsweep analyze scan --root C:\code --format json --output analyze.json
```

已安装软件清单是另一个模式：[`software inventory`](/zh/guide/software)。

## 报告含义

Analyze 会遍历一个根目录，并输出带节点、已计入大小和警告的版本化快照。大小可能是
已验证值、部分下界或未知值；不能把下界当作精确容量数字。该文档没有清理意图，也
没有可执行操作。

需要按照 DevSweep 清理规则发现候选项时使用[扫描](/zh/guide/scan)
（`clean scan`）。目标仅是观察容量时使用 Analyze。
