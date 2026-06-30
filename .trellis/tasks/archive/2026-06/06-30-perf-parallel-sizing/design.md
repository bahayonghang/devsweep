# Design — 性能与构建优化

## 1. 共享模块

新增 `src/fs_size.rs`,在 [src/lib.rs](../../../src/lib.rs) 注册 `pub mod fs_size;`。对外暴露:

```rust
pub fn estimate_tree(path: &Path) -> (u64, Option<SystemTime>);
pub(crate) fn is_unsafe_link(metadata: &fs::Metadata) -> bool;
// has_windows_reparse_point 作为内部实现,#[cfg(windows)] / #[cfg(not(windows))] 两版保留
```

- [src/scanner.rs](../../../src/scanner.rs):删除本地 `estimate_tree` / `is_unsafe_link` / `has_windows_reparse_point`,改 `use crate::fs_size::{estimate_tree, is_unsafe_link};`。`is_real_dir`、`scan_dir` 中对 `is_unsafe_link` 的调用保持不变。
- [src/providers.rs](../../../src/providers.rs):同样删除本地副本并改用共享模块;`SystemProviderProbe::estimate_tree` 转调 `fs_size::estimate_tree`。
- 语义零变更:文件计 `len()`;symlink 与 Windows reparse point **不下降、不计字节**,但保留其自身 mtime(与现状一致)。

## 2. 并行策略(方案 B:逐层 rayon,语义可证等价)

保留现有"我们自己控制递归、显式跳过 symlink/reparse"的结构,只把每个目录的子项遍历并行化:

```rust
use rayon::prelude::*;

pub fn estimate_tree(path: &Path) -> (u64, Option<SystemTime>) {
    let Ok(meta) = fs::symlink_metadata(path) else { return (0, None) };
    if is_unsafe_link(&meta) { return (0, meta.modified().ok()) }
    if meta.is_file() { return (meta.len(), meta.modified().ok()) }
    if !meta.is_dir() { return (0, meta.modified().ok()) }

    let self_mtime = meta.modified().ok();
    let Ok(entries) = fs::read_dir(path) else { return (0, self_mtime) };
    let children: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();

    let (bytes, latest) = children
        .par_iter()
        .map(|child| estimate_tree(child))
        .reduce(|| (0u64, None), combine);

    (bytes, max_mtime(latest, self_mtime))
}
```

- `combine((b1,m1),(b2,m2)) = (b1+b2, max_mtime(m1,m2))`,幺元 `(0, None)`。
- `max_mtime`:两者取较新;`None` 视为最小。这与现有串行实现"latest = max(当前, 子项)"完全一致。
- 加法、`max` 均满足交换律/结合律 → 并行 reduce 结果与串行顺序无关,**字节合计与最新 mtime 与原实现逐位一致**。
- rayon 默认用全局线程池,递归调用通过 work-stealing 自然并行,无需显式 `join`。

**为何不选方案 A(walkdir + `par_bridge`,putzen 写法)**:walkdir 需额外 `filter_entry` 才能跳过 Windows reparse point,且默认只求 size、不顺带 mtime;改造后等价性更难论证。方案 B 改动面更小、语义可证等价,符合 CLAUDE.md「最小改动」。性能子任务若实测发现逐层开销过大,再评估方案 A。

### 小目录开销

`par_iter` 对极小目录有调度开销。首版不做阈值优化;若实测回退,可加「子项数 < N 时走串行 for」的快路径(记入 implement 的可选项),不改变对外行为。

## 3. Release profile

[Cargo.toml](../../../Cargo.toml) 追加:

```toml
[profile.release]
lto = true
codegen-units = 1
strip = true
```

- `panic = "abort"`:**默认不加**。devsweep 依赖 `trash`、`anyhow`,worker 线程的 panic 在 abort 下会直接终止进程;收益(体积/速度)有限而风险存在。先验证前三项;若后续确有体积需求,单独评估。
- 新依赖:`rayon = "1"`(参考 putzen 用 1.11)。

## 4. 测试设计

- 在 `fs_size.rs` 内 `#[cfg(test)]`:用 `tempfile` 造多层目录 + 已知字节,断言 `estimate_tree` 字节数精确、`latest` 等于最新文件 mtime。
- 等价性:对同一棵树,断言并行结果与一个本地串行参照实现一致(参照实现可写在测试内)。
- symlink/reparse:复用 [src/scanner.rs](../../../src/scanner.rs) 既有用例风格(`#[cfg(unix)]` symlink、`#[cfg(windows)]` reparse),断言被跳过、不计字节。
- 既有 scanner/providers 测试必须不改语义地通过。

## 5. 兼容性

对外契约(`CleanupPlan` JSON、CLI、TUI 行为)**零变更**;纯内部重构 + 加速 + 构建配置。无需升 `CLEANUP_PLAN_VERSION`。
