# 实施计划

开始前读：`.trellis/spec/backend/desktop-runtime.md` 的"GPUI native control focus"和"GPUI window material"两节，`.trellis/spec/backend/desktop-preferences.md`，父任务 `research/DESIGN.md`。

## 步骤

1. [x] 基线证据。构建当前 release，沿用归档任务 `11-04-gpui-airy-glass/evidence/native.ps1` 的方法，截英文与中文、浅色与深色的五个模式和设置页，存入本任务 `evidence/before/`。记录浅色主题在纯黑、纯白背景上的场色采样值。
2. [x] 把父任务 `research/DESIGN.md` 复制到 `desktop-gpui/DESIGN.md`。
3. [x] `ui.rs`：加入字号、间距、圆角常量和纸的阴影；合并次要文字；内容区去描边；按压态改为填充。更新 `text_and_control_edges_meet_contrast_on_paper_and_wells`。
   回滚点 A：到此只动了 Token 和基础组件。
4. [x] 逐页迁移到具名 Token，顺序为 `page_support.rs`、`page_workbench.rs`、`page_analyze.rs`、`page_status.rs`、`page_clean.rs`、`view.rs`。每页完成后运行 `cargo test --locked`（在 `desktop-gpui` 目录）。
5. [x] 外壳：标签文字规则、`sheet_frame`、`page_bar`；删除模式页的 `page_header` 调用。
   回滚点 B：外壳布局。
6. [x] 主题：`system` 的解析、设置第三项、外观变化时刷新。补测试：旧偏好文件往返不变；`system` 在两种系统模式下各得到对应色板。
7. [x] 材质：背板着色参数。按 AC4 测量，不达标则把根渐变改为不透明。测量值写入 `evidence/material.md`。
8. [x] 文案：`ui::display_path` 及其测试；软件来源的可读名称；清理中文界面里的英文诊断句（先检索 `copy.rs` 和 `software_review.rs` 里未经 `copy::pick` 的用户可见字符串）。
9. [x] 源码守卫测试（AC2）。
10. [x] 规格更新：`desktop-runtime.md` 的材质一节补上着色背板和亮度判定。

## 验证

```powershell
just gpui        # 格式、Clippy、GPUI 测试、release 构建
just ci          # 根工作区门禁
just gpui-dev    # 实机查看
```

- 原生截图：AC3 的矩阵，存入 `evidence/after/`，与 `before/` 同视口同状态对照。
- 对比度与场色亮度：AC4 的实测表。
- 回退：AC5 的三种情形各一张截图。
- 键盘与 UIA：AC7 的清单，在开启 UIA 的 debug 构建里执行。
- 高对比和透明效果的系统级热切换需要改 Windows 全局设置，列为人工验收步骤，恢复原设置后记录结果。

## 评审关口

- 步骤 5 完成后先给用户看前后对照截图，再继续步骤 6 以后。
- 结束前按 design-atlas 的判读清单过一遍，并做一次独立上下文评审，结论写入 `research/review.md`。没有子代理可用时标明"自检，非独立复核"。
- `slop_check.mjs` 不覆盖 Rust 源码，不作为通过证据。
