# CI 与发布

## 必需的 CI 作业

| 作业 | 用途 |
| --- | --- |
| Windows、Ubuntu 和 macOS 上的 `rust` | 格式化、锁定依赖检查、锁定依赖测试，以及将警告视为错误的 Clippy。 |
| `msrv` | 在 Ubuntu 与 Windows 上使用 `Cargo.toml` 的精确 `package.rust-version` 进行检查和测试。 |
| `cargo-audit` | RustSec 公告门禁。 |
| `cargo-deny` | 依据 `deny.toml` 检查许可证、来源和公告策略。 |
| `gitleaks` | 密钥扫描。 |

所有作业只授予读取仓库内容的权限，使用 cancel-in-progress 并发控制，并配置了时限。

## 本地 Rust 门禁

```powershell
just ci
```

该配方检查格式、同步本地 Cargo lock 状态、检查所有目标、运行所有测试，并以拒绝警告的
方式运行 Clippy。

## 文档构建

安装文档依赖图后构建静态站点：

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
