# 设计：Token、材质与外壳

契约是 `desktop-gpui/DESIGN.md`（由父任务 `research/DESIGN.md` 提升）。下面只写它落到代码上的方式。

## 边界

- 改动：`desktop-gpui/DESIGN.md`（新增）、`src/ui.rs`、`src/view.rs`、`src/view/page_*.rs`、`src/copy.rs`、`vendor/gpui_windows/src/window.rs`（只动模糊着色参数）。
- 不新增依赖。偏好文件格式、`Session` 状态机、动作与快捷键绑定、元素 ID 和无障碍名称都不变。
- 页面仍只绑定现有状态和操作。本任务不改任何页面的数据来源。

## Token 层（`ui.rs`）

| 契约 | 现有 `Theme` 字段 | 处理 |
| --- | --- | --- |
| field-start、field-end | `canvas`、`canvas_end` | 不变 |
| sheet、sheet-edge | `chrome` 与 `sheet()`、`sheet_edge()` | 不变；回退仍为不透明 |
| well | `surface` | 卡片改名为内容区，去掉描边 |
| 悬停与选中填充 | `surface_alt`、`surface_active` | 不变 |
| ink | `text` | 不变 |
| 次要文字 | `text_muted`、`text_subtle` | 合并为一个值 `#505F68`（深色 `#B3C2BF`） |
| input | `accent`、`pill` | 主动作、选中、焦点 |
| output | `info` | 结果数字及其底色 |
| warning、danger | 同名 | 只用于风险与失败 |

新增具名常量：字号 `TYPE_DISPLAY`、`TYPE_TITLE`、`TYPE_BODY`、`TYPE_META`；间距 `SPACE_1` 至 `SPACE_6`；圆角 `RADIUS_SHEET`、`RADIUS_WELL`、`RADIUS_ITEM`。纸的阴影用一条 `BoxShadow`（0, 20px, 48px, `rgba(70,90,120,0.08)`），替换 `shadow_sm()`。

## 组件

- `card()` 变为内容区：实色填充、16px 圆角、普通对比度下无描边。高对比保留 1px 描边。
- `stat()` 的数值用 `TYPE_DISPLAY` 和等宽数字。
- `button_base()` 与 `segment()` 的按压态改为填充加深，删除 `ui.rs:429`、`:790` 的下划线。
- `list_row()` 圆角 12px，只在键盘焦点时显示边。
- 新增 `page_bar(status, figure, actions)`：模式页的首行。删除模式页对 `page_header`（`view.rs:1358-1404`）的调用。设置、保护、规则、历史这些非标签页面保留一个 `TYPE_TITLE` 标题和返回按钮。
- 新增 `ui::display_path(&Path) -> String`：去掉 `\\?\` 与 `\\?\UNC\` 前缀，盘符大写。无障碍名称继续用原路径。

## 外壳（`view.rs`）

- 标签文字：把 `view.rs:935` 的条件从 `!compact || active` 改为"宽度不小于 720 逻辑像素（`ui::TAB_LABEL_MIN_WIDTH`，不随文字缩放变化）或当前选中"。`compact` 继续控制内边距。
- 纸占满高度：`mode-panel` 改为 `flex_1` 加 `min_h_0`，滚动容器从 `page-scroll` 移到纸内。提供 `sheet_frame(top, body, bottom)` 给各页使用：顶部固定首行，中间滚动，底部可选固定动作栏。
- 最大宽度 76rem、20px 圆角和标题栏命中测试不变。

## 主题

- `Theme::for_preferences` 增加一个参数表示系统当前是否深色，`DesktopTheme::System` 按它选择。来源优先用 GPUI 的窗口外观查询和外观变化回调；实施第一步确认固定版本里可用，否则在 `WindowMaterial::read_preferences`（`view.rs:98-106`）里读 `UISettings`。
- 设置页（`page_support.rs:447-470`）增加第三项"跟随系统"。标题栏的切换按钮（`view.rs:222-232`）仍在浅色与深色之间切换。
- `DesktopTheme` 的默认值、序列化名和其余映射不变。

## 材质

- `try_enable_window_blur` 是 vendor 里的 DevSweep 补丁。给它增加着色参数，传入当前主题的场色；根渐变的透明度提高到能通过 AC4 为止。
- 判定规则：浅色主题在纯黑背景上场色亮度达到 0.70 则保留着色背板；否则根渐变不透明，只保留纸面的半透明。两种结果都记录测量值。
- 回退矩阵沿用 `.trellis/spec/backend/desktop-runtime.md` 的"GPUI window material"一节。

## 兼容与回滚

- 测试里引用的元素 ID、`accessible_control` 槽位和 `panel_entries` 文本投影保持不变；确需改动的测试逐条说明原因。
- 回滚顺序：先撤材质参数，再撤外壳布局，最后撤 Token。`DESIGN.md` 可以保留。

## 风险

- 视图文件共约 4600 行，Token 迁移面大。按页面分批，每批跑一次测试。
- 纸占满高度会改变各页的滚动归属。Clean 审阅的翻页键（`clean_page_up`、`clean_page_down`）依赖滚动容器，迁移后要实机验证。
- 去掉描边后，高对比模式的边界必须保留。高对比色板单独验证。
