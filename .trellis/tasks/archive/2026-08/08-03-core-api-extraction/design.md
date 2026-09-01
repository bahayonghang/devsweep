# 技术设计:workspace 拆分与 core 公开面

> 细化父任务 `design.md` §1-§2、§5。本任务为纯移动式重构,无逻辑变更。

## 1. 目标布局

```
Cargo.toml                    [workspace] members;[workspace.package] 统一
                              version / edition / rust-version / license
crates/devsweep-core/
  src/lib.rs                  pub mod scan / plan / execution / model / rules /
                              inventory / filesystem / process / cargo_metadata
                              + pub services(自 tui/runtime/services.rs 上移)
crates/devsweep-cli/
  src/main.rs                 现 main.rs
  src/lib.rs                  仅 pub fn run()(现 lib.rs 语义)
  src/application/ …          现 application/、tui/ 原样迁移
  src/bin/process_fixture.rs  原样迁移
```

## 2. 可见性策略

- core 内模块从 `pub(crate)` 提升为 `pub` 的范围 = CLI 命令层
  (`application/commands.rs`)与 services trait 签名所引用的类型闭包;
  其余保持 crate 私有。逐类型清单在实现时以编译器驱动收敛,不预先枚举。
- services trait 从 `pub(in crate::tui)` 上移到
  `devsweep_core::services`,TUI 侧改为 `use devsweep_core::services::*`。
- `unreachable_pub` lint 保留在两个 crate。

## 3. 依赖切分

- core:anyhow、serde、serde_json、sha2、rayon、trash、tracing、
  windows-sys(cfg windows)、libc(cfg unix)。
- cli:core + clap、ratatui、crossterm、unicode-width、tracing-subscriber。
- dev-dependencies(tempfile)按测试归属分配。

## 4. CI / justfile 适配

- justfile:check/test/clippy 改 `--workspace --all-targets --locked`;
  fmt 不变;`dev` / `release-archive` 指向 `-p devsweep-cli`。
- ci.yml MSRV 作业:`rust-version` 移入 `[workspace.package]`,sed 改读根
  Cargo.toml 的 `rust-version`(workspace.package 段仍在根文件,sed 模式需
  核对匹配)或直接读 `crates/devsweep-core/Cargo.toml`,实现时二选一并留档。

## 5. 兼容与回滚

- 单 PR / 单批提交完成迁移;任何验收门失败 → 整体 revert。
- 不发布 crates.io;包名 `devsweep-cli` 的 `[[bin]] name = "devsweep"` 保证
  二进制名不变。
