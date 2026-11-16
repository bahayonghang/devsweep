# 实施计划

开始前读：`.trellis/spec/backend/index.md`、`provider-authority.md`、`error-handling.md`、`quality-guidelines.md` 里与软件执行有关的部分；本任务 `design.md`。

## 步骤

### A. core 与 CLI：MSI 卸载

1. [x] 调研并写 `research/msi.md`：产品代码在清单里的来源与可靠性；`msiexec` 的静默与重启抑制参数；退出码映射；按用户与按机器安装的提权差别；`KeepAwakeService` 与 CLI 的状态共享。
2. [x] **评审关口**：固定参数、结果/残留规则、R11 与 AC7 边界已获用户确认，见 `research/step-2-review.md`；授权实施步骤 3–8。
3. [x] core：MSI 分派、产品代码校验、资格取值、历史记录；测试覆盖 AC2、AC3。
4. [x] CLI：`software plan / preview / uninstall` 支持 MSI；契约测试；中英文输出。
   回滚点 A。

### B. GPUI（在 airy-glass-shell、software-evidence、status-dashboard 之后）

按风险从低到高逐个接入，每个动作完成后跑测试并留一个回滚点。

5. [x] 共用的审阅与确认组件；摘要失效逻辑的测试。
6. [x] 保持唤醒。
7. [x] 启动项开关。
8. [x] winget 更新执行。
9. [x] 结束进程：实施与产品独立检查完成；工具门禁的额外错误与原生未验收状态见[最终记录](research/final-gates-step9.md)。
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
- 用户于 2026-11-15 要求不使用 Computer Use 验收。后续采用夹具、测试执行器和源码契约证据；尚未完成的原生截图验收如实记录，不声明通过。不改变步骤 2、9 前、10 前的既定评审关口。
- 真实主机上的验证由用户决定是否做、对哪个应用做。本阶段仅使用隔离夹具与测试执行器，没有在本机卸载软件、安装更新、改启动项、结束用户进程或请求保持唤醒。
- 检索证据：`UninstallString`、`QuietUninstallString` 在执行路径上没有引用（AC3）。

## 评审关口

- 步骤 2（MSI 参数）。
- 步骤 9 之前和步骤 10 之前各停一次，给用户看已接入动作的测试结果与实际取得的证据；用户禁止 Computer Use 后，缺少的原生截图明确列为未验证，再评审后续动作。

## 当前进度（2026-11-16）

- 已按用户“继续实施后续 Trellis 任务”的指令运行 `task.py start`；当前活动任务为本任务，状态 `in_progress`。
- 步骤 1 已由 `trellis-implement` 完成，[研究](research/msi.md)与 [trellis-check 独立检查](research/msi-review.md)已落盘；用户确认 [步骤 2 决定](research/step-2-review.md)，已同步 PRD/设计。
- 步骤 3–8 已由 `trellis-implement` 实施并经 `trellis-check` 独立检查；MSI core/CLI、共用审阅、保持唤醒、启动项、更新均有夹具与契约证据，见 [MSI](research/msi-check.md)、[保持唤醒](research/keep-awake-check.md)、[GPUI/更新](research/gpui-3-8-check.md)。GPUI MSI 卸载仍属于步骤 11，没有提前开放。
- 步骤 3–8 的串行门禁：`just ci` 915 通过、3 项既有忽略；`just gpui` 233 通过，release 构建通过；Python 23 项测试仅两个已知 gitignore 子用例失败，整套不算通过。完整计数、日志与产物哈希见 [门禁记录](research/final-gates-3-8.md)。
- 用户于 2026-11-16 确认 [步骤 9 边界](research/step-9-review.md)，继续进程动作的 core/CLI 审计与 GPUI 接入；完成门禁后停在步骤 10 前，不提前接入 MSIX 卸载。
- 步骤 9 已由 `trellis-implement` 实施并由 `trellis-check` 独立检查，见[实施记录](research/process-step9-implementation.md)与[检查记录](research/process-step9-check.md)。检查修复输出预检、明确 Confirm 键盘边界、身份规范化与隔离 Hive 权限，并补最终取消/审计失败的零调用回归。core 与 CLI 的既有测试仅结束各自创建的进程夹具；CLI 还核对实际句柄的错误 FILETIME 拒绝，不涉及用户进程，不算原生 GPUI 验收。
- 步骤 9 最终串行门禁：`just ci` 首次通过，925 通过、3 项既有忽略；`just gpui` 首次通过，240 通过与 release 构建通过。Python 两次全套均为 23 项、两个已知 gitignore 失败、另一个 `test_skill_distribution` 临时目录重命名 `WinError 5` 错误；该用例单独通过、全套复现，根因未证明，未修改无关 tools/ACL/忽略规则。工具门禁不绿，不能说只剩两个旧失败。详见[完整门禁](research/final-gates-step9.md)。
- 已停在步骤 10 前，等待用户评审[当前用户 MSIX 卸载提案](research/step-10-review.md)。步骤 9 实施项完成不代表原生验收或全部工具门禁通过。
- 用户禁止使用 Computer Use 验收；不在本机卸载软件、安装更新、改启动项、结束用户进程或请求保持唤醒。既有 Clean 代码与门禁证据保留，不把其未完成的原生验收算作本任务前置阻塞。
- 用户于 2026-11-16 另行授权拆分提交全部既有改动；产品已拆为软件 `9991d81`、Status `755a296`、GPUI `2c5b879`，规范为 `f92367f`。本任务仍为 `in_progress`，步骤 10–15、未取得的原生证据与上述工具门禁错误尚未完成或解决，不归档、不推送。后续 session 的入口见[续作 Prompt](research/next-session-prompt.md)，没有把提交授权当作步骤 10 的批准。
