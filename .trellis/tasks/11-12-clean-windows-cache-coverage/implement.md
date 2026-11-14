# 实施计划

开始前读：`.trellis/spec/backend/index.md`、`scan-coverage.md`、`provider-authority.md`、`clean-advisories.md`；本任务 `design.md`。

## 步骤

1. [ ] 调研并写 `research/families.md`：每个候选家族的目录证据、保留目录、所有者进程名。可参考 `ref/Mole` 的 `origin/windows` 分支 `lib/clean/caches.ps1` 与 `cache_guard.ps1` 作为线索，但以实测和厂商文档为准。
2. [ ] **规划关口**：把候选清单和证据交给用户，确定纳入的家族，更新 `prd.md`。
3. [ ] 评估现有 Chromium 与 VS Code 系实现的参数化；必要时先做不改行为的重构，测试不变。
4. [ ] 逐个家族加规则与夹具测试。
   回滚点：每个家族一个提交单位，可单独撤回。
5. [ ] `clean rules` 的中英文说明与 CLI 契约测试。
6. [ ] 更新 `.trellis/spec/backend/scan-coverage.md` 的家族清单。

## 验证

```powershell
cargo test --locked -p devsweep-core rules
cargo test --locked -p devsweep-cli --test cli_contract
just ci
```

- 只在夹具目录里验证，不清理本机的真实浏览器缓存。
