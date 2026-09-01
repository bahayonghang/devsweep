# 修复桌面 CI 前端门禁

## Goal

修复 Desktop CI 调用不存在 npm script 导致的必然失败,使远端作业执行与
仓库当前 Node 22 前端质量门一致。

## Requirements

- `.github/workflows/ci.yml` 不得调用 `desktop/package.json` 中不存在的脚本。
- Desktop CI 在 `npm ci` 后必须执行 fixture 类型生成、ESLint、TypeScript
  type-check 与 Vitest;生产前端 build 继续由 Tauri `build --no-bundle` 触发。
- 不新增或升级依赖,不改变产品代码、Rust 安全契约或平台矩阵。
- 保持现有 Node 22 与 npm lockfile 门禁,不 push 远端。

## Acceptance Criteria

- [x] `mise exec node@22 -- just desktop-web-check` 全绿。
- [x] CI 引用的每个 `npm run <script>` 都存在于 `desktop/package.json`。
- [x] `python ./.trellis/scripts/task.py validate` 与 `git diff --check` 通过。
- [x] 独立检查无遗留 correctness 或 CI 配置问题。

## Notes

- 这是父任务最终集成审查发现的单文件 CI 配置缺陷,按父任务回滚规则独立修复。
