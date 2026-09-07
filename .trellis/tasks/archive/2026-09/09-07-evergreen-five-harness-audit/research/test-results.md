# 测试与复现记录

核验日期：2026-09-07（Asia/Shanghai）。基线：`ff9f57eeda7e610ee326e81b80bf011e57a76fd0`，分支 `dev`，开始时工作区干净。

## 环境与实际结果

- Windows / PowerShell；rustc 1.98.0、cargo 1.98.0、just 1.58.0。
- 系统默认 Node 26.7.0/npm 12.0.2，不符合 desktop/package.json。
- `mise exec node@22 -- npm` 虽切到 Node 22.23.2，npm 仍解析到全局 12.0.2。首轮结果保留为附加证据。
- 权威桌面复测使用已安装的 Node 22.23.2 与其 `node_modules/npm/bin/npm-cli.js`（10.9.8），只改变该子进程的 PATH，没有安装或修改全局工具。

| 检查 | 实际命令/方法 | 状态 | 证据 |
| --- | --- | --- | --- |
| Rust format | `rtk proxy just fmt` | PASS，exit 0 | rust-fmt.log |
| Rust check | `rtk proxy just check` | PASS，exit 0 | rust-check.log |
| Rust tests | `rtk proxy just test` | PASS，568 passed / 0 failed / 3 ignored | rust-test.log |
| Rust clippy | `rtk proxy just clippy` | PASS，exit 0 | rust-clippy.log |
| Desktop lint/typecheck/test/build | Node 22/npm 10 CLI，`--prefix desktop run <gate>` | PASS，四项 exit 0；31 files / 181 tests | supported-desktop-results.jsonl、node22-npm10-*.log |
| Generated TS drift | 现有 types-generation.test.ts，两次 --stdout 与 committed 文件比较，再变异 fixture | PASS（包含于 181） | node22-npm10-test.log |
| Skill fixture output | `python -X utf8 skills/devsweep-inspect/scripts/output_eval.py skills/devsweep-inspect` | PASS，4/4 | skill-output-eval.json |
| Documentation | Node 22/npm 10，`npm run docs:build` | FAIL，exit 1；vitepress not recognized | node22-npm10-docs-build.log |
| RustSec | `rtk proxy cargo audit`，0.22.2 | PASS，exit 0；18 allowed warnings，不是零警告 | cargo-audit.log |
| Dependency policy | `rtk proxy cargo deny --all-features check`，0.20.2 | PASS，exit 0；四类均 ok | cargo-deny-corrected.log、cargo-deny-exit.txt |
| Existing aggregate ci | `just ci` 原样入口 | SKIPPED，包含 cargo update --offline 的依赖变更；四个质量项已独立全跑 | justfile:11、justfile:160 |
| Secret scan | 本机无 gitleaks 命令 | SKIPPED 本地；当前 HEAD hosted job PASS | 下列 hosted 链接 |

568 = CLI library 183 + cli_contract 12 + five_mode_contract 7 + core 326 + public_api 1 + desktop Rust 39。
3 ignored = native Settings ShellExecute probe 1 + 两个仅供子进程启动的 fixture 入口；不能描述为三个产品回归测试失败。
文档环境检查：`node_modules/vitepress/package.json` 与 `node_modules/.bin/vitepress.cmd` 均不存在。package.json/lock 已声明依赖；这是缺少本地已声明依赖的构建前置，尚无证据说明 VitePress 源码构建有错。本轮未执行 npm ci。

操作记录保留首次误用 `cargo deny check --all-features`（exit 2，参数位置错误）在 cargo-deny.log/security-results.json；这是审查命令错误，不列为项目缺陷。随后正确命令完整通过。

Audit 警告并未当作无风险略过：`cargo tree --locked --invert lru@0.18.1` 确认 lru → ratatui-core → ratatui → CLI；`cargo tree --locked --invert glib@0.18.5 --target all --depth 4` 指向 GTK/Tauri 图。警告包含维护状态与 unsound 提示；本轮没有证明相关 API 在本项目可触发，也没有得到一个已验证的安全升级组合。因此记录为后续依赖维护证据，不擅自升级整条 Tauri/Ratatui 依赖，也不声称没有安全风险。

## 最小失败复现

