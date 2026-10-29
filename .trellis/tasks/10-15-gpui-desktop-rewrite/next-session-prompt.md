# 下一次对话实施 Prompt

继续实施 `D:\Documents\Code\Rust\Exp\devsweep` 的 GPUI + Rust Desktop 重写。
使用当前 `codex/gpui-desktop-rewrite` 分支。先核对 Git 状态、HEAD 和未提交改动，
保留全部工作区改动，不要 reset、stash 或覆盖。不要重新建任务，不要重做已经
通过的规划，不要因为旧文档里的“待批准”再索要实施批准。

本轮只补还没接到窗口的底层功能和界面。剩余原生验证、性能样本、主机维护实测、
OS 崩溃释放、FocusPattern、打包和 Tauri 退役都放到最后，不要为它们停下来。

## 已确认范围与授权

- Windows x64 优先，保留现有 CLI 跨平台支持。功能先在共享 Rust 核心与 CLI 中实现，
  GPUI 界面用于展示和操作。界面调用已接受的 core/CLI 计划，不另写一套删除或卸载。
- 父任务实施方案及全部子任务已经批准。`implementation-approval.md` 是授权记录。
  不要因 `final-review.md` 里旧的“待批准”再提问。
- 本 Prompt 不授权提交、推送、归档、安装、签名、发布或删除 Tauri。做完可审阅的
  界面后再集中说明需要哪一项授权。
- 本机使用已安装的 Cargo 1.99.0。不要下载 1.88。不要把 `CARGO_TARGET_DIR` 导出。
  不要直接调用 rustup cargo。工作区测试用 `--target-dir dist/ci-target`。GPUI 用
  `desktop-gpui` 下的 `dist/goal-gpui-test` 和 `dist/goal-gpui-release`。release
  链接加 `RUSTFLAGS=-C link-arg=/Brepro`。
- 不要编辑 `desktop/`。不要卸载真实软件，不要改真实启动项，不要对这台机器运行
  defrag、dism `/Online`、`/RestoreHealth` 或 `w32tm /resync`。

## 先恢复真实状态

HEAD 在 2026-10-29 核实时是 `7ef934b`。未提交的 `desktop-gpui` 与 core 改动是有效
实现。账本比子任务 PRD 里更早的叙述新。改代码前读对应层的
`.trellis/spec` Pre-Development Checklist。

已经接上、不要推倒重做：

- Clean 落地页、扫描、预览、确认、历史、AI 审阅、磁盘地图文字。
- Software/Optimize 键盘选择、隔离 hive 审阅、实时 `sha256:` 摘要、再次确认、
  过期拒绝、`probe_failed`、权限拒绝、残留项不可用、guidance/observe 不可调度、
  主机维护行显示未运行。
- Analyze 在 `DEVSWEEP_ANALYZE_ROOT` 下的列表/treemap 文本行、稳定节点 id、
  空/不可读/存在、回收实时摘要。
- Status 一次快照行，显示的 PID 不能结束进程。
- 列表窗口有界。离屏选择不能丢。

当前 release 证据哈希是
`6E827733E649AE9A13E1349AE0565C17F90115C9B5B006ABF66E3F5F1CF37A69`。
不要把 `5BFE59AF…` 或 `23F7D614…` 的旧测量套到新界面上，也不要为了本轮重跑
20 次开关、五模式落地页或 10k 帧间隔。

## 2026-10-29 这一轮已经接上，不要推倒重做

未提交。没有新的 release 哈希。不要把 `6E827733` 或更早的测量套到这批界面上。

- Software Uninstall / Updates / Startup 可以进入隔离审阅。行上有身份、数量、来源、版本、权限或保护、跳过原因和最后结果。搜索、排序、过滤、刷新只读隔离清单、更新文件或临时 hive。没有隔离执行器时拒绝卸载，不调用 `SoftwareExecutor`。残留项仍不可用。没有隔离来源时不读主机卸载清单。
- Optimize 可按 startup、network and search、maintenance 分组。行仍有 class、privilege、skip、last。阶段能表达 Landing、Running、Completed、Failed、admin wait。guidance、observe 和主机 defrag / DISM / 时钟同步没有调度按钮，也不能被记成成功。
- Analyze 落地页可选择隔离根或窗口列出的驱动器，不靠手写 `DEVSWEEP_ANALYZE_ROOT`。Detail 有 treemap 矩形和列表，共用稳定节点 id，并有面包屑、说明、reveal 身份和回收。缺摘要或过期摘要拒绝，不在界面里另写删除。
- Status 有 Landing 和 Live。Live 能显示健康、CPU、GPU、内存、电池、空间、网络、温度和可搜索进程列表。缺传感器保持 unavailable。主窗口和托盘共用一个采样订阅。显示的 PID 不能结束进程。
- Keep Screen On 和 Clean Screen 已接到窗口。Escape 或 F11 恢复清屏，不锁输入。不调用 `powercfg`。

`desktop-gpui` 下 `cargo test --locked --target-dir dist/goal-gpui-test` 在 2026-10-29 退出 0（140 passed）。这不是原生验收，也不是重写完成。

## 下一轮仍不要当成完成条件

这些界面已经有模型和窗口入口。下一轮不要重做它们。剩下的是原生证据，不是再写一套删除。

## 本轮明确不做

这些不是本轮的完成条件。不要为了它们中断界面实现，也不要写成 PASS：

- C11 的 16.7 ms PageDown、100 次取消样本、FocusPattern。不要放宽 16.7 ms，
  不要分叉 AccessKit。
- C06-AC4 的 defrag、dism、`w32tm /resync`。
- 重跑 20 次创建/最小化/恢复/`WM_CLOSE`、五模式落地页截图、10k 帧间隔。
- OS 崩溃释放。做不到就保持 NOT_RUN。
- 大快照性能样本、相对 Tauri 基线、打包安装器、签名、发布。
- 宣称重写完成，或删除 Tauri。

本地门禁用已安装的 `cargo test --locked --target-dir dist/goal-gpui-test`。
编译失败就修。原生 UIA、截图和性能留到功能界面都接上之后的单独一轮。

结束时说明：哪些界面已经能操作，哪些仍然只是文本行，哪些验证被故意留到最后。
提交、构建成功或窗口能打开，都不等于重写完成。
