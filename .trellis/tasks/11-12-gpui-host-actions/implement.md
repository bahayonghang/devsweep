# 实施计划

开始前读：`.trellis/spec/backend/index.md`、`provider-authority.md`、`error-handling.md`、`quality-guidelines.md` 里与软件执行有关的部分；本任务 `design.md`。

## 步骤

### A. core 与 CLI：MSI 卸载

1. [ ] 调研并写 `research/msi.md`：产品代码在清单里的来源与可靠性；`msiexec` 的静默与重启抑制参数；退出码映射；按用户与按机器安装的提权差别；`KeepAwakeService` 与 CLI 的状态共享。
2. [ ] **评审关口**：把 `research/msi.md` 的结论（尤其是固定的参数和退出码映射）给用户过目。
3. [ ] core：MSI 分派、产品代码校验、资格取值、历史记录；测试覆盖 AC2、AC3。
4. [ ] CLI：`software plan / preview / uninstall` 支持 MSI；契约测试；中英文输出。
   回滚点 A。

### B. GPUI（在 airy-glass-shell、software-evidence、status-dashboard 之后）

按风险从低到高逐个接入，每个动作完成后跑测试并留一个回滚点。

5. [ ] 共用的审阅与确认组件；摘要失效逻辑的测试。
6. [ ] 保持唤醒。
7. [ ] 启动项开关。
8. [ ] winget 更新执行。
9. [ ] 结束进程。
10. [ ] MSIX 卸载。
11. [ ] MSI 卸载。
12. [ ] 卸载后的残留。
13. [ ] ARP 条目的系统设置跳转；提权提示与"复制等价 CLI 命令"。
14. [ ] 清理只读立场的文案；中英文。
15. [ ] 更新 `.trellis/spec/backend/provider-authority.md` 与 `desktop-runtime.md`。

## 验证

```powershell
cargo test --locked -p devsweep-core software
cargo test --locked -p devsweep-core status
cargo test --locked -p devsweep-cli --test cli_contract
just ci
just gpui
```

- 自动化与截图验证只用测试执行器和夹具（`DEVSWEEP_SOFTWARE_INVENTORY`、`DEVSWEEP_SOFTWARE_UPDATES_FILE`、`DEVSWEEP_SOFTWARE_UPDATE_PROGRAM`、`DEVSWEEP_SOFTWARE_STARTUP_HIVE`）。
- 真实主机上的验证由用户决定是否做、对哪个应用做。未经用户在当时明确同意，不在本机卸载软件、安装更新、改启动项或结束进程。保持唤醒可以实机开关一次并恢复。
- 检索证据：`UninstallString`、`QuietUninstallString` 在执行路径上没有引用（AC3）。

## 评审关口

- 步骤 2（MSI 参数）。
- 步骤 9 之前和步骤 10 之前各停一次，给用户看已接入动作的截图与测试结果，再继续破坏性更强的动作。
