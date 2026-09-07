# 扫描

`clean scan` 会发现受支持的清理候选项，并输出简要文本摘要或结构化观察结果。
观察结果不是可执行计划。

```powershell
devsweep clean scan --root PATH --scope <projects|global|all>
```

`--root` 可重复。默认根目录是 `.`，默认范围是 `all`。

## 选择范围

```powershell
# 扫描仓库根目录下的项目产物。
devsweep clean scan --root C:\code\my-project --scope projects

# 扫描受支持的全局提供方和缓存。
devsweep clean scan --root . --scope global

# 同时扫描两者。
devsweep clean scan --root C:\code\my-project --scope all
```

`--scope projects` 会考虑 `target/`、`node_modules`、虚拟环境和受支持工具缓存等
项目级产物。`--scope global` 会考虑受支持的包管理器提供方和仅检查位置。当前目录见
[规则参考](/zh/reference/rules)。

## 保存 JSON 以供复核

使用 `--format json` 和只创建新文件的 `--output`。不会覆盖已有路径；`-` 不是文件哨兵。

```powershell
devsweep clean scan --root . --scope all --format json --output observation.json
```

机器文档是 V1 信封 `{schema_version,command,outcome,data,warnings,error}`。
`data` 是扫描报告：带嵌入不可信计划、健康完整性、诊断，以及已验证、部分下界和
未知大小目标独立汇总的版本化观察结果。

该文件适合作为 [`clean plan`](/zh/guide/clean) 的 `--observation`。它**不是**
可运行计划。不要把它传给 `clean preview` 或 `clean execute`。

## 重新估算单个目标

扫描报告某个目标的大小不完整时，可对本次实时扫描中的目标请求更高预算的重新估算。
重新估算需要恰好一个 `--root`，以及包含项目的范围：

```powershell
devsweep clean scan --root C:\code --scope projects --rescan-target "python.__pycache__:C:/code/app/__pycache__"
```

目标 ID 必须存在于当前扫描中。该选项不能用来要求 DevSweep 检查任意路径。

## 保守地解读结果

部分下界不是精确大小。诊断会说明跳过、取消或不完整的工作。在把目标选入新计划前，
应先解决这些不确定性。

## 桌面端实时预览

桌面工作台会在扫描期间显示不确定进度的阶段进度条，并持续展示已发现的目标。此时的
条目按项目与全局范围分类，只是不完整、只读的观察结果，不能被选择或送入预览。
只有完整扫描报告生成后，选择与清理复核才会启用。扫描被取消或失败时，最新预览会
作为部分证据保留，但不会获得清理权限。
重新扫描停止后，可选择“Return to previous report”返回上一份完整报告，并恢复其默认
复核选择；该操作不会把预览条目提升为清理目标。
