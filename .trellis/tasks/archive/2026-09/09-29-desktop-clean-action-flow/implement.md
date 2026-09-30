# Implement: Desktop Clean 清理入口与流程可发现性

## Pre-work

- [x] 读 `.trellis/spec/desktop-frontend/index.md`、`component-guidelines.md`、`state-management.md`、`type-safety.md`。
- [x] 读 `desktop/src/i18n/index.ts` 与 catalogue 键集合测试，确认新增键的结构要求。

## Steps

1. [x] Catalogue：在 `resources/i18n/en.json` 与 `zh-CN.json` 新增 design.md 表中 6 个键；修改 zh-CN `clean.v1.action.preview` 为「预览清理」。运行 i18n 测试。
2. [x] `StageResult.tsx`：新增 `busy` 透传。
3. [x] `CleanWorkbench.tsx`：found 分支按 design 重写主/次操作、大数字、次要行；`dryRun()` 成功后 `setView("detail")`；final `ExecutePage.onReturn` 改为 `setView("stage")`。
4. [x] `ExecutePage.tsx`：接收 `targets`，行显示路径/类别；返回按钮文案按 `final` 切换。`CleanWorkbench` 传入 `state.scan?.plan.targets`。
5. [x] `ReviewPage` 汇总栏 sticky 样式。
6. [x] 测试：
   - `CleanWorkbench.test.tsx`：首屏「清理 X」→ `planDryRun` 以默认选中调用 → 演练预览可见 → 「确认清理」→ 对话框 → `planExecute` 以 dry run 摘要调用（R1）。
   - 默认选中为空 → 首屏主按钮为「复核目标」且有原因行，无清理按钮（R3）。
   - 首屏大数字等于默认选中 verified + partial（R4）。
   - 演练预览返回 → ReviewPage；结果详情返回 → 首屏且结果仍在（R7）。
   - dry run 失败 → 首屏 ErrorBanner，未进入确认。
   - 更新因文案或结构改变而失败的既有断言，不删除安全不变量断言。
7. [x] 质量门：

```powershell
cd desktop
mise exec node@22 -- npm run lint
mise exec node@22 -- npm run typecheck
mise exec node@22 -- npm run test
mise exec node@22 -- npm run build
cd ..
just ci
```

8. [x] 视觉证据：`just tdev` 或 fixture bridge 下，800 与 1440 CSS 像素、中英文，截图首屏(found)、审阅、演练预览、结果。

## Review gates

- 步骤 3 后：确认 `state-management.md` Invariants 全部仍由 reducer 保证，UI 层没有新增授权路径。
- 步骤 6 后：`skip-protect.test.ts` 与 `app-state.test.ts` 未修改且通过。

## Rollback points

- 每一步独立可回退；catalogue（步骤 1）可单独保留。

## Verification record（2026-09-29）

- desktop：lint、typecheck、test（51 文件 / 391 测试）、build 通过。
- Rust：fmt、check、clippy 通过；`cargo test --workspace --all-targets --exclude devsweep-core` 通过。
- `devsweep-core --lib` 测试二进制链接后 `.exe` 被移除（`.pdb` 保留），在两个 target 目录中复现，报 `never executed`。原因未查明。本任务未改动 `devsweep-core`。
- fixture 预览：1440×900 英文与 800×700 中文首屏、演练预览、结果、审阅页已检查。
