# 快速开始

DevSweep 是一个用于规划和执行开发工作区清理的 Rust 命令行程序。它以可复核的
计划为中心，而不是隐式删除文件。

## 前置条件

- 从本仓库构建需要 Rust 1.88.0 或更高版本。
- 仅在启动或构建本文档站点，或开发桌面应用时需要 Node.js（桌面端需要
  Node.js 22）。

本仓库同时包含主程序和一个名为 `process_fixture` 的测试二进制目标。因此使用
`cargo run` 时必须显式选择 `devsweep`。

## 构建并查看命令

```powershell
just build
cargo run --locked --bin devsweep -- --help
```

已交付的根命令是 `clean`、`software`、`optimize`、`analyze`、`status` 和
`history`。没有根级 `tui`、`scan`、`inventory`、`protect` 或 `rules` 命令。
参数详情见[命令行参考](/zh/reference/cli)。若仍记得旧调用，请看带标注的
[CLI 迁移表](/guide/cli-migration)；那不是现行教程。

## 创建第一个计划

先扫描并保存观察结果，再按精确目标 ID 写出一份**新**计划：

```powershell
cargo run --locked --bin devsweep -- clean scan --root . --scope all --format json --output observation.json
```

从该 V1 信封中的 `data.plan.targets[].id` 读取目标 ID，然后：

```powershell
cargo run --locked --bin devsweep -- clean plan --observation observation.json --select TARGET_ID --output plan.json
```

观察文件是扫描报告（通常包在机器信封里）。它**不是**可执行计划。
`clean preview` 和 `clean execute` 需要这份新计划文件。在编辑或复用前，请阅读
[计划与报告](/zh/reference/plan-and-report)。

## 先预览再执行

预览就是演练。它会计算实时 `sha256:` 摘要，且不会执行清理：

```powershell
cargo run --locked --bin devsweep -- clean preview --plan plan.json
```

仅在该实时摘要与 `--confirm` 同时具备时才执行。用 `--help` 核对契约；不要把
execute 当作首次运行命令：

```powershell
cargo run --locked --bin devsweep -- clean execute --help
```

```powershell
cargo run --locked --bin devsweep -- clean execute --plan plan.json --preview-digest sha256:DIGEST --confirm
```

没有 `--execute` 参数，也没有 `--audit-log` 路径。永久删除、Docker 清理和
Cargo home 清理保持关闭。

## 打开交互界面

终端界面是在交互式 stdin/stdout TTY 上直接运行 `devsweep`：

```powershell
cargo run --locked --bin devsweep
```

`devsweep tui` 会被拒绝（退出码 2）。`just dev` 目前仍会传入 `tui`；该配方属于
后续 release-contract 修复，不是现行教程。

```powershell
just docs
```

`just docs` 启动本文档站点；若尚未安装文档依赖，请先运行 `npm ci`。

```powershell
just tdev
```

`just tdev` 从仓库根目录启动 Tauri 桌面开发窗口。请先在 `desktop/` 下用
`npm ci` 安装依赖（需要 Node.js 22）。
