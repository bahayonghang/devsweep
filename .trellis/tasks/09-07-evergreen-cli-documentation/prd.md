# P1 对齐现行 CLI 文档与证据入口

## Goal

读者与任一 harness 从当前说明出发都能找到真实入口，正确理解观察、计划、预览、确认与固定审计存储。

## Requirements

- R1: 当前使用指南只推荐现行 CLI；迁移指南可保留明确标识的旧命令。
- R2: 中英文安全与报告说明一致，不把观察 JSON 等同于可执行计划。
- R3: 代码地图和原生证据链接可追踪到实际文件与对应历史提交。

## Acceptance Criteria

- [ ] AC1 (R1): README 与当前指南的推荐命令通过当前 --help/cli_contract 核对；旧根或 --execute/--audit-log 不再作为现行教程。
- [ ] AC2 (R2): 中英文步骤都包含保存观察、准确选择、保存新计划、live preview digest 和 --confirm；审计位置为当前固定 V1 store。
- [ ] AC3 (R3): code_map.md 中受维护的源码/说明入口路径均存在且涵盖五模式，生成目录只说明用途；原生验证文档指向已归档证据并保留历史提交/UNVERIFIED 标识。
- [ ] AC4 (R1, R2, R3): 文档构建通过；在任务独占 fixture 上复核非破坏性 scan→plan→preview 示例，不执行 clean execute/software uninstall/optimize run。

## Constraints

- Parent: `.trellis/tasks/09-07-evergreen-five-harness-audit`。本任务保持 planning，等待用户批准父任务最终方案。
- 不改业务算法、清理权限或不相关文件；不安装新依赖、不做全局更改、不发布。
- 所有检查结果分为 PASS / FAIL / SKIPPED / UNVERIFIED。
