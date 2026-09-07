# 保护路径

持久保护列表记录 DevSweep 不得清理的现有路径。它是内置保护类别之外的额外保障。
旧的根命令 `protect` 会被拒绝；该命令嵌套在 Clean 下。

```powershell
# 路径必须已经存在。变更必须同时提供 --path 和 --confirm。
devsweep clean protect add --path C:\code\keep-me --confirm

# 列出受保护路径。
devsweep clean protect list

# 以后移除条目。
devsweep clean protect remove --path C:\code\keep-me --confirm
```

在 Windows 上，DevSweep 将列表保存在 `%APPDATA%\devsweep` 下的
`protected-paths.json`。其他受支持平台使用对应的 OS 应用数据目录。变更还会向
`%LOCALAPPDATA%\DevSweep\audit\v1\clean.jsonl` 追加经过脱敏的
`protection_mutation` 记录。载荷只保存 SHA-256 身份，不含原始路径。

路径会先规范化再比较。即使原始路径已不存在，仍可以移除条目。添加路径时必须存在，
从而使保存的保护条目具有清晰且规范的含义。

如果无法读取保护列表，DevSweep 会拒绝继续，而不会假装没有受保护路径。
