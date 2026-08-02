# 快速开始

DevSweep 是一个用于规划和执行开发工作区清理的 Rust 命令行程序。它以可复核的
计划为中心，而不是隐式删除文件。

## 前置条件

- 从本仓库构建需要 Rust 1.88.0 或更高版本。
- 仅在启动或构建本文档站点时需要 Node.js 18 或更高版本。

本仓库同时包含主程序和一个名为 `process_fixture` 的测试二进制目标。因此使用
`cargo run` 时必须显式选择 `devsweep`。

## 构建并查看命令

```powershell
just build
cargo run --locked --bin devsweep -- --help
```

公开命令包括 `tui`、`scan`、`inventory`、`clean`、`protect` 和 `rules`。
参数详情见[命令行参考](/zh/reference/cli)。

## 创建第一个计划

扫描当前项目，并把报告保存为 JSON 以便复核：

```powershell
cargo run --locked --bin devsweep -- scan . --json > plan.json
```

输出是同时包含清理计划和扫描健康状态的扫描报告。在编辑或复用前，请阅读
[计划与报告](/zh/reference/plan-and-report)。

## 先演练执行

先在无副作用模式下运行保存的计划：

```powershell
cargo run --locked --bin devsweep -- clean --plan plan.json
```

仅在复核后才请求执行：

```powershell
cargo run --locked --bin devsweep -- clean --plan plan.json --execute --audit-log devsweep-audit.jsonl
```

`--execute` 必须明确添加。它不会启用永久删除、Docker 清理或 Cargo home 清理。

## 打开交互界面

```powershell
just dev
just docs
```

`just dev` 启动终端界面。`just docs` 启动本文档站点；若尚未安装文档依赖，请先运行
`npm ci`。
