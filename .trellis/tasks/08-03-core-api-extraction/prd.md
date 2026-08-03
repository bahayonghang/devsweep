# 拆分 devsweep 核心库公共 API

> 父任务:`08-03-tauri-desktop-app`(总体设计见父任务 `design.md` §1-§2、§5)。
> 依赖:无(四个子任务中的第一个)。

## Goal

将现有单 crate 拆分为 Cargo workspace:`devsweep-core`(纯逻辑,公开现有扫描/
计划/执行 API)+ `devsweep-cli`(现有 CLI + TUI,行为不变),并同步适配 justfile
与 CI,为后续契约扩展和 Tauri 桌面端提供可进程内复用的核心库。

## Requirements

- 将 `src/tui/runtime/services.rs` 的 ScanService / CleanService / InventoryService
  trait 上移到 core 作为公共服务边界(现为 `pub(in crate::tui)`,见父任务
  `research/devsweep-current-state.md` §4)。
- workspace 结构与公开 API 面按父任务 `design.md` §1-§2 执行;公开范围以
  "CLI 命令层与 services trait 实际用到什么" 为准,不做预测性公开。
- **明确排除**:不新增 serde 派生、不改 `ExecutionReport` 结构、不做任何契约
  变更 —— 这些属于 `08-03-core-contract-extension`。本任务是纯移动式重构。
- `devsweep-core` 不得包含:终端输出(`println!`/`eprintln!`)、`std::process::exit`、
  clap / ratatui / crossterm 依赖。
- CLI 对外契约完全不变:二进制名 `devsweep`、全部子命令与参数、人类可读输出、
  `--json` 输出、`process_fixture` 辅助二进制。
- CI / 构建适配(本任务所有,父任务 `design.md` §5):
  - justfile 各 target 适配 workspace;
  - `.github/workflows/ci.yml` MSRV 作业的 `sed` 读取路径从根 `Cargo.toml`
    改为拆分后 rust-version 的实际所在(建议 workspace.package 统一声明);
  - 三平台矩阵(windows/ubuntu/macos)保持不变并全部通过。
- 现有测试全部迁移随属 crate,不得删除或弱化断言。
- 公开 API 项补齐 rustdoc(一句话职责即可)。

## Acceptance Criteria

- [ ] `just ci` 全绿(justfile 已适配 workspace;等价覆盖 fmt/check/test/clippy,
      --locked -D warnings)。
- [ ] 三平台 CI 与 MSRV 作业在 workspace 布局下通过(推送分支验证或本地等价复现,
      方式留档)。
- [ ] `cargo tree -p devsweep-core` 中不出现 clap / ratatui / crossterm。
- [ ] JSON 等价门:对 `tests` 侧构造的固定合成 fixture(构造脚本随本任务提交,
      见 implement.md),拆分前后各跑
      `devsweep scan --json --roots <fixture>`,经 `jq -S .` 规范化并剔除
      implement.md 中列明的易变字段(如耗时/时间戳)后 `diff` 为空;
      比较命令与输出留档在本任务目录。
- [ ] `devsweep-core` 可被外部 crate 以 `use devsweep_core::...` 使用
      (doc-test 或最小示例验证)。

## Notes

- 改造前先读 `code_map.md`(逐文件职责索引)。
- 纯移动式重构:不改逻辑、不改算法、不顺手清理。失败即整体 revert。
- 完成后解锁 `08-03-core-contract-extension`。
