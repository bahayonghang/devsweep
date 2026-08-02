# 规则

在本机运行以下命令可查看当前目录：

```powershell
cargo run --locked --bin devsweep -- rules
```

命令会输出规则 ID、风险、操作类型和简要说明。它是当前安装二进制的权威规则列表。

## 项目规则

当前项目级规则覆盖 Rust `target/`、Node `node_modules` 与常见构建缓存、Python 虚拟
环境和工具缓存，以及 Python `__pycache__` 目录。部分项目产物通过回收站清理；Rust
`target/` 会在验证相关 Cargo metadata 和项目 marker 后使用 `cargo clean`。

当前 ID 示例：

```text
rust.target
node.node_modules
node.next_cache
node.turbo
python.venv_dot
python.pytest_cache
python.__pycache__
```

## 全局提供方与缓存

受支持的全局提供方规则包括 npm、pip、pnpm 和 Yarn 缓存命令，以及部分 Go、Gradle、
Maven、Ivy、NuGet、JetBrains 和 Hugging Face 缓存。基于命令的提供方使用注册表拥有的
固定参数模板，而不是保存计划中的字符串。

部分条目有意设置为仅检查，包括 Cargo home、Maven 本地仓库、直接检查 Go module cache
的条目、JetBrains 缓存根和 Hugging Face hub 模型。单独的 `go.mod_cache.clean` 提供方
规则会使用 `go clean -modcache`。Docker 列为延后处理，而不是当前清理目标。

## 风险与操作标签

风险级别有助于安排复核优先级，但不能替代明确的执行决定。`inspect only` 或 `deferred`
规则不具备清理执行资格。升级 DevSweep 后应再次检查 `rules`，因为规则目录属于该版本
程序行为的一部分。
