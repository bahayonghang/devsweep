# CI 与发布

## 必需的 CI 作业

| 作业 | 用途 |
| --- | --- |
| Windows、Ubuntu 和 macOS 上的 `rust` | 格式化、锁定依赖检查、锁定依赖测试，以及将警告视为错误的 Clippy。 |
| Windows 上的 `desktop` | 在 Node 22 下将桌面前端的类型漂移检查、lint、typecheck、test 和 Vite build 拆成独立步骤，然后进行未签名的 Tauri 编译（`tauri build --no-bundle`）。 |
| `docs` | 在 Node 22 下使用仓库根目录的 `package-lock.json` 构建文档站点（`npm ci` 然后 `npm run docs:build`）。 |
| `msrv` | 在 Ubuntu 与 Windows 上使用 `Cargo.toml` 的精确 `package.rust-version` 进行检查和测试。 |
| `cargo-audit` | RustSec 公告门禁。 |
| `cargo-deny` | 依据 `deny.toml` 检查许可证、来源和公告策略。 |
| `gitleaks` | 密钥扫描。 |

所有作业只授予读取仓库内容的权限，使用 cancel-in-progress 并发控制，并配置了时限。托管
Desktop 前端检查按每个 npm 脚本拆成独立步骤，避免原生非零退出被后续命令掩盖。类型漂移
在任何生成写入之前比较。

## Node 与 npm

桌面应用在 `desktop/package.json` 中声明 `engines.node` 为 `>=22 <23`，`engines.npm` 为
`>=10 <12`（npm 10 或 11）。托管的 `desktop` 与 `docs` 作业使用 Node 22。本地桌面门禁必须
使用同一 Node 22 以及 npm 10 或 11 范围；更新的系统 Node/npm（例如 Node 26 / npm 12）不在
受支持的 engines 范围内。

用 `npm ci` 安装锁定依赖：

- 文档：仓库根目录的 `package-lock.json`。
- 桌面：`desktop/package-lock.json`。

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

## 本地桌面门禁

```powershell
just desktop-web-check
```

该配方在 `desktop/` 下按顺序运行这些具名 npm 脚本：`types:generate -- --check`、`lint`、
`typecheck`、`test` 和 `build`。每次调用都会检查原生命令退出码；任一非零状态都会使入口失败。

在 Windows PowerShell 中，用 `;` 串联原生命令时，前序失败不会停止后续命令。因此该配方在每次
npm 调用后检查 `$LASTEXITCODE`，而不是依赖最后一条命令的退出码。

`types:generate --check` 会将已提交的 `desktop/src/api/types.gen.ts` 与生成器的 `--stdout`
结果比较，并且不会写入。不带旗标的 `npm run types:generate` 仍会写入，这是显式维护路径。

## 文档构建

从根目录锁文件安装文档依赖图后构建静态站点。托管 CI 在 Node 22 上运行相同命令；非零状态会
使 `docs` 作业失败。

```powershell
npm ci
npm run docs:build
```

本地编写文档时使用 `just docs`。它只会启动 VitePress 开发服务器，不会发布任何内容。

## 发布归档

构建带有可执行文件、`LICENSE`、`README.md` 和 SHA-256 校验文件的主机三元组归档：

```powershell
just release-archive
just release-smoke
```

归档写入 `dist/`，它是生成输出，不应提交。

## MSRV

`Cargo.toml` 的 `rust-version` 是唯一事实来源。若 MSRV 作业的工具链与该值不同，CI 会
失败。
