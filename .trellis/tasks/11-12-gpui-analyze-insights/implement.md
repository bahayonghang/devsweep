# 实施计划

开始前读：`.trellis/spec/backend/index.md`、`bounded-delivery.md`、`project-occupancy.md`；`.trellis/spec/guides/cross-layer-thinking-guide.md`；本任务 `design.md`。

## 步骤

### A. core 与 CLI

1. [ ] 调研并写 `research/snapshot.md`：遍历器的保留上限、快照里文件节点的完整性、卷容量的取得方式、`recycle_ids()` 的现状、GPUI 弹出菜单的可用接口。
2. [ ] 最大文件投影与测试（阈值、上限、稳定顺序、空结果、不完整标记）。
3. [ ] `analyze roots` 的容量证据与测试。
4. [ ] CLI：`analyze largest` 子命令、契约测试、中英文输出；必要时更新 Tauri 解码器与夹具。
   回滚点 A。

### B. GPUI（在 airy-glass-shell 之后）

5. [ ] 分栏布局与窄窗口堆叠。
6. [ ] 多选与固定动作栏；测试：计划内容、摘要失效。
7. [ ] 上下文菜单与键盘可达性。
8. [ ] 最大文件分段。
9. [ ] 位置卡容量。
   回滚点 B。

## 验证

```powershell
cargo test --locked -p devsweep-core analysis
cargo test --locked -p devsweep-cli --test cli_contract
just ci
just gpui
```

- 用夹具目录走一遍多选、预览、确认。不对本机真实文件执行回收。
- 原生截图：宽、窄两种布局，最大文件分段，上下文菜单，英文与中文。

## 评审关口

- 步骤 1 的结论（尤其是"最大文件"是否完整）先给用户过目。
