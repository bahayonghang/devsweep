# 保护路径

持久保护列表记录 DevSweep 不得清理的现有路径。它是内置保护类别之外的额外保障。

```powershell
# 路径必须已经存在。
cargo run --locked --bin devsweep -- protect add C:\code\keep-me

# 列出受保护路径。
cargo run --locked --bin devsweep -- protect list

# 以后移除条目。
cargo run --locked --bin devsweep -- protect remove C:\code\keep-me
```

在 Windows 上，DevSweep 将列表保存在 `%APPDATA%\devsweep` 下的
`protected-paths.json`。其他受支持平台使用对应的 OS 应用数据目录。

路径会先规范化再比较。即使原始路径已不存在，仍可以移除条目。添加路径时必须存在，
从而使保存的保护条目具有清晰且规范的含义。

如果无法读取保护列表，DevSweep 会拒绝继续，而不会假装没有受保护路径。
