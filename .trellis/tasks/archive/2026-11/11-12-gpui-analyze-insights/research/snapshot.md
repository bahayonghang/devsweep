# Analyze 快照、容量和菜单调研

- 问题：已存快照能回答哪些最大文件问题，怎样显示卷容量，以及桌面菜单和多选应接在哪里？
- 范围：集成工作树的 core、CLI、GPUI 与固定的上游 GPUI；只读研究。
- 日期：2026-11-20。

## 快照究竟保留什么

`analysis/model.rs:9–16`（本节相对 `crates/devsweep-core/src/`）限制一次遍历为 250,000 个节点、10,000 条告警、256 MiB 记账内存、1,024 层。节点上限包含目录和重解析点。遍历逐个安排目录的孩子，全局预算不足即停止，没有每目录按大小保留前若干个的逻辑。深度上限只标记受影响的父链。`analysis/walker.rs:672,818,896–949`。

`analysis/walker.rs:567–578` 的完成类型只区分正常停止、预算耗尽、取消。正常停止也可能有拒访、文件变化、深度截断和节点级不完整证据；重解析点作为叶，硬链接按一次归属计量。总是告知“仅限快照中保留的文件”，另报告快照完成类型、根证据和有无省略。目录、重解析点、未知容量不当作已知文件大小；保留节点自身的证据。`analysis/model.rs:179–257`。

最大文件在现有快照上作只读投影，稳定键为容量降序、相同容量按完整路径升序；相同键可加 ID。以定长堆保存前 K，阈值和 limit 的默认值及硬上限在命令帮助中写明。现有分页限制是默认 100、至多 1,000，可作 limit 先例。校验快照与路径只建一次节点索引。现有 `analyze_node_path` 每次构造 `NodeIndex`，用它对所有文件循环会造成二次量级开销。旧快照仍可读，无需改 V1 快照格式。`analysis/observe.rs:21–27`、`analysis/actions.rs:203–210,285`。

## 容量与界面

`analysis/observe.rs:201–306` 已有可注入 RootSource，显式指定的路径只探自身。应在身份可读后独立查询容量并保留每个位置与别名。沿用 Windows 的 `GetDiskFreeSpaceExW`，区分调用者可用空间和整卷空闲空间。在有磁盘配额时，第二输出也是调用者可见的总量，宜与第一输出（调用者可用量）配对并说明口径。Status 当前用第二和第三输出，直接复用其名为 available 的值会混淆配额。查询失败给未知原因，容量失败不应让有效目录变成不可导航。旧 V1 根记录缺新字段可反序列化。覆盖可读、失败、有用户配额、真实零可用、旧记录。参见 `status/system.rs:336–357` 与 [Windows API 文档](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getdiskfreespaceexw)。

`desktop-gpui/src/analyze.rs:32,57–94` 只有一个选中节点，`recycle_ids()` 将 Option 转成最多一项的列表。把键盘焦点/当前节点与多选集合分开，行与树图共享 ID 集，选择/快照变更清掉预览。不丢失 core 对受保护路径与父子重叠选择的最终裁决。现有 preview/confirm 入口在 `desktop-gpui/src/analyze_window.rs:303,343`。

`desktop-gpui/Cargo.toml:17–18` 锁定 Zed `322cd509dcccaee1f6bc44725d270598ae8171ad`。它的 `crates/gpui/src/elements/anchored.rs:27` 与 `elements/div.rs:869,980` 提供锚定、右键与外部点击。完整的 ContextMenu 位于独立的 `crates/ui/src/components/context_menu.rs:213`。可用现有依赖构造行/树图锚定的焦点菜单，每次绑定明确 ID；右键、Menu 和 Shift+F10 走同一路，方向键/Enter/Escape 可用，关闭恢复原焦点。Windows 后端在 `desktop-gpui/vendor/gpui_windows/src/events.rs:1669,1679` 映射 `menu` 和 `f10`。[固定版 GPUI](https://github.com/zed-industries/zed/tree/322cd509dcccaee1f6bc44725d270598ae8171ad/crates/gpui)

## 规范与证据边界

参见 `.trellis/spec/backend/bounded-delivery.md`、`project-occupancy.md`、`.trellis/spec/guides/cross-layer-thinking-guide.md` 和 `desktop-gpui/DESIGN.md`。以上是代码和上游接口调研；未运行真实磁盘分析、回收、菜单或原生键盘验收。
