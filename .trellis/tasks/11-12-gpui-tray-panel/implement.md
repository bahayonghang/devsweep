# 实施计划

开始前读：`.trellis/spec/backend/desktop-runtime.md`、`desktop-preferences.md`、`database-guidelines.md`（历史存储）；本任务 `design.md`。

## 步骤

1. [ ] 调研并写 `research/tray.md`：托盘回调消息的挂接点、GPUI 固定版本的窗口种类、Tauri 版托盘与 HUD 的做法、历史里软件与优化成功记录的判定。
2. [ ] **规划关口**：关闭行为的默认值交给用户确定，更新 `prd.md`。

### A. core 与 CLI

3. [ ] 累计统计合并视图与测试。
4. [ ] `history totals` 子命令、契约测试、中英文输出。
   回滚点 A。

### B. GPUI（在 status-dashboard 与 airy-glass-shell 之后）

5. [ ] 托盘回调消息与右键菜单。
6. [ ] 面板窗口、定位、关闭规则。
7. [ ] 可见性与订阅；更新 `status_tray_shares_one_subscription` 等测试。
8. [ ] 关闭行为偏好与设置项；偏好往返测试。
9. [ ] 中英文文案。
   回滚点 B。

## 验证

```powershell
cargo test --locked -p devsweep-core history
cargo test --locked -p devsweep-cli --test cli_contract
just ci
just gpui
```

- 实机：左键、右键、键盘激活；面板与主窗口同时打开；主窗口最小化；资源管理器重启后图标恢复；125% 缩放。
- 原生截图：面板与菜单，英文与中文。
