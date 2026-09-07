# 计划与报告

DevSweep 使用版本化 JSON 文档，使扫描结果可以先被复核，再写成一份**新**的保存
计划。观察 JSON 不是可执行权限。

## 扫描观察

`clean scan --format json` 输出 V1 机器信封：

- `schema_version`、`command`（`clean.scan`）、`outcome`、`warnings`、`error`；
- `data`：扫描报告。

扫描报告包含：

- `version`：扫描报告版本；
- `plan`：嵌入的不可信清理计划文档；
- `health`：完整性、诊断和汇总。

健康汇总区分已验证字节数、部分下界和未知目标数量。不完整的大小估算代表不确定性，
不能据此假设精确大小。

把该文件交给 `clean plan --observation`。不要把它传给 `clean preview` 或
`clean execute`。

## 清理计划

`clean plan --observation FILE --select TARGET_ID --output FILE` 会写出一份新的
计划文件（不是信封）。计划使用版本 `2`，包含已选目标列表。每个目标记录 ID、范围、
生态、类别、路径、估算大小、大小完整性、最后修改时间、风险、可逆性、默认选择、
证据和清理意图。

证据可以标识 marker file、已知缓存目录、规则匹配或其他观察结果，从而说明
DevSweep 为何将该路径列为候选项。清理意图说明文档声称的、由规则定义的工作类别，
例如移入回收站的项目产物，或由注册表所有的提供方命令。验证会将该声明与内置规则
目录进行核对。

## 执行前验证

JSON 文档不是 shell 脚本。`clean preview` 和 `clean execute` 将保存的计划解码为
不可信输入，强制检查受支持版本和严格字段，随后逐一验证目标，并按注册规则重建
可执行操作。执行还需要匹配的实时预览摘要和 `--confirm`。程序在副作用发生前还会
再次进行实时授权检查。

该设计避免被修改的计划文件把 DevSweep 变成任意命令执行器。

## Analyze 报告不同

`analyze scan --format json` 输出只读磁盘快照。由于该文档没有清理计划，`clean`
不会把它当作观察结果或计划。使用 Analyze 了解容量，使用 `clean scan` 创建可复核
的清理观察。

`software inventory --format json` 同样不是 Clean 计划。软件选择使用
`software plan --inventory FILE --select SOFTWARE_ID --output FILE`。

## 保持文档最新

不要把旧 JSON 文件视为永久批准。目标集合、项目身份、诊断健康状态或大小估算改变
后，应重新扫描。v1 计划或把观察结果误当作计划的旧文件，必须由新的 `clean scan`
观察结果和新的 `clean plan` 输出替换。
