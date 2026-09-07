# 常青项目审查与五套 Harness 对齐改造

## Goal

审查当前 DevSweep 的代码结构、现有测试、失败工作流与五套编码 harness 的规则边界，形成待用户批准的分优先级改造任务树。

## Requirements

- R1：以当前提交和实际文件为准，记录模块、信任边界和证据。
- R2：运行现有检查，分别记录通过、失败、跳过和未验证项；失败须追踪到根因或明确证据缺口。
- R3：对照 Claude Code、Codex、Grok Build、Kimi Code、OMP 的规则加载、skills、子代理、权限和执行环境。
- R4：标明强模型规划/审查与低成本模型可承接的有界实施工作，不将 harness 名称等同于模型质量或成本。
- R5：建立父任务和可独立验收的子任务，每项列出文件范围、依赖和必须通过的检查。
- R6：批准实施后，将稳定结论回写到项目说明或 skill 源，并标注适用工具。
- R7：本轮仅审查、测试和规划，不改产品代码，不激活任务，不安装依赖，不提交或发布，不执行清理/卸载/维护操作。

## Acceptance Criteria

- [x] AC1 (R1, R2, R3): 提供带当前基线与文件定位的审查报告，覆盖 R1–R3。
- [x] AC2 (R2): 报告列出实际命令、退出码、失败根因及尚缺证据，覆盖 R2。
- [x] AC3 (R3, R4): 五套 harness 均有证据分级和分工说明，覆盖 R3–R4。
- [x] AC4 (R5): 父子任务有完整规划、真实上下文清单和明确执行顺序，覆盖 R5。
- [x] AC5 (R6): 回写位置与适用工具有明确验收机制，待批准实施，覆盖 R6。
- [x] AC6 (R7): 规划快照仅任务材料、当时全部 planning；批准后的产品实施与归档不在本条。覆盖 R7。
- [x] AC7 (R5): 批准后共享门禁见 `research/parent-release-gates.md`。`just ci` 0；独占 `--target-dir` workspace 测试 0；Windows process-runner 0；Unix `macos_linux_process_group_tree_termination_dynamic` 为 **UNVERIFIED**，不宣布该项完成；`dist/` 归档被 ignore。

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
