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

Windows 全局扫描还会检查受支持默认位置中的 Chrome、Edge、Firefox、VS Code、Cursor
缓存，以及当前用户临时目录内符合条件的独立旧条目。新增候选项默认不勾选。应用正在
使用缓存或无法确认归属时，会显示不可清理的原因；浏览器档案数据、编辑器状态和整个
临时目录不属于清理目标。此范围不代表支持任意自定义应用数据目录。

## 选择桌面端文件夹

在“清理”的“项目文件夹”中添加现有文件夹的绝对路径，或选择建议的现有文件夹。
可以选择不同磁盘上的目录。桌面端会记住经过验证的选择，并在扫描前显示；不会隐式
使用应用启动目录。启用“项目”时至少选择一个文件夹；关闭“项目”可仅检查全局缓存。

重复选择会去重。如果上级目录仍被选中，移除某个子目录的选择并不会把它排除在扫描
范围外。清理路径的保护需通过[保护](/zh/guide/protection)单独设置。设置无法保存时，
界面会说明经过验证的选择仅在本次会话有效。桌面端选择不会改变 CLI 的 `--root` 默认值。

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

覆盖记录会列出已检查的根目录或提供方，并区分结果为空、工具缺失、配置不受支持、
应用占用、失败、超时和受预算限制的未完成扫描。某个根目录失败不会抹去其他目录的
有效结果。CLI 对部分结果使用部分完成状态及对应退出码；覆盖检查失败时，零目标
不代表所选位置无需清理。旧观察文件若没有覆盖记录，需要重新扫描才能获得这些信息。

桌面复核中的“重新检查大小”会以更高的计量预算检查已有目标，并使旧预览失效。
它不会将“仅检查”目标升级为可执行目标。“分析文件夹”会打开独立的只读分析流程。
结果表格采用分页显示，搜索和选择仍覆盖完整报告。

## 桌面端实时预览

桌面工作台会在扫描期间显示不确定进度的阶段进度条，并持续展示已发现的目标。此时的
条目按项目与全局范围分类，只是不完整、只读的观察结果，不能被选择或送入预览。
只有完整扫描报告生成后，选择与清理复核才会启用。扫描被取消或失败时，最新预览会
作为部分证据保留，但不会获得清理权限。
重新扫描停止后，可选择“Return to previous report”返回上一份完整报告，并恢复其默认
复核选择；该操作不会把预览条目提升为清理目标。
