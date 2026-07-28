# CI 强化与可信发布链

> 父任务：07-28-audit-remediation ｜ 审计条目：F-23、发布配方（审计 §8.5） ｜ 对应审计 Milestone M4
> 二轮评审 CORR-002/ARCH-004 拆分：F-10 许可证已拆出为 P1 的 07-28-license-baseline；本任务只含 CI 与发布工件。

## Goal

强化 CI（三平台、预算、最小权限、锁定依赖、安全扫描、MSRV）并修正发布配方的目标命名与校验和，使 release 工件可信可验证。

## 背景与证据（当前 HEAD）

1. **CI 缺口（F-23）**：`.github/workflows/ci.yml` 仅 windows/ubuntu；无 `timeout-minutes`、`concurrency`、`permissions: contents: read`；cargo 命令未加 `--locked`；`actions/checkout@v4` 未 pin commit SHA；无 `cargo audit`/`cargo deny`/secret scan/coverage；无 MSRV 验证。
2. **发布配方错标（§8.5）**：`justfile:20-25` 在 host 上 `cargo build --release` 却固定命名 `devsweep-x86_64-pc-windows-msvc.zip`——ARM64 Windows 错标，非 Windows 无 `.exe` 可打包；无 checksum。

## Requirements

1. CI 强化（最低集）：全部 cargo 命令 `--locked`；`permissions: contents: read`；`timeout-minutes`；`concurrency` + cancel-in-progress；Actions pin 完整 commit SHA；矩阵加入 `macos-latest`；MSRV job（固定版本 + stable 双轨，MSRV 值与 license-baseline 写入 Cargo.toml 的 `rust-version` 一致）。
2. CI 安全 gate：`cargo audit`（或等价 RustSec gate）与 `cargo deny`（advisory/license/source policy）；secret scan（gitleaks 或等价）；豁免须有记录。
3. 发布配方：产物命名取自实际构建 target triple；附 SHA-256；archive 内含 LICENSE + README；解压 smoke（`--version`、`scan --json`、dry-run）。
4. 安全回归套件（前置 P1 子任务交付物）纳入 required checks 的清单写入 CI 文档；SBOM/签名/provenance 记为 stretch，不阻断本任务。

## Acceptance Criteria

- [ ] CI 在 windows/ubuntu/macos 三平台绿色；所有 job 有 timeout 与最小权限；Actions 均为 SHA pin
- [ ] Cargo.lock 漂移会使 CI 失败（`--locked` 生效的反向验证）
- [ ] `cargo audit` 与 `cargo deny` 独立 job 运行并通过（或有记录的豁免清单）
- [ ] release recipe 产物名与实际 target triple 一致并附 SHA-256；非 x86_64 Windows 构建不再错标
- [ ] MSRV 在 CI 验证且与 `Cargo.toml` `rust-version` 一致
- [ ] 解压 smoke 脚本可本地复跑

## 约束与依赖

- 最后收口：required checks 的收紧应在前置 P1 子任务的安全测试套件落地后进行，避免先造空门。
- 依赖 07-28-license-baseline 先落 `rust-version` 与 LICENSE（archive 内容引用）。
- macOS CI 只验证编译与单元测试；真实 Trash/APFS 行为仍标注"需动态验证"。
- 不做 winget/scoop/Homebrew 分发与自更新（父任务范围外）。
