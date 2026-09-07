# 父任务发布门证据

核验日期：2026-09-07（Asia/Shanghai）。产品 HEAD 在记录时为 `93dc07c`（`chore(task): archive 09-07-evergreen-harness-alignment`）。分支 `dev`。父任务当时仍为未跟踪的 planning 目录，尚未归档。

本文件只记录父集成共享门禁。父任务不是产品实施目标；产品改动已由五个子任务的 `trellis-implement` 提交，并已独立归档。

未运行：`clean execute`、`software uninstall`、`optimize run`、`tinstall`、`install-all`、安装器、签名、发布、`git push`、`git commit --amend`、`git add -f .trellis/`。

## 五子归档

均位于 `.trellis/tasks/archive/2026-09/`，`task.json.parent` 仍为 `09-07-evergreen-five-harness-audit`（`find_task_by_name` 不搜索 `archive/`，因此归档父任务时不应把子 `parent` 写成 `null`）。

| 子任务 | 产品提交 | 归档提交 | 归档目录 |
| --- | --- | --- | --- |
| validation-gates | `8b4fdf9` | `d8b4ef2` | `09-07-evergreen-validation-gates` |
| cli-documentation | `33ea265` | `d9e7ef2` | `09-07-evergreen-cli-documentation` |
| release-contract | `ee373c6` | `96c5471` | `09-07-evergreen-release-contract` |
| skill-integrity | `49b0c5c` | `025687a` | `09-07-evergreen-skill-integrity` |
| harness-alignment | `ef29121` | `93dc07c` | `09-07-evergreen-harness-alignment` |

## `just ci`

父 shell 未设置 `CARGO_TARGET_DIR`。未删除现有 `target/`。

```text
python -c "import hashlib, pathlib; print('CARGO_LOCK_SHA256_BEFORE=' + hashlib.sha256(pathlib.Path('Cargo.lock').read_bytes()).hexdigest())"
just ci
python -c "import hashlib, pathlib; print('CARGO_LOCK_SHA256_AFTER=' + hashlib.sha256(pathlib.Path('Cargo.lock').read_bytes()).hexdigest())"
```

| 项 | 值 |
| --- | --- |
| `JUST_CI_EXIT` | `0` |
| 日志 | `ci complete` |
| `CARGO_LOCK_SHA256_BEFORE` | `73b7fe3189dd1c9c04b04f023755af950e6ce5636709608d22fb818d6d87a7a9` |
| `CARGO_LOCK_SHA256_AFTER` | `73b7fe3189dd1c9c04b04f023755af950e6ce5636709608d22fb818d6d87a7a9` |
| `Cargo.lock` 是否变脏 | 否 |

测试计数（与 `just ci` 输出一致）：CLI lib 183、cli_contract 12、five_mode 7、core 326（+1 ignored）、public_api 1、desktop 39（+2 ignored）。

Hosted CI：本轮未 push。**UNVERIFIED**。

## 独占 `--target-dir` workspace 测试

要求：全新独占输出目录；参数为 Cargo `--target-dir`，不 `export CARGO_TARGET_DIR`；测试子进程不继承该变量；不删除现有 `target/` 来模拟干净构建。

父 shell：`CARGO_TARGET_DIR` 运行前 `(unset)`，运行后 `(unset)`。未向 `cargo test` 传入该环境变量。

### 第 1–2 轮（TEMP，已换证据来源）

同一检查连续失败 2 次后改到仓库内 `/target/` 子目录（仍是全新独占目录，不删除 `target/debug`）。

| 轮 | `--target-dir` | 退出码 | 观察 |
| --- | --- | --- | --- |
| 1 | `C:\Users\lyh\AppData\Local\Temp\devsweep-ws-992c5c87-7578-43f3-83e8-79977ebe568c` | 101 | 与默认 `target/` 的 `process::tests` 并行；core lib 测试 exe「never executed」 |
| 2 | `C:\Users\lyh\AppData\Local\Temp\devsweep-ws-e802bc19-fbac-4ff8-905c-9121984b22ac` | 101 | 串行仍失败。`debug\deps\devsweep_core-9d550e53eac9cfb8.exe` 不存在；同名 `.pdb`（57610240）与 `.d` 存在。CLI 契约测试已通过 |

