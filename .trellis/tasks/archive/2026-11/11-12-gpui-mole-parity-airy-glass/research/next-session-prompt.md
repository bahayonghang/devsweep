# 继续实施 DevSweep 的 Trellis 任务，直至整棵任务树归档

请在 Windows 仓库 `D:\Documents\Code\Rust\Exp\devsweep` 继续现有父任务 `11-12-gpui-mole-parity-airy-glass`。目标是交付剩余 6 个子任务，在实际验收通过后逐个归档，最后完成父任务集成验收并归档。直接推进已授权的工作。某一项需要我决定时，告诉我具体选择和影响，同时继续能独立完成的事项。

## 从当前版本接手

1. 读根 `AGENTS.md` 和 `code_map.md`。用 `trellis-start`、`trellis-continue` 恢复上下文，运行 `python .trellis/scripts/get_context.py`、`python .trellis/scripts/task.py list` 和 `python .trellis/scripts/task.py current`。本轮核查时会话指针为空，父任务的 9 个子任务都已有记录，直接复用。
2. 核对 `git status -sb`、`git worktree list`、`git log --oneline --left-right HEAD...origin/dev` 和远端实际分支头。2026-11-20 核查时，本地 `dev` 为 `264e7ac`，相对远端 `324e72b` 领先 9、落后 3。远端 PR #7 已交付 Clean 材质、焦点修正和 Cargo 测试辅助程序定位修复。本地保有 Host Actions、Clean 审阅和任务证据。先比较两侧在 Clean、shell 和测试上的重叠；需要合并时，在独立工作树先验证同时保留两侧行为的结果。保留现有工作树、未提交改动和签收证据。
3. 按[父任务 PRD](../prd.md)、[实施顺序](../implement.md)和[差距表](gap-analysis.md)接手。每次修改前读该子任务的 PRD、设计、实施清单和对应规范。后端从 `.trellis/spec/backend/index.md` 开始，GPUI 另读 `.trellis/spec/backend/desktop-runtime.md` 和 `desktop-gpui/DESIGN.md`。7 份任务记录的上下文清单已全部通过校验；`quality-guidelines.md` 超过注入上限，按改动读原文件相应章节。

## 逐项完成剩余工作

| 子任务 | 已核实的起点 | 下一步及验收重点 |
| --- | --- | --- |
| R4 · Host Actions | 步骤 1–9 已完成，其中 MSI core/CLI 和 GPUI 的保持唤醒、启动项、更新、结束进程已有实施与检查记录 | 先向我简述[步骤 10 提案](../../11-12-gpui-host-actions/research/step-10-review.md)，等我确认后接入当前用户 MSIX 卸载。覆盖身份变化、摘要过期、缺少确认、重复派发、审计失败和结果重查。步骤 11–15 按[任务清单](../../11-12-gpui-host-actions/implement.md)及各自授权推进，包括 MSI 界面、残留、ARP 设置跳转、双语文案和规范。 |
| R5 · Clean 审阅 | 汇总、分组、固定动作栏、完成页已实现。已有 11 张原生图；[原生验收表](../../11-12-gpui-clean-review-flow/evidence/native-acceptance.md)逐项记录了语言、状态和缺口 | 先对照原表和 PR #7 的材质签收。后者覆盖中文只读夹具的宽窄窗口、浅深主题和向前 Tab。优先补六态中缺失的截图、键盘确认到完成及翻页、UIA 部分选中的实际状态、隔离夹具执行后完成页与本次历史核对。Command、InspectOnly 保持仅供审阅。 |
| R7 · Analyze | 处于规划阶段。CLI 已有 `analyze roots` 和回收站计划入口，缺 `analyze largest` | 先完成并给我看快照保留上限、卷容量和菜单接口调研。先交付 CLI 最大文件和卷容量，再验宽窄分栏、多选计划与摘要失效、右键和键盘菜单、中文错误提示。 |
| R8 · Optimize | 处于规划阶段。`optimize plan` 当前接受单个操作 | 调研后由我确定 D6 默认勾选项。先验旧计划兼容、多操作顺序和摘要，再用测试执行器覆盖逐项成功、跳过、失败与项间取消，核对界面和历史。 |
| R9 · 托盘 | 处于规划阶段。`history` 现有 `list`、`show` 两个入口 | 先调研托盘回调与历史统计口径，由我确定 D5 关闭默认行为。交付 `history totals`、菜单、面板和设置，验证隐藏与重开、共用一个采样订阅、均不可见后停采样。 |
| R6 · Windows 缓存 | 处于规划阶段。当前规则表含 Chrome、Edge、Firefox、VS Code、Cursor、用户临时文件六类 | 先为候选家族列出可再生目录、必须保留的数据和进程证据，由我确定 D4 范围。仅对确认家族加规则，用夹具验命中、排除、所有者运行时跳过，以及 CLI、GPUI、TUI 呈现。 |

