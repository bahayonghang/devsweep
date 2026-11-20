# R6 实施与定向验收

2026-11-20，Windows，`integration/mole-parity-1120`。

Chrome Beta、Canary（目录 `Chrome SxS`）、Edge Beta、Dev 各有独立规则 ID。core 复用现有 Chromium 登记校验，只允许默认根中已登记 profile 的直接 `Cache` 和 `Code Cache`。发现同品牌任何同名进程，或无法证明进程空闲时，该预发布通道生成零目标，并在覆盖记录中提示关闭后重新扫描。扫描、预览与执行继续走既有规则表和实时核验。

中英文 `clean rules list` 和 `clean rules show --id` 显示通道名称与清理范围。JSON v1 保留现有字段，四条规则均为全局、Windows、`trash`、中风险。实际渲染代码沿用原有 GPUI 分组与终端列表。

| 定向验证 | 结果 |
| --- | --- |
| `devsweep-core --lib` 的 `rules` 过滤器 | 31 项通过。四个通道逐一覆盖两个登记配置、未登记目录、受保护数据、伪造元数据、运行中和未知所有者。 |
| `execution::tests::windows_cache_pipeline` | 3 项通过。临时目录子进程穿过全局扫描、保存计划、真实验证器、预览和记录式执行器，核对新增八个叶路径、busy/unknown 零目标与既有遗漏表。测试执行器接收路径，不触及真实浏览器缓存。 |
| `devsweep-cli --test cli_contract` | 全部 19 项通过，覆盖四条规则的 JSON 和中英文列表及详情。 |
| `devsweep-cli --lib` 的 `chromium_preview_channel` | 1 项通过。TUI TestBackend 实际绘制并逐行按向下键，四条规则均能在详情显示通道、规则 ID 和全局范围，初始勾选为零。 |
| GPUI `chromium_preview_channel_scan_rows_reach_grouped_global_review` | 1 项通过。受控 ScanReport 的四个可选行进入全局分组，并生成中英文无障碍标签。R7 补齐共享 `ShellView` 构造后重跑，输出在 `dist/verify-1120/r7-r6-gpui.log`。该用例验证交付给界面的数据，未拍摄系统原生窗口。 |

`scan-coverage.md` 已补充路径、所有者和复验契约。`git diff --check` 已通过。独立复核与 `just ci` 由父任务在集成树上登记。
