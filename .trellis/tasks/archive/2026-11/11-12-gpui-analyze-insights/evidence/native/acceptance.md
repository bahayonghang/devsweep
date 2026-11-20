# Windows 原生验收

任务程序在独立的非输入桌面运行。每次只分析其独立临时配置中登记的项目目录，含 120、60、30 MiB 三个文件。窗口为 125% DPI。UIA 观测来自真实 AccessKit 树。所有测试进程已退出。

| 验收 | 观测 |
| --- | --- |
| 宽窄布局 | [1240 × 820 中文浅色](zh-light-wide.png) 的目录列表与树图左右排列。[720 × 700 中文深色](zh-dark-narrow.png) 上下排列。[窄布局的树](narrow-layout.json)与[宽布局的树](wide-layout.json)保留实际控件坐标。点击目录行后相应矩形和行高亮。[中文最大文件](zh-dark-largest.png)显示快照保留范围。 |
| 其余配色 | [英文浅色](en-light-wide.png)、[英文深色](en-dark-wide.png)。 |
| 三项预览及改选 | [预览截图](three-node-preview.png)和[同一时刻的控件](three-node-preview.json)包含三个已选项与可用确认。再次切换一个复选框后，[控件](changed-selection.json)只剩两项选中，需要重新预览。执行授权由 core 另行检查。该原生环节止于预览。 |
| 同一行菜单与焦点 | [右击](mouse-menu.png)、[Windows 菜单键](apps-menu.png)、[Shift+F10](keyboard-menu.png)实际打开同一菜单。第一次检查发现 Escape 会退回位置选择，修复后在一个新进程右击、Escape、菜单键、Escape、Shift+F10，期间未再点击或聚焦行，即可连续从原行打开菜单。两次向下和 Enter 后菜单关闭。私有桌面的 UIA 键盘焦点属性为 false，焦点恢复以实际连续键盘行为验证。[逐步操作与窗口/程序指纹](menu-input.txt)。 |

布局、选择与审阅图来自普通 debug 程序 283A406CA6BA3C30552F21F176D6D594614F2ADCA0C002482A9E8823C1AB1650，菜单复验为 D325B433025E7912DA03482457369BCEE3881E55700D21F9649D4E9679CBF658。[图片校验值](screenshots.json)。

另用正常 CLI 对同一个隔离目录执行 analyze scan，再运行 analyze largest，三项依次为 125829120、62914560、31457280 字节，证据为 complete、retained_files_only 为 true。[榜单 JSON](largest-cli.json)。对已知根和一个不存在的同级路径运行 analyze roots，得到部分成功（退出 5）、当前用户可用容量及缺失路径原因。[根和容量 JSON](roots-cli.json)。重复提供三个 --select 参数保存计划，然后运行 trash-preview，返回恰好三个条目、一个 sha256 摘要及零拒绝项。[CLI 预览](three-node-cli-preview.json)。
