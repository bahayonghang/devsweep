# 集成验收与归档

9 个子任务已分别验收归档。父任务 AC1–AC8 通过。差距表 33 项中，31 项已解决，Downloads 安装包与清空回收站按范围保留差异，跨扫描增长比较延后。

[集成验收、各域命令与逐项差距](research/integration-review.md) · [24 张原生截图总览](research/native/matrix.jpg) · [门禁日志](research/integration-gates.zip)。

| 子任务 | 归档证据 |
| --- | --- |
| R1 桌面外壳 | [查看](../11-12-gpui-airy-glass-shell/evidence/material.md) |
| R2 Status | [查看](../11-12-gpui-status-dashboard/evidence/status.md) |
| R3 软件清单 | [查看](../11-12-gpui-software-evidence/evidence/native-acceptance.md) |
| R4 主机动作 | [查看](../11-12-gpui-host-actions/result.md) |
| R5 Clean 审阅 | [查看](../11-12-gpui-clean-review-flow/evidence/native-acceptance.md) |
| R6 Windows 缓存 | [查看](../11-12-clean-windows-cache-coverage/research/integration-check.md) |
| R7 Analyze | [查看](../11-12-gpui-analyze-insights/result.md) |
| R8 Optimize | [查看](../11-12-gpui-optimize-batch/result.md) |
| R9 托盘 | [查看](../11-12-gpui-tray-panel/result.md) |

最终 just ci 通过 943 项、3 项按配置忽略。just gpui 通过主程序 262 项与画廊 4 项，2 项需显式启用的原生取证用例已分别执行。两个 Rust 工作区的格式、检查和 Clippy 均通过。release SHA-256 为 FE2F166AB2D3AE12E8D2A59D3536BF599373C40DD04B60B817648F7F3FA9F5EC。Python 工具测试 23 项通过。

工作保存在独立工作树 D:/ds-integrate-1120，分支 integration/mole-parity-1120，合并输入为 264e7ac 与 324e72b。本次合并和产品改动未提交或推送。

原生窗口、语言、主题、鼠标、键盘和指定 UIA 路径均使用任务自己的数据。真实软件卸载、第三方 UAC 及主机维护未执行。托盘图标恢复采用用户确认的 TaskbarCreated 消息复验。

归档后再次验证父子关系、9 份已完成的子任务元数据、8 条父任务验收、33 个差距编号与 24 张截图链接。父任务和九个子任务共十套 Trellis 上下文清单均通过 task.py validate。task.py list 显示 0 个活动任务。
