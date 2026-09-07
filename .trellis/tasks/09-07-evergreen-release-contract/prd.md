# P1 修复开发发布入口和版本一致性

## Goal

开发启动、归档冒烟和安装包元数据对应当前 CLI 与工作区版本。

## Requirements

- R1: just dev 和 release-smoke 使用现行入口，冒烟只作用于独占测试数据。
- R2: 工作区、desktop npm 根包/锁、Tauri bundle 版本一致。
- R3: 归档检验针对新生成的精确产物，不把旧二进制当作本次证据。

## Acceptance Criteria

- [ ] AC1 (R1): dev 的命令不再含 tui 根；当前终端交互入口的既有测试通过。原生 TUI 手工启动如未运行须单列 UNVERIFIED。
- [ ] AC2 (R1, R3): Windows just release-archive 与修正后的 just release-smoke 通过，记录二进制版本/SHA256、归档与 sidecar；scan/plan/preview 仅在任务临时 fixture 上，无执行副作用。
- [ ] AC3 (R2): Cargo workspace 版本等于 desktop/package.json、package-lock 根版本和 tauri.conf.json；有一个变异反例能使版本检查失败。
- [ ] AC4 (R2): desktop 构建通过且本地 NSIS artifact metadata 与源版本一致；不启动安装器、不签名、不发布。

## Constraints

- Parent: `.trellis/tasks/09-07-evergreen-five-harness-audit`。本任务保持 planning，等待用户批准父任务最终方案。
- 不改业务算法、清理权限或不相关文件；不安装新依赖、不做全局更改、不发布。
- 所有检查结果分为 PASS / FAIL / SKIPPED / UNVERIFIED。
