# CI 与发布

## 必需的 CI 作业

| 作业 | 用途 |
| --- | --- |
| Windows、Ubuntu 和 macOS 上的 `rust` | 格式化、锁定依赖检查、锁定依赖测试，以及将警告视为错误的 Clippy。 |
| `docs` | 在 Node 22 下使用仓库根目录的 `package-lock.json` 构建文档站点（`npm ci` 然后 `npm run docs:build`）。 |
| `msrv` | 在 Ubuntu 与 Windows 上使用 `Cargo.toml` 的精确 `package.rust-version` 检查和测试 core/CLI。 |
| `cargo-audit` | RustSec 公告门禁。 |
| `cargo-deny` | 依据 `deny.toml` 检查许可证、来源和公告策略。 |
| `gitleaks` | 密钥扫描。 |

托管 CI 只在 `pull_request` 与 `workflow_dispatch` 时运行。直接推送到 `dev` 或 `main`
不会启动该工作流。

非 Windows Rust 作业和所有 MSRV 作业均显式选择 `-p devsweep-core -p devsweep-cli`。
GPUI 桌面应用（`desktop-gpui/`）是独立工作区，使用本地门禁 `just gpui`，不在托管 CI 中。

所有作业只授予读取仓库内容的权限，使用 cancel-in-progress 并发控制，并配置了时限。

## Node 与 npm

文档站点使用 Node 22。用 `npm ci` 从仓库根目录的 `package-lock.json` 安装锁定依赖。

## 本地 Rust 门禁

```powershell
just ci
```

该配方检查格式、使用 `--locked` 检查所有目标、运行所有测试，并以拒绝警告的方式运行
Clippy。它**不会**更新 `Cargo.lock`。

仅在显式维护时刷新锁文件：

```powershell
just sync-lock
```

## 文档构建

从根目录锁文件安装文档依赖图后构建静态站点。托管 CI 在 Node 22 上运行相同命令；非零状态会
使 `docs` 作业失败。

```powershell
npm ci
npm run docs:build
```

本地编写文档时使用 `just docs`。它只会启动 VitePress 开发服务器，不会发布任何内容。

## 发布归档

`just dev` 以不带额外 argv 的方式运行工作区 CLI 二进制。交互式终端入口是裸的
`devsweep`，并且要求 stdin 与 stdout 都是 TTY。没有 `tui` 子命令。

构建带有可执行文件、`LICENSE`、`README.md` 和 SHA-256 校验文件的主机三元组归档：

```powershell
just release-archive
just release-smoke
```

`just release-smoke` 使用该新归档中的精确 `devsweep.exe`，而不是 `target/release`
里的旧二进制。它会准备一份任务独占的临时项目 fixture，然后仅以
`--scope projects` 运行 `clean scan` → `clean plan` → `clean preview`。它不会运行
`clean execute`、`software uninstall` 或 `optimize run`，也不会用 `--help` 或手拼
空计划代替这些命令。

归档写入 `dist/`，它是生成输出，不应提交。

## MSRV

`Cargo.toml` 的 `rust-version` 是唯一事实来源。若 MSRV 作业的工具链与该值不同，CI 会
失败。
