# 实施计划

开始前读：`.trellis/spec/backend/index.md`、`scan-coverage.md`、`provider-authority.md`、`clean-advisories.md`；本任务 `design.md`。

## 步骤

1. [x] 调研并写 `research/families.md`：每个候选家族的目录证据、保留目录、所有者进程名。可参考 `ref/Mole` 的 `origin/windows` 分支 `lib/clean/caches.ps1` 与 `cache_guard.ps1` 作为线索，但以实测和厂商文档为准。
2. [x] **规划关口**：把候选清单和证据交给用户，确定纳入的家族，更新 `prd.md`。
3. [x] 评估现有 Chromium 与 VS Code 系实现的参数化；必要时先做不改行为的重构，测试不变。
4. [x] 逐个家族加规则与夹具测试。
   回滚点：每个家族一个提交单位，可单独撤回。
5. [x] `clean rules` 的中英文说明与 CLI 契约测试。
6. [x] 更新 `.trellis/spec/backend/scan-coverage.md` 的家族清单。

## 验证

```powershell
cargo test --locked -p devsweep-core rules
cargo test --locked -p devsweep-cli --test cli_contract
just ci
```

- 只在夹具目录里验证，不清理本机的真实浏览器缓存。

AC5 已通过。集成工作树执行完整 just ci，格式、编译、940 项测试及 Clippy 通过，3 项按项目配置忽略。独立检查修复损坏登记记录后，四通道均有拒绝无效登记并保留有效同级配置的回归。AC1–AC4 的分层证据见 [实施记录](research/implementation.md) 和 [独立检查](research/integration-check.md)。CLI 正常入口返回全部四个已批准规则。未访问真实浏览器数据。
