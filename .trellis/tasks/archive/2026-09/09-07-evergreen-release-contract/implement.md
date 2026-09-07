# P1 修复开发发布入口和版本一致性 — Implementation plan

## Approval and sequence

在 validation-gates 后实施，避免 justfile 重叠；CLI 文档独立修改，但最终冒烟与文档示例必须互相核对。

1. 取得本父子任务最新方案的明确实施批准后，复核 Git/任务状态与本任务证据；仅激活本子任务。
2. 强模型确认 `design.md` 文件范围、事实和改造机制，分配独占文件责任。
3. 先保存最小失败证据/反例，再完成设计中的最小改动；不扩大权限或加入兼容旧根。
4. 按下面命令运行适用检查，保留命令、环境、退出码和未验证项。
5. 强模型独立核验每条 AC 的每个子句和回写；若低成本模型遇到跨模块语义/权限/两次失败，升级给主审，不继续堆修补。
6. 形成验收记录供父任务汇总。本方案不授权 push、安装应用、签名、发布或额外知识库写入。

## Required checks

- python -m unittest discover -s tools/tests -p test_release_contract.py
- cargo test --locked -p devsweep-cli --test cli_contract --test five_mode_contract
- just release-archive；just release-smoke（Windows，task-owned fixture，记录产物哈希）
- Node 22/npm 10–11：npm --prefix desktop run build；npm --prefix desktop run tauri -- build
- 验证 NSIS artifact metadata，不运行 tinstall、install-all 或任何安装器
- 取得本次唯一 NSIS 路径后：`$v = [Diagnostics.FileVersionInfo]::GetVersionInfo($installerPath)`；输出八个 File/Product 版本整数、原版本字符串、`Get-FileHash -Algorithm SHA256 -LiteralPath $installerPath`。按 design 的源版本整数比较判定；缺失/不一致非零，不运行安装器。

## Final evidence

本任务修改 justfile，因此还必须满足父 implement.md 中从 backend CI/release 场景继承的最终共享门禁：clean/custom --target-dir workspace 测试、Windows/Unix process-runner（Unix 动态树终止）、生成 dist ignored 验证。父集成在最后一次 justfile 改动后集中运行一次；该证据齐全前不把本任务完整完成/归档。测试进程不继承 CARGO_TARGET_DIR，不以当前基线 hosted PASS 替代新代码证据。

记录 AC → 文件/行为 → 命令/输出映射。通过结构校验不等于实现验收；对 native/provider 等未运行项保留 UNVERIFIED，不能靠删除或弱化 AC 过关。
