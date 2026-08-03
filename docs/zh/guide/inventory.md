# 盘点

`inventory` 会把根目录的直接内容测量为只读容量观察值。它不会创建清理目标，输出也
不能作为计划传给 `clean`。

```powershell
cargo run --locked --bin devsweep -- inventory [ROOT]
```

需要供其他工具读取的报告时，使用 `--json`：

```powershell
cargo run --locked --bin devsweep -- inventory C:\code --json > inventory.json
```

## 报告含义

每条观察记录都有路径、估算大小和完整性标识。报告还包含与扫描类似的健康诊断和汇总。
大小可能是已验证值、部分下界或未知值；不能把下界当作精确容量数字。

对于 pnpm，盘点可以显示仅检查的孤立 store 发现。这是容量观察，而不是清理该 store
的授权。

需要按照 DevSweep 清理规则发现候选项时使用[扫描](/zh/guide/scan)。目标仅是观察容量
时使用盘点。