| 复现 | 输出 | 结论 |
| --- | --- | --- |
| `cargo run --locked -p devsweep-cli --bin devsweep -- scan --json` | exit 2，unrecognized subcommand scan | README/release-smoke 使用已移除根 |
| 同上 `-- tui --help` | exit 2，unrecognized subcommand tui | just dev/README 入口失效 |
| 同上 `-- clean --plan does-not-exist.json` | exit 2，unexpected argument --plan | 旧 clean 平铺调用失效，尚未触及文件/执行 |
| `research/reproduce-gate-exit.ps1` | 原版 recipe，mock npm exit 23；types:generate/lint/typecheck/test 四个位置最终都 exit 0，build 失败才 exit 1 | PowerShell 分号链吞掉前序失败 |

CLI 日志为 cli-scan.log、cli-tui.log、cli-clean.log。失败注入证据为 gate-exit-probe.log，脚本从真实 justfile 读取原始 recipe，在独立 gate-probe/desktop 中用 mock npm 运行；没有修改真实配置/依赖/类型文件。

## 当前与历史 Hosted CI

- [当前 HEAD 的完整 CI](https://github.com/bahayonghang/devsweep/actions/runs/33588352167)：2026-09-02，headSha 与本轮基线完全相同，9/9 jobs success，包括三 OS Rust、双 OS MSRV、Desktop、audit、deny、secret-scan。只是既有 run，本轮没有触发远端工作流。
- [历史失败 run](https://github.com/bahayonghang/devsweep/actions/runs/33452741798)：2026-08-31，`953cb7f84975230f2ff3243bed5ac147566c4cfa`，7 个 job failure、audit 和 Desktop success。不能把这些旧失败算成本轮未修复项。

| 历史失败链 | 根因与修复证据 | 当前判定 |
| --- | --- | --- |
| Unix/MSRV isolation；Windows golden | 测试依赖 LOCALAPPDATA；CRLF/LF 和 manifest/golden 前提不一致。`53840d1` 修改 cli.rs/i18n/five_mode_contract。 | 已有修复提交；当前相应 jobs PASS |
| 非 Windows Optimize | dns.flush preview 正确返回 OsBuildUnavailable/exit 4，旧测试预期 Windows 行为；`cee63e0` 改测试契约。 | 已修复，不要求跨平台强行执行 Windows 维护 |
| Unix Clippy | Windows 专用符号在非 Windows 下 dead_code；`53f387c` 调整 cfg 相关 lint。 | 当前 Clippy jobs PASS |
| cargo-deny | Tauri 依赖许可证 MPL-2.0 未纳入 allow；`1bf02d1` 补策略。 | 本地与当前 hosted PASS |
| Gitleaks/报告异常 | 历史任务 evidence 误纳 Edge profile，generic-api-key 形态命中并产生大量日志；`bd53bd6` 清除当时工作树中的 profile 后归档。 | 当前 secret-scan PASS；未声称历史对象被清除，也未确认命中是可用凭据 |

以上历史判断来自只读工作流审查与实际修复提交；不复制命中值、浏览器 profile 内容或 secrets。后续知识回写应要求 native evidence 只保留必要输出/索引，不纳入浏览器用户配置；无需新清理/改写 Git 历史。

## 尚未验证的表面

规划审查当时没有运行发布归档、NSIS 打包/安装、原生 TUI/桌面交互、跨 OS 本地重测、真实清理/卸载/DNS/Settings 操作、其他四套模型会话与 hooks。历史 native 文档的 PASS 对应其旧提交，不是当时 HEAD 的新验证。

## 实施后父发布门（2026-09-07，产品 HEAD `93dc07c`）

权威记录：`parent-release-gates.md`。摘要：

| 检查 | 命令 | 退出码 | 判定 |
| --- | --- | --- | --- |
| 本地 `just ci` | `just ci` | 0 | PASS；Cargo.lock SHA256 前后均为 `73b7fe3189dd1c9c04b04f023755af950e6ce5636709608d22fb818d6d87a7a9` |
| 独占 `--target-dir` workspace | `cargo test --workspace --locked --all-targets --target-dir target/ws-exclusive-9a1a5474-967f-4822-bc65-a970201492c6` | 0 | PASS；`CARGO_TARGET_DIR` 未设置、未导出；未删除现有 `target/` |
| Windows process-runner | `cargo test --locked -p devsweep-core --lib process::tests -- --nocapture` | 0 | PASS（19）；无 Unix 测试名 |
| Unix process-runner | 同上，Unix 主机 | 未运行 | **UNVERIFIED**；缺 `macos_linux_process_group_tree_termination_dynamic`；不宣布完成 |
| `dist/` ignore | `git check-ignore -v dist/devsweep-x86_64-pc-windows-msvc.zip` | 0（被忽略） | `.gitignore:5:/dist/` |
| Hosted CI | 未 push | — | **UNVERIFIED** |
