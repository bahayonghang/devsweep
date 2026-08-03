# 可行性分析:为 devsweep 构建 Tauri 桌面应用(参考 Mole)

## 结论:可行性高

Tauri 2 的后端就是 Rust。devsweep 核心逻辑可作为 crate 依赖被 Tauri 后端进程内直接调用,无 IPC 桥接、无 FFI、无子进程解析 stdout 的成本。主要改造点是把当前私有的核心模块暴露为公共 API。

## devsweep 现状(2026-08-03)

- 单 crate(`devsweep` v0.2.0,edition 2024),`src/lib.rs` 仅公开 `run()`,全部核心模块为私有 `mod`。
- 模块分层:`application`(CLI 解析 + 命令编排)、`scan`(`Sweeper`)、`plan`(`validate_plan`)、`execution`(`Executor` / `ExecutionRequest` / `ExecutionReport` / `UserProtectionList`)、`model`(`ScanReport` / `UntrustedPlan` 等)、`rules`(规则注册表、`RuleScope`)、`inventory`、`filesystem`、`process`(Win32 JobObjects)、`tui`(ratatui)。约 22.5k 行(TUI 约占一半);现状深度盘点见 `research/devsweep-current-state.md`。
- **对 GUI 友好的既有设计**:
  - 数据模型全部 serde 可序列化(CLI 已支持 `--json` 输出 `ScanReport`);
  - 扫描入口 `Sweeper::full_scan_report(&options, &mut |progress| {})` 带进度回调(`src/application/commands.rs:24-30`),与 Tauri event 天然匹配;
  - `println!` 只出现在 `application/commands.rs` 命令层,核心层无终端耦合;
  - 安全模型(默认 dry-run、v2 声明式计划验证、trash-backed 删除、registry-owned argv、cargo home inspect-only)全部在核心层实现,GUI 直接继承,不需要重新实现安全逻辑。
- **主要障碍**:`lib.rs` 未公开任何核心类型/函数。Tauri crate 无法 `use devsweep::scan::Sweeper`。需要 workspace 拆分或扩大 `lib.rs` 公开面。

## Mole 参考(`ref/repo/Mole`)

> 深度分析见 `research/mole-analysis.md`。

- macOS 终端清理工具(clean / uninstall / purge / analyze / status),**Bash 为主**
  (清理逻辑全在 `lib/*.sh`),仅 `analyze`/`status` 两个 TUI 用 Go(Bubbletea)。
- **它的桌面版 "Mole for Mac"(mole.fit)是闭源商业产品,不在仓库内** —— 仓库里没有任何 Tauri 或 GUI 代码可直接参考。
- 因此参考价值在**产品形态与 UX**,不在代码:
  - "扫描 → 可视化审查 → 选择性清理 → 释放空间统计" 的主流程;
  - 风险分级 + 逐项证据展示(devsweep 的 evidence/risk 模型已具备对应数据);
  - CLI 与桌面应用并行共存(而非替代)的产品结构;
  - 醒目的 "已释放 XX GB" 成果反馈。

## 技术方案要点(详见父任务 design.md)

1. Workspace 化:`devsweep-core`(纯逻辑)+ `devsweep`(CLI/TUI bin)+ `devsweep-desktop`(Tauri 2 app)。
2. Tauri command 层:`scan_start`(`spawn_blocking` + 进度 event)、`plan_validate`、`plan_execute`(默认 dry-run)、保护清单读写。
3. 前端 React + TS,遵循 `.trellis/spec/frontend/` 既有规范(项目 spec 已预置 frontend 层,说明前端方向早有预留)。

## 风险与缓解

| 风险                                                  | 评估             | 缓解                                                                       |
| ----------------------------------------------------- | ---------------- | -------------------------------------------------------------------------- |
| 核心 API 拆分破坏 CLI 行为                            | 中               | 拆分为纯移动式重构,现有测试全绿作为验收门                                  |
| 长时扫描阻塞 Tauri 主线程                             | 低               | `tauri::async_runtime::spawn_blocking` + 回调转 event;`FnMut` 回调签名兼容 |
| `process` 模块(Win32 JobObjects)在 GUI 进程内行为差异 | 中               | 子任务 2 中做冒烟验证;执行路径与 CLI 共用同一 `Executor`                   |
| Windows 打包/签名                                     | 低(MVP 可不签名) | `tauri build` 产 NSIS/MSI,签名留待发布阶段                                 |
| 前端体积/复杂度膨胀                                   | 中               | MVP 严格限定扫描-审查-执行主流程,不做 Mole 的 uninstall/monitor 等         |
