# TUI 可靠性补齐：终端 RAII、列表视口与 stale 目标

> 父任务：07-28-audit-remediation ｜ 审计条目：F-11、F-21、F-25 ｜ 对应审计 Milestone M0

## Goal

补齐 TUI 的三类可靠性缺口：终端状态恢复的 panic 安全（RAII）、目标列表视口滚动、清理完成后的 stale 目标治理，以及渲染层的字符净化与宽度处理。

## 背景与证据（当前 HEAD）

1. **终端生命周期非 RAII（F-11）**：`terminal::enter` 先 `enable_raw_mode` 再 `EnterAlternateScreen`（`src/tui/terminal.rs:12-19`）——第二步失败时 raw mode 泄漏；`restore_terminal` 链式 `?`（`terminal.rs:21-26`）——`disable_raw_mode` 失败则 `LeaveAlternateScreen`/`show_cursor` 永不执行；`tui::run` 无 panic hook/Drop guard（`src/tui/mod.rs:12-20`），事件循环 panic 会跳过 restore，把用户终端留在 raw+alternate screen。
2. **列表无视口（F-21）**：`render_targets` 把全部 visible targets 渲染成 `Paragraph` 行（`src/tui/render.rs:385-411`），无 scroll offset——光标 `j/k` 可以移动到面板高度之外，长列表（如大量 `__pycache__`）不可用。
3. **clean 后 stale（F-21）**：`CleanFinished` 只更新 job/进度（`src/tui/app.rs:370-400`），`targets`/`estimated_bytes`/`selected_ids` 原样保留——已清理路径仍显示旧 size、仍可被再次选中执行（第二次通常失败，但 command-backed 会真实重跑）。
4. **渲染净化（F-25）**：argv preview 用空格 join（`src/tui/render.rs:1232`），含空格参数边界不清；path/stderr 未统一净化控制字符；CJK 路径按 char 截断而非 terminal cell width（`compact_text` 相关），中文路径可能错位/越界。

## Requirements

1. `TerminalSession` RAII：构造即 enter、`Drop` 逐项 best-effort restore（disable raw mode、leave alternate screen、show cursor 各自独立尝试，不因前者失败中断）；安装 panic hook 保证 panic 时终端仍被恢复。
2. 目标列表视口：基于面板高度维护 scroll window，光标始终可见；页面滚动键（PgUp/PgDn 或等价）可选。
3. stale 目标治理：clean 成功的 target 标记 tombstone（显示已清理/从列表移除），并从 `selected_ids` 移除；提示 rescan；在 rescan 前不可再次进入执行选择。
4. 渲染净化：argv preview 对含空格/特殊字符参数加引号或转义；所有进入 UI 的 path/stderr 过滤控制字符；截断按 unicode cell width（CJK=2）计算。

## Acceptance Criteria

- [ ] 单元测试：事件循环内 panic → 终端 restore 仍被执行（可通过 mock backend/hook 验证 Drop 路径）
- [ ] 单元测试：restore 第一步失败时其余步骤仍尝试执行
- [ ] 渲染测试：targets 数 > 面板高度时，光标行始终在渲染窗口内；首/尾边界移动正确
- [ ] 回归测试：clean 成功后该 target 不再出现在可执行选择中；再次执行需 rescan
- [ ] 渲染测试：含空格 argv、含控制字符 stderr、CJK 长路径 —— 输出转义/净化/按 cell width 截断正确
- [ ] `cargo test` 全绿；现有 render 快照测试同步更新

## 约束与依赖

- 独立可并行；与 07-28-tui-race-hardening 同处 `src/tui/`，若并行注意 rebase 顺序（建议 race-hardening 先合入）。
- 终端行为的真实效果（各 terminal emulator 对控制字符的处理）标注"需动态验证"，单元测试只保证净化逻辑本身。
- 审计将 F-25 列为 P3，并入本任务顺带完成；不单独立项。
