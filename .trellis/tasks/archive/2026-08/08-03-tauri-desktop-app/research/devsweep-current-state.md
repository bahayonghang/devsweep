# devsweep 现状分析(来源:devsweep-explorer 子代理,2026-08-03)

## 1. 定位

安全优先的开发者磁盘清理规划器 + 执行器(Rust 2024,v0.2.0,MIT)。扫描两类对象:项目构建产物(node_modules、target、.venv、**pycache**,marker-first 遍历)与全局包管理器缓存(npm/pip/pnpm/yarn,只读命令探测)。输出带证据与风险等级的可审计清理计划。

安全模型:clean 默认 dry-run;删除走回收站(trash crate);Rust target 优先 `cargo clean`;全局清理用注册表自有 argv 模板而非 shell 字符串;cargo home 仅检查不删;不跟随符号链接 / reparse point。

## 2. 结构

**单 crate,无 workspace**,约 22500 行(TUI 约占一半)。核心与入口物理分离但可见性未开放:

- `main.rs` → `lib.rs`(仅导出 `run()`)→ `application/`(clap 解析 + 分发)
- `model/` 领域 DTO(全部带 serde)
- `rules/` 规则目录 + 类型化 intent→action 重建
- `scan/` `Sweeper` 扫描编排
- `plan/` UntrustedPlan→ValidatedPlan 信任边界 + SHA-256 摘要
- `execution/` Executor 状态机 + audit JSONL + 安全授权漏斗
- `filesystem/` + `process/` 底层原语(有界可取消的目录测算、Windows Job Object 进程树)
- `tui/` ratatui 界面

## 3. 交互与输出

子命令:tui / scan(--json/--global/--projects/--rescan-target)/ inventory / clean(--plan/--execute/--audit-log)/ protect / rules。JSON 支持完善:`ScanReport` / `InventoryReport` 带版本号,`scan --json` 输出可直接回灌 `clean --plan`。日志走 tracing→stderr,不污染 stdout JSON。

## 4. 复用性(关键结论)

**架构非常适合被 Tauri 后端复用,但可见性完全封死。**

有利:

- **`src/tui/runtime/services.rs` 已定义 ScanService / CleanService / InventoryService 三个 trait —— 一套现成的前端无关服务边界**,是未来 core 公共 API 的雏形;
- 核心 API 普遍支持进度回调(`&mut dyn FnMut(ScanProgress)`)与**协作式取消(`Arc<FlagCancelObserver>`)**;
- 无全局可变状态、无 `process::exit`;Sweeper / Executor 泛型可注入。

阻碍:

- `lib.rs` 仅导出 `run()`,所有类型 `pub(crate)`,服务 trait 甚至是 `pub(in crate::tui)`;
- 约 25 处 `println!` 全部集中在 `application/commands.rs`(终端输出未渗透核心层,剥离成本低);
- crate 内绑定 ratatui / crossterm 依赖。

改造路径:提升核心模块为 pub、将 services trait 上移出 tui、拆 workspace(core + cli + tauri)。主要是可见性与切分工作,无需重写逻辑。

## 5. GUI/Tauri 意图

零代码但有明确意图:`TODO.md` 全文仅一行「做一个tauri应用」。源码/依赖/文档/spec 无 tauri/webview/egui 痕迹。

## 6. 构建与测试

- **`just ci`** = fmt --check + cargo update --offline + check/test/clippy(均 --locked --all-targets,clippy -D warnings)
- CI Rust 作业矩阵为 **windows / ubuntu / macos 三平台**(`.github/workflows/ci.yml`;另有 MSRV 作业用 sed 读根 Cargo.toml 的 rust-version,workspace 化时需适配)
- 测试内联在各模块 `mod tests`;TUI 用 ratatui TestBackend;`src/bin/process_fixture.rs` 进程树夹具
- `just dev` 起 TUI;`just release-archive` 出 Windows 单文件 zip;`just docs` 起 VitePress 中英双语文档站

## 关键路径

- `src/lib.rs` — 复用瓶颈(仅 20 行)
- `src/tui/runtime/services.rs` — 未来 core API 雏形
- `src/application/commands.rs` — println 集中地
- `code_map.md` — 逐文件职责,**改造前必读**
- `Cargo.toml`、`design.md`(42KB 历史设计,优先级低于代码/README/spec)
