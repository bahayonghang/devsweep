# 技术设计:devsweep Tauri 桌面应用总体架构

> 本文件是父任务级总体设计。各子任务在自己的 design.md 中细化本文件对应部分;
> 若细化时发现与本文冲突,以子任务实际调研为准并回写修订此文件。

## 1. 目标架构:Cargo Workspace 三成员

```
devsweep/                     (workspace root)
├── Cargo.toml                [workspace] members = ["crates/*", "desktop/src-tauri"]
├── crates/
│   ├── devsweep-core/        纯逻辑库:scan / plan / execution / model / rules /
│   │                         inventory / filesystem / process / cargo_metadata
│   └── devsweep-cli/         现有 CLI + TUI(application / tui / bin),依赖 core
└── desktop/
    ├── src-tauri/            devsweep-desktop:Tauri 2 app,依赖 core
    └── src/                  React + TS 前端
```

要点:

- `devsweep-core` **无终端输出、无进程退出、无全局状态**(现状已基本满足,
  `println!` 全部留在 CLI 命令层)。
- CLI 二进制名、参数、输出保持不变;`[[bin]] process_fixture` 随 CLI crate 走。
- 备选方案(已否决):不拆 workspace、直接扩大现有 `lib.rs` 公开面。
  否决原因:Tauri crate 会连带依赖 ratatui/crossterm/clap,拖慢编译且边界模糊。

## 2. core 公开 API 面(最小集)

**API 雏形已存在**:`src/tui/runtime/services.rs` 定义了 ScanService / CleanService /
InventoryService 三个前端无关的服务 trait(现为 `pub(in crate::tui)`)。子任务 1
应将其上移到 core 作为公共服务边界,而非从零设计 API(详见
`research/devsweep-current-state.md` §4)。

以现有 `application/commands.rs` 与 services trait 的用法为准,公开且仅公开:

- 扫描:`ScanOptions`、`Sweeper`(`full_scan_report`、
  `full_scan_report_rescanning_target`)、`ScanReport`、`ScanProgress` / `ScanPhase`
  与协作式取消(`FlagCancelObserver`)
- 计划:`UntrustedPlan`、`validate_plan`、版本常量(`SCAN_REPORT_VERSION` 等)
- 执行:`Executor`、`ExecutionRequest`、`ExecutionReport`、`UserProtectionList`
- 规则:`RuleScope`、`TargetId`
- 清单:`inventory_root` 及其报告类型

原则:CLI 命令层用到什么公开什么;不为 GUI 预测性地公开内部结构。

### 2.1 契约缺口(由子任务 2 `core-contract-extension` 补齐)

现状核对(2026-08-03):

- `ScanOptions`、`ScanPhase`、`ScanProgress`(`src/scan/mod.rs`)、
  `ExecutionReport`、`ActionFailure`(`src/execution/mod.rs`)均**无 serde 派生**;
  仅 `ScanReport` 等 `--json` 路径类型有。
- `ExecutionReport` 只有计数(selected/attempted/succeeded/failed/skipped)+
  失败列表,**没有逐目标结果、没有字节数**,无法支撑 UI 的逐项明细与
  "预计可回收 XX" 展示。
- 不存在跨进程可用的确认 digest API(digest 冻结语义目前只活在 TUI 内部,
  见 `.trellis/spec/frontend/state-management.md`)。

子任务 2 需交付(细节在其 design.md):

1. 上述类型的 serde 派生(Serialize + Deserialize,camelCase 由前端约定统一);
2. `ExecutionReport` 扩展逐目标结果(target_id、动作种类、成功/失败/跳过 + 原因);
3. 每目标"预计可回收字节"沿用扫描侧已有的容量口径(verified / partial / unknown),
   执行报告聚合展示,**表述为回收站口径,不称"已释放"**;
4. 确认 digest:对已验证清单(validated manifest)计算稳定 digest 并公开
   "dry-run 返回 digest、execute 校验 digest" 的 API,digest 语义与 TUI 现有
   冻结语义一致(计划或选择集变化 → digest 变化)。

## 3. Tauri 桥接层(desktop/src-tauri)

### Commands(全部 async)

| command | 签名要点 | 说明 |
|---|---|---|
| `scan_start` | `(options) -> ScanReport` | `spawn_blocking` 内跑 `Sweeper`,回调经 Channel 转发为 `scan://progress` event;并发时第二个调用**返回结构化错误 `scan_already_running`(不排队)** |
| `scan_cancel` | `() -> ()` | 触发 `FlagCancelObserver` 协作式取消;验收界限:取消后最迟在下一次进度回调边界停止,合成 fixture 上不超过 5 秒 |
| `plan_dry_run` | `(plan, selected_ids) -> { report, digest }` | 验证计划 + dry-run,返回 `ExecutionReport` 与确认 digest(子任务 2 提供) |
| `plan_execute` | `(plan, selected_ids, digest) -> ExecutionReport` | Rust 侧强制:计划必须通过 `validate_plan`;digest 必须与当前 (plan, selected_ids) 重新计算结果一致,否则结构化错误 `stale_confirmation`;**没有无 digest 的执行路径** |
| `protection_list_get/set` | — | 读写 `UserProtectionList` |

selected_ids 边界(dry-run 与 execute 同规则,Rust 侧实现):

- 未知 TargetId → 结构化错误(整体拒绝,不静默跳过);
- 重复 TargetId → 去重后继续,报告中注明;
- inspect-only 目标(`NoopInspectOnly`)→ 不可被选中执行,选中即错误;
- `irreversible: true` 的 command 动作 → 在 dry-run 报告中显式标记,前端必须
  单独展示该标记。

