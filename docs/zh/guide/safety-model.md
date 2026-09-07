# 安全模型

DevSweep 将发现、计划、验证和副作用分开处理。这是产品边界，而不是便捷功能。

## 先扫描

`clean scan` 会识别符合条件的项目产物和受支持的全局提供方。它在版本化观察结果中
记录证据、风险级别、清理意图、大小信息和健康诊断。扫描不会执行清理操作。

用 `--format json --output FILE` 保存观察结果。该 JSON 不是可运行计划。

## 将保存的文档视为不可信输入

`clean plan --observation FILE --select TARGET_ID --output FILE` 读取观察结果，
只保留精确选中的标识，并写出一份**新**的版本化计划。`clean preview --plan FILE`
和 `clean execute --plan FILE` 将该保存计划解码为不可信输入，验证版本和结构，
并依据 DevSweep 受信任的规则目录重建可执行操作。序列化计划不能选择任意程序或
shell 命令。详见[计划与报告](/zh/reference/plan-and-report)。

## 预览就是演练

`clean preview` 会报告选中的目标、计算实时 `sha256:` 摘要，并且不执行清理。
请利用该步骤确认保存的计划仍符合目标范围。不存在带 `--execute` 的演练模式。

## 执行必须明确请求

执行必须同时提供已保存计划、匹配的实时预览摘要（`sha256:` 加 64 位小写十六进制）
和 `--confirm`。项目产物通常通过回收站清理；Rust `target/` 则在适用 manifest 下
使用 `cargo clean`。基于命令的全局清理会保持程序名与 argv 分离，不会拼接 shell
命令字符串。

请求执行时，DevSweep 会在副作用发生前重新检查已选目标和授权范围。它只向固定的
Clean V1 存储 `%LOCALAPPDATA%\DevSweep\audit\v1\clean.jsonl` 追加 JSONL 审计记录。
没有 `--audit-log` 参数。遗留的 `%APPDATA%\devsweep\audit.jsonl` 不会被打开、
导入、转换或搜索。

## 明确的非目标

- 此构建中永久删除已禁用，且没有对应 CLI 选项。
- Docker 清理尚未实现，不会被扫描或执行。
- Cargo home 仅供检查。凭据、已安装二进制、registry 内部文件和 Git 缓存内部文件
  永远不会成为清理操作。
- 符号链接目录和 Windows reparse point 受到保护，不会作为清理目标被跟随。

## 保护自己拥有的路径

使用 [`clean protect`](/zh/guide/protection) 把现有路径加入持久保护列表。变更
必须同时提供 `--path` 和 `--confirm`。保护列表读取失败时会拒绝继续，而不会静默
退化为空列表。
