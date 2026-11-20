# DevSweep #6 · Clean 页 Windows 签收

维护者已签收 1240×820、720×560 两种窗口尺寸各自的浅色与深色对照。界面使用中文只读审阅夹具，系统缩放为 125%。Clean 实现在本地集成分支的 `c74f749`，验收报告记录在 `4b9062e`。[签收评论](https://github.com/bahayonghang/devsweep/issues/6#issuecomment-7001256767)

- 四组原生图均与同尺寸的 design-atlas 0.7.0 参考页并排检查。纸面之间可见连续色斑，输入框和复选框有独立边界，选中记录用填充区分。与 SkillPort 对照时，Clean 的横向工具栏和固定底栏占用更多纵向空间，窄窗首条记录需滚动或 Tab 才能看到。
- 两种窗口尺寸均验证了向前 Tab 至第 21 站时整行和焦点环可见。还检查了悬停、禁用按钮、失败状态、应用内减少动效和调试高对比度。真实 Shift+Tab、Windows 全局辅助功能变化通知、屏幕阅读器全流程及 macOS/Linux 原生界面仍未验证。
- mint 浅深色两组的 56 个颜色和透明度值与生成结果一致。最差纸面文字对比度为浅色 12.16 / 4.96、深色 13.04 / 6.90。控件边界为浅色 3.17、深色 3.24。Rust 源目录没有 slop_check 可扫描的文件，返回码为 2。
- 原签收版本两次通过 Windows `just gpui`，每次运行 243 项产品测试和 4 项画廊测试，并完成 release 构建。公开分支从远端 `dev` 提取了签收所需的 Clean 审阅、夹具和焦点代码；其门禁结果见交付 PR。[签收版复验](https://github.com/bahayonghang/devsweep/issues/6#issuecomment-7001210406)
- 本地签收分支 `feat/airy-glass-material` 的 `.trellis/tasks/11-18-gpui-clean-airy-glass-material/evidence/shots/` 保存四组 `pair-{light,dark}-{1240,720}.png`、同尺寸 `pair-skillport-1240.png` 和焦点证据。公开画廊可以用 `just gpui-gallery` 运行。[材质画廊 A–F 实测](https://github.com/bahayonghang/devsweep/issues/5#issuecomment-6932241960)
