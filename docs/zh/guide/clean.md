# 清理已保存计划

`clean` 使用已保存的清理计划或扫描报告。它会在执行前验证文档，并使用计划中默认选中
的目标 ID。

```powershell
cargo run --locked --bin devsweep -- clean --plan plan.json
```

## 先演练

上面的命令是演练执行。它会报告选中目标的数量，不会调用清理命令，也不会把路径移入
回收站。

计划过旧、项目发生变化或之前扫描报告不完整健康状态时，应再次演练。计划是复核产物，
不是永久授权令牌。

## 仅在复核后执行

执行同时需要计划路径和明确标志：

```powershell
cargo run --locked --bin devsweep -- clean --plan plan.json --execute --audit-log devsweep-audit.jsonl
```

DevSweep 会把 JSONL 审计记录写入指定路径。执行时如果省略 `--audit-log`，默认使用
`devsweep-audit.jsonl`。

## 验证边界

- 计划格式 v1 会被拒绝；请重新运行 `devsweep scan --json`。
- 盘点报告会被拒绝，因为它只有观察值，没有清理授权。
- 不受支持的报告版本和未知字段会被拒绝。
- 序列化可执行命令字段不会被信任。有效操作由内置规则目录重建。
- 仅检查、已不再获得授权，或已不符合预期身份的目标不可执行。

## 没有永久删除开关

没有任何命令行选项可重新启用永久删除。Docker 和 Cargo home 仍不属于可执行清理行为。
