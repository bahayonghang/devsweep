# R7 独立复核

在 Windows 集成工作树 `D:/ds-integrate-1120` 复核，输入版本为 `264e7ac` 与 `324e72b`，包含 2026-11-20 的 R7 实施及本次修正。

| 验收 | 独立核验 |
| --- | --- |
| AC1、AC2 排名 | core 先验证快照，再使用有界堆保留最多 K 个文件。排序键为逻辑字节降序、路径升序、节点 ID 升序。验收覆盖并列、阈值、上限、空结果、快照保留范围和局部扫描。未知大小排除，实测零参与，已观测下界保留。独立运行 core 边界用例与中英文 CLI 契约，各 1 项通过。 |
| AC3 卷容量 | 检查 GetDiskFreeSpaceExW 前两个输出的绑定与失败处理。独立复验同卷别名、已知可用零、拒绝及无容量字段的旧 V1 数据。另见下方零配额修正。 |
| AC4 布局与选中 | 复算[原图索引](../evidence/native/screenshots.json)全部哈希，核对[宽窄布局与操作记录](../evidence/native/acceptance.md)。宽屏行的 x=130、宽=531，对应树块 x=723；窄屏两者 x=110/111，树块在行下方。行和树块通过同一节点 ID 与勾选集合绘制。 |
| AC5 三项预览 | 独立运行三节点保存计划、改选后撤销摘要与无摘要拒绝的桌面测试。真实窗口存档另显示 3 项 On 和 1 个确认控件，改选后为 2 项 On、0 个确认控件。CLI 独立预览有 3 项并返回一个摘要。 |
| AC6 菜单 | 独立运行生产 GPUI 快捷键解析回归，验证 Menu、Shift+F10 与 Esc 的上下文优先级。非输入桌面的实际窗口记录覆盖右击、Esc 和两种键盘入口；UIA 的前台焦点属性在该桌面恒为 false，行为证据为未再点击原行仍可用键盘重开原菜单。 |
| AC7 集成门禁 | 完整 `just ci` 与 `just gpui` 由父任务在最终工作树统一登记。 |

## 容量呈现修正

- GPUI 的榜单、目录列表、树块和详情此前直接格式化节点字节，可能把下界显示成精确值，或把未知容量显示成 0。现在同一双语格式器保留 Complete、Incomplete、Unknown 三种证据。最大文件榜单的辅助名称也采用与可见行相同的完整路径，同名文件可区分。
- Windows 成功返回配额总量 0、可用量 0 时，此前的额外非零条件会把合法零值显示成读取失败。改为区分 API 成功且数据自洽、API 失败或可用量超过总量。新核心回归独立通过，见本地 `dist/verify-1120/r7-caller-quota-review.log`。
- 新桌面回归 `observed_sizes_distinguish_zero_unknown_and_lower_bound_in_both_languages` 覆盖中英文零、下界和未知，已在 R8 共享的普通 GPUI 测试中通过，见本地 `dist/verify-1120/r8-desktop-fixed.log`。这里的原生图只覆盖已知夹具容量。

完整桌面门禁已复验 observed_sizes_distinguish_zero_unknown_and_lower_bound_in_both_languages 与菜单绑定，二者均通过。归档门禁见 ../result.md。
