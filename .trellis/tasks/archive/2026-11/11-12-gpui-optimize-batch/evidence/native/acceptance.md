# Windows 批量维护验收

两个真实 GPUI 窗口分别使用英文浅色和中文深色，窗口逻辑尺寸 1240 × 820，DPI 125%。确认前先从 UIA 核对隔离适配器标记。两次均从七项全未选开始，依次选择时钟采样、SSD 重新修剪、刷新 DNS。计划审阅后只确认一次。适配器将第一项报告为失败、第二项报告需要管理员权限、第三项成功。未调用任何实际维护工具。

| 语言 | 计划与结果 | 无障碍审阅 | 对应审计 |
| --- | --- | --- | --- |
| 英文 | [选择](en-catalogue.png) · [审阅](en-review.png) · [结果](en-result.png) | [审阅](en-review-uia.json) · [结果](en-result-uia.json) | [三项终态](en-batch.json) · [CLI 历史](en-history.json) · [CLI 总计](en-totals.json) |
| 中文 | [选择](zh-catalogue.png) · [审阅](zh-review.png) · [结果](zh-result.png) | [审阅](zh-review-uia.json) · [结果](zh-result-uia.json) | [三项终态](zh-batch.json) · [CLI 历史](zh-history.json) · [CLI 总计](zh-totals.json) |

首轮原生窗口显示了三个结果，但 Windows UIA 的结果组无名称且读不到各项。修正后用两个全新隔离目录重复整批。两种语言都能读出每项名称、状态及适用的原因。每批三条终态有不同操作 ID，共享同一个批次 ID，history totals 对每批均报告一个成功项且结果精确。

复验使用显式启用 optimize-fixtures 的可执行文件，SHA-256 为 8D7DA2105137FA04E99A84FB5522EA16BDEA9710884D87CA23DDCAC3C8F70A2D。

普通构建（无夹具 feature，SHA-256 为 EB4B155F37C6DD2919B0DFBB6B3D1A2C5298B873F8341B175896DE7D8BFE7788）在[四次实际启动](defaults.json)中均显示七个未勾选项，审阅按钮不可用。[英文浅色](en-light-ordinary.png) · [英文深色](en-dark-ordinary.png) · [中文浅色](zh-light-ordinary.png) · [中文深色](zh-dark-ordinary.png)。

修正批次聚合后，还用新 CLI 查询了此前原生夹具留下的同一份历史。旧审计缺少计划项数，返回原批次 ID 并标为 partial / unknown，可以查看已记录的各项。[旧格式批次详情](legacy-batch-show.json)。

最终再用包含 batch_size 的夹具程序（SHA-256 C64B765E67C20F8618BFE559CBA50EA143F56D92635BD106D35AF2E01B760CBE）分别在两种语言中从零选到确认完成。每批九条审计事件均绑定同一个批次 ID 和三项计划，查询该 ID 返回该批次自身的摘要，包含 failed、skipped、succeeded 三种稳定结果，unknown 为 false。两次累计成功项均为 1。[英文结果图](en-aggregate-result.png) · [英文批次详情](en-aggregate-show.json) · [英文累计](en-aggregate-totals.json) · [中文结果图](zh-aggregate-result.png) · [中文批次详情](zh-aggregate-show.json) · [中文累计](zh-aggregate-totals.json)。
