# Tauri 桌面应用(参考 Mole)

> 父任务:持有需求集、子任务地图、跨子任务所有权与验收标准;自身仅承载最终集成审查。

## Goal

为 devsweep 提供一个 Tauri 2 桌面应用,复用现有 Rust 核心(扫描 / 计划 / 执行),
以 Mole 的产品形态为参照("扫描 → 可视化审查 → 选择性清理 → 成果反馈"),
与现有 CLI / TUI 并行共存。

可行性分析见 `research/feasibility.md`;现状盘点见 `research/devsweep-current-state.md`;
Mole 分析见 `research/mole-analysis.md`。

## Requirements

- 桌面应用与 CLI 共用同一套核心逻辑与安全模型(默认 dry-run、v2 计划验证、
  trash-backed 删除、registry-owned argv),GUI 不得绕过或重新实现安全逻辑。
- MVP 主流程:触发扫描(含进度与取消)→ 展示清理目标(大小 / 风险 / 证据)→
  用户勾选 → dry-run 预览 → 基于确认 digest 的显式执行 → 结果报告。
- **成果口径(已决策)**:执行结果表述为"已移入回收站,预计可回收 XX",并明确
  提示只有清空回收站后磁盘空间才真正释放。禁止使用"已释放 XX"字样 ——
  主执行路径是 `MoveToTrash`(`src/model/plan.rs`),移入回收站不立即释放空间。
  command-backed 动作(如 `cargo clean`、包管理器缓存清理)可单独表述为
  "已清理",其字节数按核心层提供的口径展示,不自行推算。
- 桌面确认流程必须继承 TUI 的冻结语义(`.trellis/spec/frontend/state-management.md`):
  dry-run 产出不可变的已验证清单及其 digest,执行必须携带同一 digest,
  扫描或选择变化后旧 digest 失效。
- CLI / TUI 行为在整个改造过程中保持不变。

## Non-Goals

- 不移除或弱化 CLI / TUI;不新增清理规则;不解锁 permanent delete;不做 Docker。
- 不做 Mole 的 uninstall / optimize / monitor 功能域。
- MVP 不做自动更新、代码签名、多语言、macOS/Linux 桌面端构建验证。

## 子任务地图(依赖按序)

| 顺序 | 任务 | 交付物 |
|---|---|---|
| 1 | `08-03-core-api-extraction` | workspace 拆分(纯移动式),core 公开现有 API;适配 justfile 与 CI(含 MSRV 作业路径) |
| 2 | `08-03-core-contract-extension` | GUI 支撑契约:serde 派生、逐目标执行明细、预计可回收字节、确认 digest API(依赖 1) |
| 3 | `08-03-tauri-shell-backend` | Tauri 2 骨架 + command/event 桥接 + 未签名打包验证(依赖 2) |
| 4 | `08-03-desktop-frontend-ui` | Web 前端规范建立 + 扫描-审查-执行 UI + 端到端验收(依赖 3) |

依赖关系已写入各子任务 prd.md(父子结构本身不表达依赖)。

## 跨子任务所有权矩阵

| 关注点 | 所有者 |
|---|---|
| workspace 拆分、justfile / CI(三平台矩阵 + MSRV sed 路径)适配 | 子任务 1 |
| serde 契约、逐目标执行结果、可回收字节、确认 digest | 子任务 2 |
| Tauri command/event、digest 强制校验、并发/取消行为、未签名 Windows 打包、Tauri 是否入 CI 的决策与 Node 门禁 | 子任务 3 |
| Web/桌面前端 spec 层建立、TS 类型生成与校对、完整 E2E 流程验收 | 子任务 4 |
| 最终集成审查(见 implement.md) | 父任务 |

## 验收标准(父任务最终集成审查)

- [x] `just ci` 全绿(fmt/check/test/clippy,三平台 CI 通过);现有 CLI 测试无一修改语义。
- [x] 拆分前后 `devsweep scan --json` 在固定 fixture 上等价(比较方法由子任务 1 定义并留档)。
- [x] 端到端流程(由子任务 4 交付)复核通过:扫描 → 勾选 → dry-run(获得 digest)
      → 执行 → 报告;执行动作与同一计划下 CLI `clean --execute` 语义一致。
- [x] GUI 无任何路径可触发核心层之外的删除逻辑;执行必经 digest 校验(代码审查确认)。
- [x] 全部结果文案符合回收站口径,无"已释放"误导表述。
- [x] 四个子任务全部 archive 后,父任务完成本清单再 archive。
