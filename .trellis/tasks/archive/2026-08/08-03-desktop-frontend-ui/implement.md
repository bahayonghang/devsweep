# 执行计划:桌面前端

## 步骤

1. 建立 `.trellis/spec/desktop-frontend/` 规范层(design.md §1);
   重写本任务 implement.jsonl / check.jsonl 指向新规范;人工 review 规范。
2. api 层:invoke/listen 封装、CommandError 判别联合、types.gen.ts 生成
   流程落地(design.md §3)。
3. 状态机 + ScanPage(不定进度 + 取消 + 错误展示)。
4. ReviewPage:表格、勾选规则(inspect-only 禁选)、irreversible 标记、
   容量口径展示、汇总。
5. ExecutePage:dry-run → digest 确认 → 执行 → 回收站口径报告与逐项明细;
   digest 失效路径。
6. E2E:在合成 fixture 上按 prd 验收清单走全流程,脚本/步骤留档到 research/。

## 验证命令

```bash
cd desktop && npm run lint && npm run typecheck && npm run build
just ci          # 确认未破坏 Rust 侧
# E2E 步骤按本任务 research/ 留档执行
```

## review 门

- 步骤 1 规范层需人工 review 后再进入 2;步骤 5 后整体走查文案口径。

## 回滚点

- 页面级提交,单页可独立 revert;规范层独立于代码可保留。