R1 外壳、R2 Status 仪表盘、R3 软件证据已在 `.trellis/tasks/archive/2026-11/`。R4 的首个评审和 R5 的验收取证可以先进行。等待答复时，可先做后四项各自的只读调研，集中列出有事实依据的待决问题。规划中的子任务完成既定评审后再运行 `task.py start`。

## 验证与归档

- 按 Trellis 自动模式派发 `trellis-implement`，再由 `trellis-check` 独立检查。派发首行写 `Active task: <task.py current 返回的路径>`。主会话协调改动、验收和归档。新能力从 core 和 CLI 契约做起，GPUI 在进程内调用同一 core。清理沿用保存计划、实时 `sha256:` 摘要和明确二次确认；扫描层只产计划，命令分开传程序与参数，文件仅移入回收站，Cargo home 保持只读，Docker 清理维持既定排除。
- 保留现有的 Computer Use 限制。只对任务自己的隔离数据、窗口和测试执行器取证，保留用户前台。可先评估签收工作树 `.worktrees/airy-glass-material/.trellis/tasks/11-18-gpui-clean-airy-glass-material/evidence/` 的窗口截图工具和原图；复用时核对构建、场景和实际覆盖。真实用户软件、缓存、进程和系统设置不作为测试对象。若某条原生验收只能人工完成，指出具体操作、预期结果和缺少的证据，请我签收；继续其他工作。
- 从正常入口验证改动和相关回归，再按任务串行运行 `just ci`、`just gpui` 和适用的工具测试。步骤 9 的[历史门禁](../../11-12-gpui-host-actions/research/final-gates-step9.md)为根测试 925 通过、3 忽略，GPUI 240 通过并完成 release 构建。2026-11-20 复跑 `python -B -m unittest discover -s tools/tests` 共 23 项，两个 gitignore 子用例失败，分别涉及符号链接后的 `target/debug/devsweep.exe` 和 `.trellis/hooks.local.json`。此前全套还出现过技能同步目录重命名 `WinError 5`，此次未复现。保留首轮失败和复跑结果，按证据排查，集成门禁以集成后的构建为准。
- 子任务的每条验收标准都有结果后，写清对应提交或工作树版本、测试、原生证据和限制，更新必要的规范、父任务进度和 journal。验收通过的子任务用 `python .trellis/scripts/task.py archive <子任务目录名> --no-commit` 逐个归档；Git 提交和推送沿用用户的单独授权。
- 最后逐条核对父任务 AC1–AC8，把结果写入父任务 `research/integration-review.md`。核实 9 份子任务归档、桌面动作与 CLI 的同一 core 路径、GPUI 进程派生点、设计 Token、V1 JSON 兼容、安全契约、集成分支两道 Rust 门禁，以及五个模式加托盘的中英文和浅深色截图。对差距表第 4 节逐项记录已解决、有意差异或延后及依据。父任务验收通过后归档，再用 `task.py list` 和归档目录核对整棵树。向我报告实际通过的检查、归档路径和仍需我处理的事项。
