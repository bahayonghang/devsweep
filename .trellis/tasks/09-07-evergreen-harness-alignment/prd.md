# P2 对齐五套 Harness 的规则和能力证据

## Goal

使五套工具获得同一项目契约，同时准确描述各自的发现、子代理、hooks、权限与成本分工边界。

## Requirements

- R1: 共享业务规则以 AGENTS.md 为准，工具说明只记录真实入口与能力差异。
- R2: 保留当前 Trellis 0.6.12 生成基线；不自动升级到本机 beta CLI 模板，不改用户全局配置。
- R3: 强模型规划与审查；低成本模型仅处理已冻结输入/输出/文件范围且有明确回归的实施。
- R4: 区分官方支持、磁盘存在、实际发现、实际执行；无法验证的边界明确 UNVERIFIED。

## Acceptance Criteria

- [ ] AC1 (R1): AGENTS 命令/安全规则与当前 CLI 一致；CLAUDE 的 @AGENTS.md 仍有效，managed Trellis block 字节不变。
- [ ] AC2 (R1, R2): 五工具矩阵列出 canonical/compatibility/native-Trellis 入口；项目/全局边界明确，缺原生适配不误判为不可用，无 beta 模板/全局改动。
- [ ] AC3 (R3): 每个子任务标明强模型规划审查者、有界低成本执行范围、失败升级条件与必需检查；不把 OMP/Kimi 本身称为更便宜模型。
- [ ] AC4 (R4): 五工具各有只读发现/上下文握手记录或明确缺口，包含版本/路径/输出来源；需要真实模型或 hook 执行但未运行者保留 UNVERIFIED，不声称五套运行全验收。
- [ ] AC5 (R1, R4): 稳定规则回写到受版本控制项目指南，注明适用工具和核对日期；本轮与实施阶段都不写未批准的团队知识库或用户原生记忆。

## Constraints

- Parent: `.trellis/tasks/09-07-evergreen-five-harness-audit`。本任务保持 planning，等待用户批准父任务最终方案。
- 不改业务算法、清理权限或不相关文件；不安装新依赖、不做全局更改、不发布。
- 所有检查结果分为 PASS / FAIL / SKIPPED / UNVERIFIED。
