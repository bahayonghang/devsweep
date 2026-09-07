# P1 修复本地与托管检查的失败传播

## Goal

让本地和托管质量门禁对任一步骤失败可靠报错，并检查同一份锁定依赖与生成契约。

## Requirements

- R1: 任一桌面检查失败必须使入口失败，后续步骤不能掩盖退出码。
- R2: 标准检查不更新 Cargo.lock；保留显式人工维护入口。
- R3: 类型生成漂移和文档构建纳入可见检查；Node/npm 版本符合已声明范围。

## Acceptance Criteria

- [ ] AC1 (R1): types:generate/lint/typecheck/test/build 五个位置分别注入失败，入口全部非零；全成功对照为零。
- [ ] AC2 (R2): 在依赖齐备的环境运行 just ci 成功，前后 Cargo.lock 字节哈希相同；sync-lock 仅显式调用。
- [ ] AC3 (R3): 已提交类型与 --stdout 生成结果一致；修改副本中的生成文件后门禁失败，不能被先行重写掩盖。
- [ ] AC4 (R3): 受支持的 Node 22/npm 10 或 11 环境下 desktop 检查和 docs:build 通过；CI 中存在相同职责且失败可传播的步骤。

## Constraints

- Parent: `.trellis/tasks/09-07-evergreen-five-harness-audit`。本任务保持 planning，等待用户批准父任务最终方案。
- 不改业务算法、清理权限或不相关文件；不安装新依赖、不做全局更改、不发布。
- 所有检查结果分为 PASS / FAIL / SKIPPED / UNVERIFIED。
