# 设计：状态指标与实时仪表盘

## 等价 CLI

| 桌面动作 | core 调用 | CLI |
| --- | --- | --- |
| 进入页面开始采样 | `status::spawn_live(LiveRequest)` | `devsweep status live --format ndjson` |
| 刷新一次 | `status::capture_snapshot` | `devsweep status snapshot --format json` |
| 健康证据 | `status::explain_health` | `devsweep status health` |

桌面端在进程内调用 core，不派生 CLI。新字段必须先能从 CLI 的 JSON 里看到，再接到界面。

## core 契约（`crates/devsweep-core/src/status/`）

现有 `StatusSnapshotV1`（`mod.rs:195-208`）的每个块都是 `AvailabilityV1<T>`。新增内容沿用这个形状：

| 新增 | 位置 | 说明 |
| --- | --- | --- |
| `host: AvailabilityV1<HostV1>` | 快照新块 | `cpu_model`、`os_version`、`boot_time_unix_ms` |
| `disk_activity: AvailabilityV1<DiskActivityV1>` | 快照新块 | 系统级读、写字节每秒 |
| `frequency_mhz: Option<u32>` | `CpuV1` | 当前频率 |
| `commit_used_bytes`、`commit_limit_bytes: Option<u64>` | `MemoryV1` | 提交内存 |
| `name: Option<String>` | `GpuAdapterV1` | 适配器名称 |
| `full_charge_basis_points`、`cycle_count`、`rate_milliwatts` | `PowerV1`，均为 `Option` | 电池健康与功率 |
| `kind: Option<NetworkKind>` | `NetworkInterfaceV1` | 以太网、无线或其他 |

- 采集方式在实施第一步逐项调研并写入 `research/collectors.md`：只用只读、不需要提权的系统接口；`status/pdh.rs` 已有只读 PDH 探针可以复用。某项在调研后确认拿不到，就保持不可用并写明原因，不为它引入驱动或服务。
- 版本策略：先读 `.trellis/spec/backend/` 里关于版本化数据契约的规则。纯增量的可选字段若允许留在 V1，就留在 V1；否则出 V2 并保留 V1 读取。结论写入 `research/collectors.md`。
- 不可用状态沿用 `UnsupportedCapabilityV1` 的原因码。

## 消费方

- CLI：`application/commands/status.rs` 的人类可读渲染和 `presentation` 的双语文案。
- Tauri：`desktop/src/api/contract.ts` 用 `exact(...)` 做封闭解码，任何新增字段都会被拒绝。同一次改动里更新解码器、生成的 IPC 类型和 `fixtures/contract-variants.json`。
- TUI：进程内使用类型，编译期即可发现。
- 工具脚本：`tools/status-sink-smoke.py`、`tools/summarize-native-status.py` 读 JSON，改动后各跑一次。

## GPUI

- 采样生命周期：`Session::status_set_main_visible`（`shell.rs:2283`）在可见时调用 `status_start_live`，不可见时停止。`main.rs:62` 把托盘恒标为可见，本任务把它改为假；托盘面板出现后由 tray-panel 负责置位。更新 `status_tray_shares_one_subscription`。
- 趋势缓冲：`StatusPanel` 增加容量 60 的环形缓冲，纯数据结构，带单元测试。暂停时不追加。
- 绘制：趋势线优先用 GPUI 的画布元素和路径；实施第一步确认固定版本的可用接口。不可用时退为 60 个等宽条形。两种方式都不请求动画帧。
- 布局：四列两行的指标格，窄窗口两列。首格是健康证据和机器身份标签。数字用 `TYPE_DISPLAY`，结果类读数用 output 色。
- 进程表沿用 `status_panel.rs` 的排序和解释逻辑，只换行组件和列。

## 兼容与回滚

- 旧的快照 JSON 仍可被新代码读取（新字段缺省）。
- 回滚 GPUI 部分不影响 core 与 CLI 的新增字段。

## 风险

- Tauri 封闭解码器漏改会让 Tauri 的 Status 页整体失败。验收 AC3 专门覆盖。
- 自动采样改变了空闲开销。AC7 记录前后对比。
- 电池与频率接口在台式机或虚拟机上经常不可用，界面必须把"不可用"当常态处理。

## 已定事项（用户 2026-11-13 批准，依据 `research/collectors.md` 第 6 节）

1. 字段级缺失原因：扩展封闭的 `UnsupportedCode`，新增 `cpu_frequency`、`battery_health`、`battery_cycle_count`、`battery_rate`、`gpu_name`，字段为 `null` 时列出。不把父块标为 partial。
2. `physical_disk_activity` 改为条件列出：`disk_activity` 有值时不列，与 gpu/thermal 一致。`host`、`disk_activity` 不参与 envelope `outcome` 判定。
3. 新增 windows-sys 特性 `Wdk_Graphics_Direct3D` 与 `Win32_Devices_DeviceAndDriverInstallation`（注册表读取沿用现有本地声明或已启用特性）；同步更新规格中"只新增 `Win32_System_Performance`"的记录。
4. `NetworkKind` 不加 `virtual`：`Type` 为以太网但 `InterfaceAndOperStatusFlags.HardwareInterface` 为假的接口归为 `other`。
5. 电池：`full_charge_basis_points` 超过 10000 照实输出，界面条形封顶；`cycle_count` 为 0 视为未报告（`null` 并列 `battery_cycle_count`）；充放电功率沿用 30 秒电源节奏。
6. `os_version` 为实测版本加 DisplayVersion，例如 `10.0.26300.9974 (26H2)`；不按构建号推断产品名，不读 `ProductName`。
