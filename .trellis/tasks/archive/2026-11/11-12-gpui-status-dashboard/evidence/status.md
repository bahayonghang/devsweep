# Status 仪表盘原生验收（部分 B）

2026-11-13 首次采集，2026-11-14 在检查修复后重采。本机 Windows 10.0.26300，24 个逻辑处理器，窗口 DPI 120。窗口缩放到 1080×720 逻辑像素（1350×900 物理像素）。每个用例用隔离的 LOCALAPPDATA 启动 release 二进制，通过 UIA 切换标签，只截本进程窗口。脚本：[capture.ps1](capture.ps1)。原始记录：[capture.txt](capture.txt)（改动后）、[capture-baseline.txt](capture-baseline.txt)（改动前）。

- 改动后：检查修复后工作树 `just gpui` 产出的 release，SHA-256 `93F07E02…71A1C59E`。
- 改动前：`HEAD`（`ac36090`）的根 `Cargo.toml`、`crates/devsweep-core` 和 `desktop-gpui` 用 `git archive` 导出，用同样的 `RUSTFLAGS` 构建 release，SHA-256 `C9E11AC1…86BC6`。

## 截图

| 文件 | 内容 |
| --- | --- |
| [status-en-dark-2s.png](status-en-dark-2s.png) | 进入 Status 后约 2.0 秒截取，没有点击任何按钮。首行显示 2 个样本，八个格子都有读数。趋势线只有 2 个样本，所以只在右边缘出现一小段 |
| [status-en-dark.png](status-en-dark.png) | 采样约 12 秒后：12/60 个样本。健康格显示 "CPU not busy"、CPU 型号、Windows 10.0.26300.9974 (26H2)、已运行时间和 "9 readings, 5 not reported. No score."，没有分数。CPU 格带频率、逻辑处理器数和最高温区；GPU 格带适配器名称并注明 "busiest of 4 adapters"（名称与百分比是同一个最忙的适配器）；内存格带提交内存；电源格带健康度 100%、"cycle count not reported" 和功率；可用空间格带 C: 总量和条形；网络格带连接类型 "Wi-Fi"，速率只计以太网与无线硬件接口（过滤层与隧道不重复计入）；磁盘活动格带读写速率 |
| [status-zh-dark.png](status-zh-dark.png)、[status-zh-light.png](status-zh-light.png) | 同一页面的中文版。新增指标和缺失原因都是中文（例如 "未报告循环次数"、"无线网络"、"4 个适配器中最忙"、"提交 60.7 GB / 91.5 GB"），只有硬件名称、Windows 版本号和单位是原文 |
| [status-en-light.png](status-en-light.png) | 浅色英文 |

本机有电池，也有温度读数，所以截图里看不到"无电池""无温度传感器"的格子。这一情形（AC6）由单元测试 `missing_battery_and_sensors_show_their_reason_not_zero` 覆盖：使用 CLI 契约夹具（无电池、温度不可用）和一份全部缺失的快照，中英文都断言格子显示原因文字、不是 0% 或空白，且中文没有英文单词。

## AC7：空闲进程 CPU 时间

方法沿用 `11-04-gpui-airy-glass/evidence/native.ps1`：读取被测进程的 `TotalProcessorTime`，间隔 3 秒再读一次。Windows 的计时粒度是 15.625 ms，所以 0 与 15.625 是同一量级。

| 状态 | 改动前 | 改动后，2026-11-14 重采（en-dark / zh-dark / en-light / zh-light） |
| --- | --- | --- |
| Clean 落地页，从未进入 Status | 0 ms | 31.3 / 46.9 / 156.3 / 140.6 ms |
| Status 非实时（改动前：进入时生成一次快照后；改动后：点"暂停"后） | 0 ms | 0 / 15.6 / 0 / 0 ms |
| 离开 Status 回到 Clean | 15.6 ms | 31.3 / 0 / 0 / 0 ms |
| Status 实时（采样约 12 秒后），只记录不设门限 | 无此状态 | 62.5 / 62.5 / 109.4 / 0 ms |

2026-11-13 首次采集的改动后数值为：Clean 15.6 ×4；暂停 0 / 0 / 0 / 15.6；离开 0 / 0 / 15.6 / 0；实时 93.8 / 46.9 / 31.3 / 125 ms。

结论：暂停与离开后的非实时状态与改动前同一量级（0–31 ms / 3 s）。重采中 Clean 落地页在浅色两例达到 140–156 ms，此时 Status 尚未进入、采样器未启动，Clean 页面也没有改动；首次采集同一状态为 15.6 ms，判断为启动后不久的一次性开销或主机负载，未单独复现。实时采样开销在两次采集中为 0–125 ms / 3 s，即单核 0–4%；一例为 0 说明单次采样低于计时粒度。线程数：暂停后少 1 个（采样线程已 join），离开后与进入前相同或少 1。

## 订阅生命周期（AC4、AC5）

- 单元测试 `status_samples_while_its_page_is_visible_and_stops_when_it_is_not`：进入即开始计数 1，每次渲染不重复开始；暂停停止计数加一且趋势不增长；离开停止计数加一，之后 `poll_status_live` 读不到任何样本；再次进入清除暂停。
- 单元测试 `the_window_spawns_one_sampler_on_entry_and_joins_it_on_leave`（仅 Windows，真实采样器）：进入后不点击，第一个样本 855 ms 到达；离开后停止计数为 1，等 1.5 秒没有新样本。
- 单元测试 `trends_keep_the_last_sixty_samples_and_do_not_grow_while_paused`：120 个样本后每条趋势恰好 60 个，最旧的是第 60 个；暂停时不追加。
- 原生截图：2 秒时 2/60，12 秒时 12/60（首次采集为 13/60）。

## 未覆盖

- 原生运行没有连续跑满两分钟；"恰好 60 个"只由单元测试证明。
- 进程表（列：名称、内存、CPU、PID、磁盘读取、磁盘写入）在 720 逻辑像素高的窗口里位于首屏以下，截图只拍到表头区域和计数行。列顺序由代码和源码检查保证，没有原生截图。
- 窄窗口两列布局（小于 52 rem）只在代码中实现，没有截图。
- 高对比度主题没有截图。
- UIA 不按名称暴露指标格，所以首个读数的原生延迟用 2 秒截图证明，不是逐毫秒测量。
- 视觉评审为自检；代码由独立检查子代理复核，并修复了网络重复计数与 GPU、健康标题的文案。
- 健康标题的"繁忙"阈值来自 core 的健康判定，CPU 45% 时显示 "CPU not busy"。
