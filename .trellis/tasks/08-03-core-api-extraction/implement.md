# 执行计划:workspace 拆分

## 前置

1. 读 `code_map.md` 与父任务 `research/devsweep-current-state.md`。
2. 基线留档:构造合成 fixture(脚本 `tests/fixtures/make_scan_fixture.*`,
   内含 node_modules/、Cargo.toml+target/、.venv/、__pycache__ 等代表性目标,
   内容确定性生成),跑
   `cargo run -- scan --json --roots <fixture> | jq -S . > baseline.json`,
   同时记录需剔除的易变字段清单(耗时、时间戳类;以实际输出核对)。

## 步骤(每步后 `cargo check` 保持可编译)

1. 根 Cargo.toml 改 `[workspace]` + `[workspace.package]`;新建
   `crates/devsweep-core`、`crates/devsweep-cli` 空壳。
2. 移动 core 模块九件套(scan/plan/execution/model/rules/inventory/
   filesystem/process/cargo_metadata)到 core;`pub(crate)` → `pub`
   以编译器报错驱动最小化提升。
3. services trait 上移至 `devsweep_core::services`;TUI 改引用。
4. 移动 application/、tui/、main.rs、bin/ 到 cli crate;lib.rs 收敛。
5. 依赖切分按 design.md §3;确认 `cargo tree -p devsweep-core` 无
   clap/ratatui/crossterm。
6. justfile 与 ci.yml 适配(design.md §4)。
7. rustdoc 一句话补齐(仅新公开项)。

## 验证(最后一轮全量)

```bash
just ci
cargo run -p devsweep-cli -- scan --json --roots <fixture> | jq -S . > after.json
diff baseline.json after.json          # 剔除留档字段后应为空
cargo tree -p devsweep-core | grep -E "clap|ratatui|crossterm"   # 应无输出
```

CI 三平台 + MSRV:推送分支观察,或本地 `rustup run <msrv> cargo check --workspace`
等价复现,方式与结果留档到本目录 `research/`。

## review 门

- 验收全绿后停下,等待人工 review 通过再进入 Phase 3。

## 回滚点

- 任一验收门失败且无法当场定位:`git revert` 整批,回到单 crate 布局。
