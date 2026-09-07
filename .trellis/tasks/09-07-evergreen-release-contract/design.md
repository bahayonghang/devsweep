# P1 修复开发发布入口和版本一致性 — Design

## Evidence and scope

参见父任务 `research/audit-report.md`、`research/test-results.md` 和 `research/harness-matrix.md`。报告中的事实对应审查基线，实施前复核变动。

## Owning files

- justfile（dev、release-archive、release-smoke；顺序接续 gate 子任务）
- desktop/package.json、desktop/package-lock.json（只根包版本元数据）
- desktop/src-tauri/tauri.conf.json（version）
- tools/tests/test_release_contract.py（新增，命令/版本/产物契约的标准库检查）
- docs/ci.md、docs/zh/ci.md（release archive 段：bare `just dev` 与 scan→plan→preview 冒烟）

## Mechanism

保留 Cargo.toml workspace.package.version=0.3.0 为当前版本事实；同步桌面元数据而不引入迁移。dev 调用 bare binary 的交互入口。release-smoke 使用新归档内的精确 devsweep.exe，准备任务独占项目 fixture，走 clean scan→plan→preview，无全局 provider 扫描和真实清理。不要只把旧命令改成 --help 以绕过功能冒烟；也不沿用手拼旧 empty-plan 来代替真实计划生成。版本检查在一处比较四份元数据，不加通用发布框架。

## NSIS artifact version measurement

AC4 的观测对象是本次 `npm --prefix desktop run tauri -- build` 生成的精确 NSIS `*-setup.exe` 的 PE VERSIONINFO。构建前记录开始时间及现有产物路径/SHA256；构建后按配置的 productName、workspace version、目标架构确定唯一产物，并记录绝对路径、时间、大小和 SHA256。缺失/多个候选均不得仅取 LastWriteTime 最新者过关。

用 `[System.Diagnostics.FileVersionInfo]::GetVersionInfo($installerPath)` 读取 `FileMajorPart/FileMinorPart/FileBuildPart/FilePrivatePart` 与 `ProductMajorPart/ProductMinorPart/ProductBuildPart/ProductPrivatePart`。两组整数均应等于当前 Cargo workspace 0.3.0 对应的 `[0,3,0,0]`；预期由 Cargo.toml 的三个版本整数加 Windows build 位 0 得到，后续版本变更时从同一源重算。同时记录 `FileVersion`/`ProductVersion` 原字符串以便审查。版本资源缺失或任一分量不符即失败；文件名、构建日志不能单独替代该判据。不执行安装器。

版本回归的变异对照在临时复制的元数据中把一份根版本改为 0.3.1，版本检查须非零；真实产物由上述 PE 检查覆盖。

## Ordering and shared ownership

在 validation-gates 后实施，避免 justfile 重叠；CLI 文档独立修改，但最终冒烟与文档示例必须互相核对。

## Tool and model assignment

Codex 强模型负责 Windows 产物/fixture 边界与最终验收；便宜模型可同步版本和已确定的命令替换。shell 临时路径、模拟文件和真实产物选择由强模型复核。

## Knowledge writeback

发布检查与修复结果回写 docs/ci.md 的 release 段（在 gate 子任务结束后）和任务验收；适用五套 harness，打包限 Windows。

## Rollback

保留实施前差异和生成文件哈希；仅回退本任务负责的变更，不恢复其他任务文件。若验证揭示新产品语义选择，暂停该依赖项并回到父任务修订批准范围。
