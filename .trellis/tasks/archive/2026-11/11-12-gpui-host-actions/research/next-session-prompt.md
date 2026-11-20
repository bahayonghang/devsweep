# 下个 session 续作 Prompt

复制下面的内容作为新 session 的第一条消息。本 Prompt 保留步骤 10 的评审关口；本轮提交授权没有批准 MSIX 卸载接入。

```text
继续 Trellis 父任务 .trellis/tasks/11-12-gpui-mole-parity-airy-glass 的子任务树。
仓库：D:\Documents\Code\Rust\Exp\devsweep，分支 dev。先核对 git status、git log 和 task.py current，不假定旧状态仍有效。

按 trellis-start/trellis-continue 恢复上下文。先读 AGENTS.md、code_map.md；再读父任务 prd.md、implement.md、research/gap-analysis.md、research/status-version-strategy.md；随后读活动子任务 gpui-host-actions 的两个 jsonl、prd.md、design.md、implement.md、research/step-10-review.md、research/final-gates-step9.md 和 research/process-step9-check.md。GPUI 修改前读 desktop-runtime.md、desktop-gpui/DESIGN.md，以及新 process-actions.md、software-host-actions.md、keep-awake-session.md。quality-guidelines.md 超过上下文注入上限，按相关章节直接读取源文件。

已知状态（以当前源码复核）：
- shell、status-dashboard、software-evidence 已归档；父任务为 3/9。Tauri 已整体移除，不恢复或修改任何 Tauri 内容。
- clean-review-flow：产品步骤 2–8 与检查已完成，只有部分旧原生证据；用户禁止 Computer Use，未取得的原生项保持未验收，未归档，不阻塞独立后续工作。
- host-actions：步骤 3–9 已实施并独立检查，任务仍 in_progress。软件/Status/GPUI/规范提交分别为 9991d81、755a296、2c5b879、f92367f，另有任务证据和日志提交，以 git log 为准。所有既有改动已获授权本地提交，没有推送。
- 最新组合门禁：just ci 925 passed/3 既有 ignored；just gpui 240 passed，release 构建通过。Python 两次全套均 23 项、2 个已知 gitignore 失败加 1 个 WinError 5：test_skill_distribution 的 resync 用例在 staging.replace(dest) 重命名私有临时目录时拒绝访问；单独通过、全套复现，根因未证明。不要称工具门禁通过，也不要说只剩两个旧失败，不改无关 tools/ACL/忽略规则来掩盖它。

当前必须停在步骤 10 前。先读 research/step-10-review.md，给我当前用户 MSIX 接入边界的简短评审摘要，等我明确确认后才实施；不要把本 Prompt 或上轮“拆分提交”当作步骤 10 已批准。
确认后仅按获批范围推进：复用现有 core 选择计划→实时 sha256: 预览→明确二次确认→SoftwareExecutor 审计/派发/重查链，单进程调用 core，不派生 devsweep.exe。保持既有 MSIX 结果/恢复语义，不以未知当成功，不自动重试。MSI GPUI 卸载、残留、ARP 新入口分别属于步骤 11/12/13，不连带开放；后续按我届时授权继续。

实施和检查交给 trellis-implement、trellis-check 子代理；dispatch prompt 第一行必须是“Active task: <task.py current 输出的路径>”。优先原生上下文注入，缺失或截断时由子代理读 jsonl→prd→design→implement；子代理不递归派发 implement/check。主代理协调、更新规范与生命周期。

验证只用 fake、隔离日志和任务拥有的夹具，不用 Computer Use，不在本机真实卸载软件、安装更新、改启动项、结束用户进程或发起电源请求。core/CLI 既有自建进程夹具可验证其 native helper，不等于原生 GPUI 交互/截图验收。不得以模型/源码测试冒充未取得的原生证据。
严格遵守 AGENTS.md：清理仅从保存计划+实时 sha256: 摘要+--confirm 执行，没有 --execute 或永久删除；扫描/模型只产计划，程序和参数分离；Cargo home 仅检查，不做 Docker 清理；不提交 target/、dist/、__pycache__/。

收尾串行运行 just ci、just gpui、python -m unittest discover -s tools/tests；不要并发 Cargo。provider_authority 若高负载超时可定向重跑，保留首次结果；不要弱化断言/跳过用例。如实记录所有失败与未验收项，更新父任务和 journal。只有整个子任务完成才 archive --no-commit，部分步骤完成不归档。新实施改动先不要提交或推送，等我另行说“提交”。

后续顺序：先完成 Host Actions 的获批步骤，再按父任务推进 analyze-insights、optimize-batch、tray-panel、clean-windows-cache-coverage；各自 D4/D5/D6 等决策关口先评审，不默认获批。全部子任务归档后才做父任务 AC1–AC8 集成评审。
提交格式仍为中文 conventional commit + emoji，trailers 顺序为 Agent-Task、Agent-Model、Generated-By: agent、Co-Authored-By。
```
