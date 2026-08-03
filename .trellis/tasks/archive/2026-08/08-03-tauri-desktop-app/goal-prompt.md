# Goal 模式启动 Prompt(devsweep Tauri 桌面应用任务树)

> 用法:将下方分隔线内的全文作为 goal 模式的目标输入(`claude` 会话中直接粘贴,
> 或作为 --goal / 长任务模式的初始 prompt)。

---

## 目标

按 Trellis 规划完整实施 devsweep Tauri 桌面应用任务树,直至四个子任务全部
archive、父任务完成集成审查并 archive。所有规划工件已就绪并经过审阅,
不要重新规划,只做规划中定义的事。

## 任务树与顺序(严格按序,前一个 archive 后才开始下一个)

1. `.trellis/tasks/08-03-core-api-extraction` — workspace 拆分,core 公开 API,CI/justfile 适配
2. `.trellis/tasks/08-03-core-contract-extension` — serde 派生、逐目标执行明细、可回收字节、确认 digest
3. `.trellis/tasks/08-03-tauri-shell-backend` — Tauri 2 骨架、command/event 桥接、未签名打包
4. `.trellis/tasks/08-03-desktop-frontend-ui` — desktop-frontend spec 层、扫描-审查-执行 UI、E2E
5. 父任务 `.trellis/tasks/08-03-tauri-desktop-app` — 按其 implement.md 做最终集成审查

## 每个子任务的固定流程(Trellis Phase 1.4 → 3.5)

1. `python ./.trellis/scripts/task.py start <task-dir>`(状态 → in_progress)。
2. 读该任务的 `prd.md` → `design.md` → `implement.md`,以及 jsonl 清单所列
   spec/research 文件;implement.md 的步骤就是执行顺序。
3. 按 Trellis 流程 dispatch `trellis-implement` → `trellis-check` 子代理
   (prompt 以 `Active task: <task path>` 开头)。
4. implement.md 里标注的"人工 review 门"在本 goal 运行中替换为:
   自查 + 将审查结论写入该任务目录 `research/self-review.md`,然后继续;
   不等待人工输入。
5. 每个任务的 Acceptance Criteria 逐条验证并在 prd.md 中勾选,留档要求的
   证据(fixture 输出、diff 结果、验证记录)写入任务目录。
6. `just ci` 全绿后,走 Phase 3.3 spec 更新(`trellis-update-spec` 判断)、
   Phase 3.4 批量提交(见下方提交约定),然后
   `python ./.trellis/scripts/task.py archive <task-dir>`。
7. 进入下一个任务。

## 硬性约束(违反即停止并报告,不得变通)

- 安全模型不可触碰:默认 dry-run、执行必经 validate + digest、无免 digest
  执行路径、不解锁 permanent delete、GUI 不绕过 core 删除逻辑。
- 成果文案一律回收站口径("已移入回收站,预计可回收 XX"),禁止"已释放"。
- CLI/TUI 行为不变:JSON 等价门(子任务 1 定义的 fixture + `jq -S` diff)
  在子任务 1、2 各验一次,失败即 revert 当前批次。
- 子任务 1 是纯移动式重构,发现"顺手改进"冲动一律放弃。
- 不 push 远端;全部提交留在本地 `dev` 分支。
- 不修改 `.trellis/workflow.md`、权限配置、CLAUDE.md。

## 提交约定

- 沿用仓库风格:`type(scope): [AI] <emoji> 中文描述`
  (参考 `git log --oneline -5`)。
- 开工前先把 `.trellis/tasks/08-03-*` 五个任务目录的规划工件单独提交一次:
  `chore(task): [AI] 📋 规划 Tauri 桌面应用任务树`。
- 每个子任务的工作提交与 archive 提交分开,不 amend,不合并跨任务改动。

## 卡死与失败处理

- 同一问题修复 3 次仍失败:停止该路径,加载 `trellis-break-loop` 做根因
  分类并写入任务目录,然后:若有可行替代方案(design.md 中列过的备选)则
  换方案继续;若没有,停止整个 goal 并输出阻塞报告(已完成什么、卡在哪、
  建议的人工决策点),不要在失败路径上无限消耗。
- 需要联网安装的依赖(create-tauri-app、npm 包、Node 工具链)安装失败时,
  记录确切错误后重试一次;仍失败则按上一条输出阻塞报告。
- 任何验收门无法客观判定时,以对用户更保守的解释执行。

## 完成定义

- [ ] 四个子任务 prd.md 的 Acceptance Criteria 全部勾选且证据留档
- [ ] 四个子任务 + 父任务全部 archive
- [ ] 父任务 implement.md 集成审查清单逐项完成
- [ ] `just ci` 在最终状态全绿
- [ ] 全部提交在本地 dev 分支,无未跟踪的工作文件残留
- [ ] 输出最终报告:每个任务做了什么、验收证据位置、遗留的后续迭代项

---