### Events

- `scan://progress`:阶段(projects/global)、message、可选 partial 计数。
  **`ScanProgress` 无总量字段,前端必须使用不定进度指示(indeterminate),
  不得伪造百分比**;如需真实百分比,由子任务 2 之后单独立项扩展进度契约,
  MVP 不做。
- 执行阶段 MVP 不做流式进度,同步返回 `ExecutionReport`。

### 序列化契约

- IPC 载荷直接使用 core 的 serde 模型(不建平行 DTO 层),**前提是子任务 2
  已补齐 §2.1 所列派生**;前端 TS 类型从 `--json` 样例与 dry-run 样例生成,
  由子任务 4 人工校对并留档生成方法。
- 风险:core 模型字段变更会直接击穿前端 —— 接受,MVP 阶段以 core 为单一事实源。

### 安全边界

- Tauri capability 最小化:不开 fs / shell 插件;一切文件操作走后端 command 内的
  core 逻辑。CSP 保持默认严格。
- 执行链路唯一入口是 `plan_execute`,且必经 validate + digest 双重校验,
  不信任前端任何状态。

## 4. 前端(desktop/src)

- React + TypeScript + Vite(Tauri 2 默认模板)。
- **规范前置**:`.trellis/spec/frontend/` 是 ratatui TUI 规范(其 index 明言
  "It does not describe a web frontend"),**不适用于本前端**。子任务 4 的第一步
  是建立 Web/桌面前端 spec 层(建议 `.trellis/spec/desktop-frontend/`),再据其
  实现;在此之前不得向实现代理注入 TUI 规范。
- 页面流(参照 Mole UX):
  1. **扫描页**:扫描按钮 + 不定进度指示(阶段 + 当前 message 滚动)+ 取消;
  2. **审查页**:目标表格(路径 / 类别 / 大小口径标记 / 风险色标 / 证据展开),
     勾选与全选(inspect-only 目标不可勾选,irreversible 动作显式标记),
     底部汇总 "预计可回收 XX";
  3. **执行页**:dry-run 预览(展示 digest 生效)→ 显式二次确认 → 执行 →
     结果报告(回收站口径:"已移入回收站,预计可回收 XX,清空回收站后释放";
     逐项成功/失败/跳过明细)。
- 状态管理:MVP 用 React 内建状态 + context;不引入重型状态库。
- digest 冻结语义在前端的映射:重新扫描或改变勾选后,旧 dry-run 结果与 digest
  立即作废(UI 置灰),必须重新 dry-run 才能执行。

## 5. 兼容与回滚

- 子任务 1 是纯移动式重构,以 `just ci` 全绿 + CLI JSON 输出等价为门;
  失败即整体 revert,不影响主线。等价比较方法由子任务 1 在 implement.md 中
  固化(固定 fixture + 剔除易变字段 + `jq -S` 规范化 diff)并留档执行记录。
- 子任务 2 是纯增量契约(新增派生与字段),CLI 输出不变仍受同一等价门约束。
- desktop/ 目录为纯新增,任何阶段可整体删除回滚,不触碰 CLI。
- CI 现状与迁移(核对自 `.github/workflows/ci.yml`):
  - Rust 作业矩阵为 **windows / ubuntu / macos 三平台**;
  - **MSRV 作业用 `sed` 从根 `Cargo.toml` 读 `rust-version`**,workspace 化后
    根文件变成 `[workspace]`,该作业必须同步改为读 `crates/devsweep-core/Cargo.toml`
    (或 workspace.package)—— 子任务 1 所有;
  - justfile 各 target 需适配 workspace(`--workspace` / `-p` 粒度)—— 子任务 1 所有;
  - Tauri 是否入 CI、Node/npm 版本门禁、Ubuntu runner 的 webkitgtk 系统依赖策略、
    前端 lockfile 布局 —— 子任务 3 决策并落地,决策结果回写本节;
  - 未签名 Windows bundle(`cargo tauri build`)作为子任务 3 的验收项,签名留待发布。

子任务 3 决策(2026-08-03):

- Tauri 后端以独立 Windows CI 作业执行 `npm run tauri -- build --no-bundle`,
  同时保留 Windows Rust workspace 的 check/test/clippy 门禁;
- Node 固定为 22.x,`desktop/package.json` engines 同时约束 Node `>=22 <23`
  与 npm `>=10 <12`;CI 使用 immutable SHA 固定的 `actions/setup-node`;
- 前端依赖由 `desktop/package-lock.json` 独立锁定,CI 只使用 `npm ci`;
- Ubuntu/macOS 的既有 Rust 作业排除 `devsweep-desktop`,继续守护 core/CLI;
  前端 TypeScript 门在 Windows 桌面作业执行。MVP 平台仅 Windows,因此不在
  Ubuntu runner 安装 WebKitGTK 系统包,避免把尚未承诺的 Linux 桌面构建引入
  跨平台核心门禁;正式支持 Linux 桌面时再新增带版本化 WebKitGTK 依赖的作业;
- MSRV 作业排除桌面 crate,MSRV 继续表示 core/CLI 契约;Tauri/Node 工具链由
  Windows stable 作业单独守护。

## 6. 平台范围

- MVP 目标平台:Windows(与当前开发环境一致,devsweep 对 Win32 有专门支持)。
- macOS / Linux:core 本身跨平台且 CI 三平台守护;桌面端构建验证推迟到 MVP 之后。