不把 TEMP 失败当作产品缺陷关闭本门。第 3 轮改到与源码同盘、且被 `/target/` 忽略的独占目录。

### 第 3 轮（采用）

```text
Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
$td = Join-Path (Join-Path (Get-Location) 'target') ('ws-exclusive-' + [guid]::NewGuid().ToString())
cargo test --workspace --locked --all-targets --target-dir $td
```

| 项 | 值 |
| --- | --- |
| `EXCLUSIVE_TARGET_DIR` | `D:\Documents\Code\Rust\Exp\devsweep\target\ws-exclusive-9a1a5474-967f-4822-bc65-a970201492c6` |
| `PARENT_CARGO_TARGET_DIR_SET` | `False` |
| `AFTER_CARGO_TARGET_DIR` | `(unset)` |
| `EXCLUSIVE_TEST_EXIT` | `0` |
| `DEFAULT_TARGET_DEBUG_STILL_PRESENT` | `1` |
| core 测试 exe | 存在，`LEN=9752064` |

包结果：CLI lib 183、cli_contract 12、five_mode 7、core 326（+1 ignored）、public_api 1、desktop 39（+2 ignored）。含 `process::tests::hung_child_and_grandchild_are_terminated`。

## Windows process-runner

```text
Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
cargo test --locked -p devsweep-core --lib process::tests -- --nocapture
```

| 项 | 值 |
| --- | --- |
| `PROCESS_TESTS_EXIT` | `0` |
| 结果 | `19 passed; 0 failed; 0 ignored; 308 filtered out` |
| 原始输出 | `research/windows-process-tests-nocapture.log` |
| 含 `hung_child_and_grandchild_are_terminated` | 是 |
| 含 `macos_linux_process_group_tree_termination_dynamic` | 否（该测试 `#[cfg(unix)]`，Windows 不编译） |

## Unix process-runner

无获准 Unix 环境，未 push，未创建远端 runner。

| 项 | 值 |
| --- | --- |
| 命令 | `cargo test --locked -p devsweep-core --lib process::tests -- --nocapture`（Unix） |
| 退出码 | 未运行 |
| 日志是否含 `macos_linux_process_group_tree_termination_dynamic` | 否 |
| 判定 | **UNVERIFIED** |

不宣布 Unix process-group 动态树终止完成。不把 Windows Job Object 通过当作 Unix 通过。

## `dist/` 忽略

本次归档文件（release-contract 构建，未运行安装器）：

```text
git check-ignore -v dist/devsweep-x86_64-pc-windows-msvc.zip dist/devsweep-x86_64-pc-windows-msvc.zip.sha256
```

| 路径 | 输出 |
| --- | --- |
| `dist/devsweep-x86_64-pc-windows-msvc.zip` | `.gitignore:5:/dist/` |
| `dist/devsweep-x86_64-pc-windows-msvc.zip.sha256` | `.gitignore:5:/dist/` |

`git status --short --ignored` 将 `dist/` 列为 `!! dist/`。

## 范围外保留

未纳入提交：`tools/__pycache__/`、`tools/tests/__pycache__/`、其他活动或未跟踪任务目录、范围外脏文件。未修改 `.trellis/scripts/`。

## 仍为 UNVERIFIED 的表面

- Hosted CI（未 push）
- Unix `macos_linux_process_group_tree_termination_dynamic`
- 原生 TUI / 交互桌面会话
- Claude Code / Codex / Kimi / OMP 新会话实际加载
- Grok hook/agent **执行**（检查配置 ≠ 运行）
- 安装器、签名、发布
