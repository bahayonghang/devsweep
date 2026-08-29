# 扫描

`scan` 会发现受支持的清理候选项，并输出简要文本摘要或结构化扫描报告。

```powershell
cargo run --locked --bin devsweep -- scan [ROOT]...
```

## 选择范围

未提供范围标志时，DevSweep 会同时包含项目目标和全局提供方。需要缩小范围时可使用
其中一个标志：

```powershell
# 扫描仓库根目录下的项目产物。
cargo run --locked --bin devsweep -- scan --projects C:\code\my-project

# 扫描受支持的全局提供方和缓存。
cargo run --locked --bin devsweep -- scan --global
```

`--projects` 会考虑 `target/`、`node_modules`、虚拟环境和受支持工具缓存等项目级
产物。`--global` 会考虑受支持的包管理器提供方和仅检查位置。当前目录见
[规则参考](/zh/reference/rules)。

## 保存 JSON 以供复核

使用 `--json` 写出完整报告：

```powershell
cargo run --locked --bin devsweep -- scan . --json > plan.json
```

文档包含版本化计划、健康完整性、诊断，以及已验证、部分下界和未知大小目标的独立
汇总。复核后可以交给[`clean --plan`](/zh/guide/clean)。

## 重新估算单个目标

扫描报告某个目标的大小不完整时，可对本次实时扫描中的目标请求更高预算的重新估算：

```powershell
cargo run --locked --bin devsweep -- scan --projects --rescan-target "python.__pycache__:C:/code/app/__pycache__" C:\code
```

目标 ID 必须存在于当前扫描中。该选项不能用来要求 DevSweep 检查任意路径。

## 保守地解读结果

部分下界不是精确大小。诊断会说明跳过、取消或不完整的工作。在决定执行清理前，应先
解决这些不确定性。

## 桌面端实时预览

桌面工作台会在扫描期间显示不确定进度的阶段进度条，并持续展示已发现的目标。此时的
条目按项目与全局范围分类，只是不完整、只读的观察结果，不能被选择或送入试运行。
只有完整扫描报告生成后，选择与清理复核才会启用。扫描被取消或失败时，最新预览会
作为部分证据保留，但不会获得清理权限。
重新扫描停止后，可选择“Return to previous report”返回上一份完整报告，并恢复其默认
复核选择；该操作不会把预览条目提升为清理目标。
