# P1 保证 Skill 源与发现副本一致

## Goal

让实际会被工具发现的 devsweep-inspect 与受版本控制的 v2 源一致，并能检测后续漂移。

## Requirements

- R1: skills/devsweep-inspect 是唯一语义源；项目发现目录的副本不允许保留相反的执行指令。
- R2: 同步限制在明确的仓库内目标；独立检查不得隐式修复副本。
- R3: 测试区分静态 fixture 通过、发现一致与 provider 实际遵循；知识回写标注适用工具。

## Acceptance Criteria

- [ ] AC1 (R1): 同步后 .agents/skills/devsweep-inspect 与 .claude/skills/devsweep-inspect 的相对文件集合/字节哈希与源一致，含新增脚本。
- [ ] AC2 (R2): 在隔离目录人为改旧版本/删除一个源文件副本/增加陈旧文件均检测失败；再次同步恢复一致，源哈希不变；越出仓库的目标被拒绝。
- [ ] AC3 (R1, R3): 现有 output_eval 四个正反例通过；全局 binary、禁止仓库 cargo run、先列表再确认的契约在真实发现材料中一致。
- [ ] AC4 (R3): README/skill 安装说明无个人绝对路径依赖，列出各 harness 实际发现根和验证方式；不把静态 eval 写成五平台行为测试。

## Constraints

- Parent: `.trellis/tasks/09-07-evergreen-five-harness-audit`。本任务保持 planning，等待用户批准父任务最终方案。
- 不改业务算法、清理权限或不相关文件；不安装新依赖、不做全局更改、不发布。
- 所有检查结果分为 PASS / FAIL / SKIPPED / UNVERIFIED。
