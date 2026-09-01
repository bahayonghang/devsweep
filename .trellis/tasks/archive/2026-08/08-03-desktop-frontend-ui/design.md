# 技术设计:桌面前端

> 细化父任务 `design.md` §4。开工时先完成规范层,再按新规范细化本文件。

## 1. 规范层(第一交付物)

`.trellis/spec/desktop-frontend/`:index.md(入口 + Pre-Dev Checklist +
Quality Check)、component-guidelines.md、state-management.md、type-safety.md。
内容基线:React 函数组件 + hooks;类型从后端样例生成不手写业务模型;
lint = eslint + typescript-eslint,type-check = tsc --noEmit。
可参考 TUI spec 的**语义约束**(digest 冻结、selected_by_default 纯投影),
但不复制其 ratatui 实现规则。

## 2. 结构草案

```
desktop/src/
  api/        invoke/listen 封装 + 生成的类型(types.gen.ts)+ CommandError 判别
  state/      AppState reducer + context:idle → scanning → reviewed →
              dryrun{digest} → executing → reported;digest 失效 = 回退 reviewed
  pages/      ScanPage / ReviewPage / ExecutePage
  components/ TargetTable / EvidencePanel / RiskBadge / CapacityLabel /
              ProgressIndicator(indeterminate)/ ConfirmDialog
```

- 状态机单向推进,任何 rescan/选择变更事件把 dryrun 之后的状态整体作废
  (对应 core digest 语义)。
- 事件订阅:`scan://progress` 在 ScanPage 挂载期监听,卸载即退订。

## 3. 类型生成

- 脚本(node)从留档样例 JSON 推导 TS(如 quicktype),输出 types.gen.ts,
  人工 diff 校对后提交;生成命令写入 package.json scripts,方法留档到
  本任务 research/。

## 4. 回滚

- 纯前端目录,页面级提交;规范层独立提交,可先行合入。
