# 实施计划（父任务）

父任务不直接改产品代码。它的工作是排期、关口和集成评审。

## 启动前

- [x] 用户确认 D1（2026-11-12：全部接入），`11-12-gpui-host-actions` 的三份文档已写完。
- [ ] 用户批准最终规划摘要。在此之前不运行 `task.py start`，不改产品代码，不提交。

## 波次

1. [x] 波次 1：启动并完成 `11-12-gpui-airy-glass-shell`。已完成并归档；前后对照截图与验收记录见归档任务。
2. [ ] 波次 0（可与波次 1 并行）：status-dashboard、software-evidence、analyze-insights、optimize-batch、tray-panel 的 core 与 CLI 部分。共用版本结论见本目录 `research/status-version-strategy.md`。
   - [x] status-dashboard 的 core 与 CLI：已完成并归档；版本结论见 `research/status-version-strategy.md`。
   - [x] software-evidence 的 core 与 CLI：V1 增量与双语契约已完成并归档。
3. [ ] 波次 2：status-dashboard、software-evidence、clean-review-flow 的界面部分。
   - [x] status-dashboard 的 GPUI：已完成并归档。
   - [x] software-evidence 的 GPUI：已完成并归档；原生验收及 WindowsApps 图标回退限制见[子任务记录](../archive/2026-11/11-12-gpui-software-evidence/evidence/native-acceptance.md)。
4. [ ] 波次 3：host-actions。
5. [ ] 波次 4：analyze-insights、optimize-batch、tray-panel 的界面部分；clean-windows-cache-coverage。这三个 P2 子任务和缓存扩展各有自己的规划关口（D4、D5、D6），到时再问。

每个子任务单独 `task.py start`、实施、检查、归档。一次只启动能独立验证的那一个或互不相干的几个。

## 集成评审（全部子任务归档后）

- [ ] 逐条对照 `research/gap-analysis.md` 第 4 节，标注已解决、有意差异或延后，写入 `research/integration-review.md`。
- [ ] 按各子任务的"等价 CLI"表核对父任务 AC2。
- [ ] 检索 `desktop-gpui/src` 的进程派生点（AC3）。
- [ ] 逐条复核 `AGENTS.md` 的安全契约（AC6）。
- [ ] 原生截图矩阵（AC8）。
- [ ] 更新 `code_map.md` 和受影响的 `.trellis/spec/` 文档。

## 验证命令

```powershell
just ci      # 格式、检查、测试、Clippy
just gpui    # GPUI 检查、测试、release 构建
just gpui-dev
```

## 约束提醒

- 验证清理、卸载、维护类动作只用夹具或测试执行器。不在本机真实执行，除非用户明确要求。
- 不提交 `target/`、`dist/`、`__pycache__/`。只在用户要求时提交。
