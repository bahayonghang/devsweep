# 实施计划

开始前读：`.trellis/spec/backend/index.md`、`provider-authority.md`、`error-handling.md`、`quality-guidelines.md` 里与 Optimize 有关的部分；本任务 `design.md`。

## 步骤

1. [ ] 调研并写 `research/batch.md`：每个可执行操作是否需要提权、典型耗时、前置检查；历史记录的粒度；Tauri 与 TUI 对计划和运行输出的使用点。
2. [ ] **规划关口**：把默认勾选的建议清单交给用户确定，更新 `prd.md`。

### A. core 与 CLI

3. [ ] 计划 V2、校验、摘要与测试；V1 兼容测试。
4. [ ] 批量执行与逐项结果；取消；历史。
5. [ ] CLI：可重复的 `--operation`、输出、契约测试；必要时更新 Tauri 解码器与夹具。
   回滚点 A。

### B. GPUI（在 airy-glass-shell 之后）

6. [ ] 页面两区与勾选。
7. [ ] 审阅、确认、运行进度、结果明细。
8. [ ] 中英文文案。
   回滚点 B。

## 验证

```powershell
cargo test --locked -p devsweep-core optimize
cargo test --locked -p devsweep-cli --test cli_contract
just ci
just gpui
```

- 界面与 CLI 的执行验证只用测试执行器或夹具。不在本机真实运行维护操作，除非用户在评审时明确要求。
- 原生截图：两区布局、审阅、结果明细，英文与中文。
