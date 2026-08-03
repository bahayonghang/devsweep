# 桌面前端 UI:扫描-审查-执行流程

> 父任务:`08-03-tauri-desktop-app`(总体设计见父任务 `design.md` §4)。
> 依赖:`08-03-tauri-shell-backend` 必须先完成(需要可用的 command / event 桥接)。

## Goal

建立 Web/桌面前端规范层,实现桌面应用完整 MVP 流程(参照 Mole 产品形态):
扫描(含进度与取消)→ 目标审查与勾选 → dry-run 预览 → digest 确认执行 →
结果报告;并交付端到端验收。

## Requirements

- **规范前置(第一步)**:`.trellis/spec/frontend/` 是 ratatui TUI 规范
  (index 明言不描述 Web 前端),不适用。先建立
  `.trellis/spec/desktop-frontend/` 规范层(组件、状态、类型、质量检查),
  再据其实现;同时重新整理本任务 jsonl 清单指向新规范。
- 技术栈:React + TS + Vite(子任务 3 已搭骨架)。
- 页面流(细节见父任务 `design.md` §4):
  1. 扫描页:触发/取消扫描;**不定进度指示**(`ScanProgress` 无总量字段,
     禁止伪造百分比),展示阶段与当前 message;
  2. 审查页:目标表格(路径 / 类别 / 大小及容量口径标记 / 风险色标 /
     证据展开);inspect-only 目标不可勾选;irreversible 动作显式标记;
     底部汇总"预计可回收 XX";
  3. 执行页:dry-run 预览(绑定确认 digest)→ 显式二次确认 → 执行 →
     结果报告,**回收站口径**:"已移入回收站,预计可回收 XX,清空回收站后
     才真正释放";逐项成功/失败/跳过明细。
- digest 冻结语义映射:重扫或改变勾选后,旧 dry-run 结果与 digest 立即作废
  (UI 置灰),必须重新 dry-run 才能执行;`stale_confirmation` 错误有明确
  用户提示。
- TS 类型:从 `scan --json` 与 dry-run 真实样例生成,人工校对,生成方法留档;
  不在前端伪造或加工安全相关字段;文案不得出现"已释放"。
- 状态管理:React 内建方案,不引入重型状态库。
- **E2E 验收归本任务所有**:在子任务 1 的合成 fixture 上留档完整流程脚本/
  步骤(含取消、digest 失效、错误路径),供父任务集成审查复跑。

## Acceptance Criteria

- [x] `.trellis/spec/desktop-frontend/` 规范层存在且本任务实现符合其检查清单。
- [x] 合成 fixture 上完整 E2E 通过并留档:扫描 → 取消一次 → 重扫 → 勾选 →
      dry-run → 改选触发 digest 失效 → 重新 dry-run → 执行 → 报告。
- [x] 扫描期间 UI 不冻结,不定进度指示实时刷新;扫描失败与各结构化错误
      (`scan_already_running`/`stale_confirmation`/未知 id)均有明确展示。
- [x] 风险、证据、容量口径、irreversible 标记与后端 JSON 数据一致(抽查留档)。
- [x] 未勾选目标时执行入口不可用;dry-run 与 execute 结果可区分;全部文案
      为回收站口径。
- [x] 前端 lint / type-check 全绿(工具链在新 spec 层中定义)。

## Notes

- 视觉参照 Mole 的"清爽 + 醒目成果反馈",不要求像素级模仿。
- 后续迭代(非本任务):托盘、定时扫描、多语言、macOS/Linux 构建、审计日志
  history 视图(钩子已在子任务 3 预留,参见父任务 `research/mole-analysis.md` §6)。
