# 下一次对话实施 Prompt

继续实施 `D:\Documents\Code\Rust\Exp\devsweep` 的 GPUI + Rust Desktop 重写。
使用当前 `codex/gpui-desktop-rewrite` 分支。先核对 Git 状态和 HEAD。
不要 reset、stash 或覆盖未提交改动。不要重建任务，不要重做已经通过的规划，
不要因为旧文档里的“待批准”再索要实施批准。

写本 Prompt 时 HEAD 是 `0b80363`。产品接线在 `c7846e4`。
`6E827733`、`5BFE59AF`、`23F7D614` 都不能套到这个产品提交上。

## 顺序

未完成任务都不能归档。按这个顺序做，软件验证放在最后：

1. C11 实施，不做原生验证：
   `.trellis/tasks/10-15-gpui-clean-workbench/implement-prompt.md`
2. C13 实施，不做原生验证：
   `.trellis/tasks/10-15-gpui-analyze-status-tray/implement-prompt.md`
3. C06 只保持开放，不在这台机器上做主机维修：
   `.trellis/tasks/10-15-gpui-optimize-maintenance/implement-prompt.md`
4. C14 只保持阻塞，不打包、不退役 Tauri：
   `.trellis/tasks/10-15-gpui-acceptance-migration/implement-prompt.md`
5. 最后才做软件验证：
   `.trellis/tasks/10-15-gpui-software-optimize-workbenches/implement-prompt.md`

前四份不构建 release，不跑 UIA，不跑帧间隔，不打开 Software 审阅。
第五份先补 C11/C13 的原生缺口，再判断 C06 有没有可丢弃目标，
Software 工作台验证放在该份的最后一节。C14 的测量仍在软件验证之后，
而且本 Prompt 不授权开始 C14。

## 已提交，不要推倒

- `66e4a84` 扫描去重只规范化一次路径。
- `7cc16b6` 没有 Git 标记时不启动 git。
- `2bb5e1e` Windows 子进程不再弹出控制台。
- `532eff0` 暴露清单与启动项的稳定身份。
- `c7846e4` 把五个工作台接到原生窗口。
- `8d050f8`、`47585e0`、`deba8d1`、`0b80363` 是证据和父账本，不是新的验收通过。

已经接上、不要重做：Clean 落地页、扫描、预览、确认、历史、AI 审阅、磁盘地图；
Software/Optimize 的隔离审阅和未运行主机行；Analyze 的根选择、treemap 与列表；
Status 的一次采样订阅；Keep Screen On 和 Clean Screen 的 Escape/F11。

## 共同约束

- Windows x64 优先。功能走已接受的 core/CLI 计划，界面不另写删除或卸载。
- 父任务实施方案及子任务已经批准。`implementation-approval.md` 是授权记录。
- 本 Prompt 不授权提交、推送、归档、安装、签名、发布或删除 Tauri。
- 本机使用已安装的 Cargo 1.99.0。不要下载 1.88。不要导出 `CARGO_TARGET_DIR`。
  不要直接调用 rustup cargo。工作区测试用 `--target-dir dist/ci-target`。
  GPUI 用 `desktop-gpui` 下的 `dist/goal-gpui-test` 和 `dist/goal-gpui-release`。
  release 链接加 `RUSTFLAGS=-C link-arg=/Brepro`。
- 不要编辑 `desktop/`。不要卸载真实软件，不要改真实启动项，不要对这台机器运行
  defrag、dism `/Online`、`/RestoreHealth` 或 `w32tm /resync`。
- 改代码前读对应层的 `.trellis/spec` Pre-Development Checklist。
- 提交、测试退出码或窗口能打开，都不等于重写完成。
