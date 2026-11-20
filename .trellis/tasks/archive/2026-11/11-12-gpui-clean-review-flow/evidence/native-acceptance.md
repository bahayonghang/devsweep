# Clean 原生验收

原生验收尚未完成。2026-11-15 用户选择“先完成代码与门禁，稍后再做原生验收”，界面操作已停止。下列记录区分实际观察与待验收项目，不能据此声明整个任务完成。

同日后续指令要求继续其他 Trellis 任务，并禁止使用 Computer Use 验收。本记录保留已有证据，未完成项目保持未验证；不再调用 Computer Use 补验，也不因此阻塞可独立推进的后续任务。

## 已确认的验收边界

- 完成页与会话记录使用完整的本次执行报告；持久化历史仅表示实际记录的证据。正常落盘时核对同源同范围结果；取消、未启动、审计失败时说明证据可能不完整。
- Command 与 InspectOnly 目标仅供审阅，不能经回收站选择、预览或确认路径执行。core/CLI 不变。
- 真正的移入回收站仅允许任务拥有的夹具；不清理宿主真实缓存，不执行命令型清理，不扫描宿主全局目录来取得验收数据。

## 原生验收矩阵（部分完成）

| 项目 | English | 简体中文 | 证据与限制 |
| --- | --- | --- | --- |
| 未扫描：首行无容量或 0 | 已观察 | 已观察 | [English](en-unscanned.jpg)、[中文](zh-unscanned.jpg)，正常隔离会话 |
| 扫描中：进度与已发现的下界 | 待补截图 | 已观察 | [中文真实扫描](zh-scanning.jpg)，未知容量保留说明 |
| 已扫描：找到量、建议量、审阅动作 | 已观察 | 已观察 | [English](en-scanned.jpg)、[中文](zh-scanned.jpg)：各 12 KiB、三个目标，近期文件建议量为 0 |
| 审阅：分组小计、三态、仅供审阅 | 已观察，最终版待复核 | 已观察，最终版待复核 | [真实英文](en-review-real.jpg)、[真实中文](zh-review.jpg)；[英文仅审阅夹具](en-review.jpg)明确禁用执行 |
| 已预览：固定栏、有效摘要、确认动作 | 已观察，未确认执行 | 待执行 | [英文有效预览](en-previewed.jpg)，保存快照绑定三项共 12 KiB |
| 完成：成功移入量、失败与跳过原因、分组 | 待执行 | 待执行 | — |
| 键盘完整流程、既有行与翻页操作 | 已走到预览；执行与翻页待补 | 组头 Space、Tab 到行、行 Space 已观察 | 英文：根焦点 Enter 扫描、Enter 审阅，Space 与 Alt+Down 逐行选择，Enter 预览；没有确认执行 |
| UIA Mixed 状态 | Checkbox 名称报告 Partially selected 1/2 | Checkbox 名称报告 部分选中 1/2 | [英文 UIA](en-review-real-uia.txt)、[中文 UIA](zh-review-uia.txt)。Sky 不输出 ToggleState 数值；不能声称直接读到了 Indeterminate，真实 Toggled/Mixed API 与固定适配器映射已有源码检查 |
| 无摘要与改选后的确认拒绝 | 已观察 | 无摘要禁用已观察，改选失效待补 | [英文改选](en-digest-invalidated.jpg)：即时移除摘要、确认禁用；恢复第三项仍禁用，重新预览才启用 |
| All 请求范围与实际范围、全局确认 | 模型测试，原生待补 | 未确认只扫项目已观察 | [确认前](zh-all-unconfirmed.jpg)、[扫描结果](zh-scanned.jpg)首行明确仅扫描项目；确认后双旗标由隔离测试覆盖，不扫描宿主全局 |
| 正常记录的结果与隔离持久化历史核对 | 隔离模型测试已覆盖；原生待补 | 待执行 | 不以累计历史冒充单次结果；真实夹具没有执行 |
| 取消与审计异常的证据限定 | 模型测试已覆盖；原生场景待补 | 同左 | 混合结果需用明确标识的只读夹具观察 |

## 门禁

- `$env:RUST_TEST_THREADS='4'; just ci`：最终完整通过，格式、编译检查、测试（863 passed、0 failed、3 ignored）、Clippy 均通过；先前的 stderr 截断失败本轮未复现，provider_authority 亦通过，无需隔离重跑。原始日志 `C:\Users\lyh\AppData\Local\rtk\tee\1794793162_just_ci.log`。初次失败与定向核实仍保留在 [root-gate.md](../research/root-gate.md)，未修改无关测试。
- `$env:RUST_TEST_THREADS='1'; just gpui`：完整通过，格式、Clippy（all targets、`-D warnings`）、217 项测试（0 failed，10.68s）、release build 均通过。原始日志 `C:\Users\lyh\AppData\Local\rtk\tee\1794793061_just_gpui.log`。
- `python -m unittest discover -s tools/tests`：已运行 23 个测试，退出码 1；仅有 `test_gitignore` 的两个已知旧失败：`target/debug/devsweep.exe` 位于符号链接之后（Git 128），以及 `.trellis/hooks.local.json` 缺少忽略规则（Git 1）。未观察到新增失败；不能记为整套通过。

## 环境与来源

夹具根目录为 `C:\Users\lyh\AppData\Local\Temp\devsweep-gpui-clean-review-20261115-c145696e`。`en/` 与 `zh/` 各含一个 `pyproject.toml` 和三个目录（`.pytest_cache`、`.mypy_cache`、`.ruff_cache`），每个缓存只有一个 4096 字节的零填充 `fixture.bin`。正常会话分别使用 `en-profile/`、`zh-profile/` 下的 `localappdata/` 与 `appdata/`，配置为 light、100%、reduced motion、显式单一项目根。两份项目已扫描，均未执行，原始夹具文件仍应保留。

测试二进制是 debug 构建的任务临时副本，不进入 Git：英文真实流程及初次只读分组使用 `native-v2/devsweep-gpui-clean-review.exe`（SHA256 `1B760E68B04F9AD32CB4DCD578FA7BCB803A0E14E136845C7DEC0516DA8564B6`）；中文真实流程使用 `native-v3/` 同名程序（SHA256 `7B7875179A47F640B226392225936E74EA1C2D2E704C622EC2F381DD2DE3980A`，补充扫描初始未知下界）。检查代理后续修复的最终构建仍须原生复核，旧截图不能自动替代。

检查修复后的 debug 二进制为 `desktop-gpui/target/debug/devsweep-gpui-spike.exe`，SHA256 `9D731E3791C24763134615EB7AB8DC4B230C2281A3ADB98B661D36B4F0ED8981`。新增只读夹具导航隔离、非法环境变量拒绝、地图置信度保留与中文“确认移入回收站”文案；恢复原生时以该版本或之后经过检查的版本复核。

暂停界面操作时，英文已生成预览 `sha256:0d72f235a259e6dd014dd54db8b173aa78f5dbb7fbd13e7eb48d084e33f6dd65`（三项共 12 KiB），中文只选中 `.mypy_cache`。之后只读核对时，这两个可见测试进程已退出，六个 `fixture.bin` 仍各为 4096 字节，未移动文件。恢复时需要重新启动、扫描与预览，不能把历史摘要当作当前授权。

初次错误使用 Hidden 启动的只读测试实例（PID 38076，任务拥有的 `native-bin/devsweep-gpui-clean-review.exe`）仍存活且无可见窗口，未参与通过证据；没有通过强制结束进程来清理它。恢复原生阶段时一并处理这个测试实例。

工具曾报告其他输入，并返回非目标前台截图；这些截图未保存为验收证据。已保存的截图逐张观察为 DevSweep。尝试已有 Windows SDK Inspect 时遇到 UIAccess 读取限制，没有得到可引用的 ToggleState 数值。模型测试、只读原生夹具、原生 UIA 名称与真实回收站执行分别记录，不互相替代。

## 2026-11-20 集成工作树的原生复验

本节替代上方 11 月 15 日矩阵的待验状态，保留首次尝试的限制与旧截图。集成输入为本地 264e7ac 和远端 324e72b。Windows 10.0.26300，125% DPI，窗口客户区为 1240×820 逻辑像素。测试窗口运行在从未切换到前台的独立 Win32 桌面。PrintWindow 只抓取该 PID 的 DevSweep 窗口，键盘消息也只发往该窗口。树读取使用 Windows UI Automation 的 TogglePattern。

| 验收项 | 本次检查 |
| --- | --- |
| AC1 六态 | 下表两组原生图分别覆盖未扫描、扫描中、已扫描、审阅、已预览、完成。扫描中使用程序自带只读夹具以稳定捕获瞬间，其余状态来自完整可执行的隔离项目流程。 |
| AC2 全部范围 | 未授权时，旧中文原生图及模型断言证明实际只扫描项目。确认后桌面测试 `clean_all_scope_requires_explicit_roots_and_fresh_global_consent` 核对双旗标，core 的 `full_scan_merges_and_ranks_once`、`scan_options_skip_phases` 和组合报告测试验证同一轮合并两类夹具、单范围仅访问对应来源。按约定，全局范围验证使用隔离数据。 |
| AC3 分组 | 每个项目夹具 4,096 B，53 个目标的小计为 212.0 KiB。原生 UIA 实际读取一项选中时的 Indeterminate，独立组夹具读取 On、Off，三种值和辅助名称存于 receipts.json。 |
| AC4 仅供审阅 | 分组测试检查 Command 和 InspectOnly，单项、整组及推荐选择均排除这两类目标。篡改计划后预览和后台执行都拒绝。原生只读夹具的“仅供审阅”组保留独立复选框语义。 |
| AC5 预览有效期 | 中英文分别预览一项、取消选择、检查确认控件禁用、重新选中并重新预览后才确认。正常入口销毁旧摘要，后台仍独立核对。 |
| AC6 完成与历史 | 中英文都只移动任务目录中的一个 4,096 B 缓存到回收站。每次从不同的隔离配置启动，53 个文件剩 52 个。用相同配置和操作 ID 运行正常 CLI 的 `history show`，两条事件分别为 Started、Succeeded，有相同的目标身份，估算容量 4,096 B。修正文案后的复验见下方两份最终历史。审计异常与取消的图为明确标识的只读混合结果夹具。 |
| AC7 全键盘及分页 | 两种语言均从根焦点使用 Enter 扫描和打开审阅，PageDown 将 48 行窗口从 item-01…48 移至 item-06…53，PageUp 返回，再用 Space 选中、Enter 预览、Enter 确认到完成。页面变更前后保存 UIA 可见行 ID，确认证据包括当前已启用的动作控件和所有可见行均属于指定夹具。 |
| AC8 项目门禁 | 合并后的 `just ci` 通过。步骤 10 初轮完整 `just gpui` 通过 248 个产品测试、4 个画廊测试及 release 构建。复验文案与实际完成页对照，后续父任务再次运行两个最终集成门禁。 |

| 状态 | 英文 | 中文 |
| --- | --- | --- |
| 未扫描 | [图](integration-1120/en-landing.png) | [图](integration-1120/zh-landing.png) |
| 扫描中，明确标识为只读夹具 | [图](integration-1120/en-scanning.png) | [图](integration-1120/zh-scanning.png) |
| 已扫描 | [图](integration-1120/en-scanned.png) | [图](integration-1120/zh-scanned.png) |
| 审阅 | [浅色](integration-1120/en-review.png) · [深色](integration-1120/en-dark-review.png) | [深色](integration-1120/zh-review.png) · [浅色](integration-1120/zh-light-review.png) |
| 已预览 | [图](integration-1120/en-previewed.png) | [图](integration-1120/zh-previewed.png) |
| 最终完成页 | [图](integration-1120/en-final-complete.png) · [同次历史](integration-1120/en-final-history.json) | [图](integration-1120/zh-final-complete.png) · [同次历史](integration-1120/zh-final-history.json) |

[UIA、翻页、改选和首次完整执行记录](integration-1120/receipts.json) · [审计异常夹具](integration-1120/en-audit-failed.png) · [取消夹具](integration-1120/zh-canceled.png)

首轮原生 release SHA-256 为 29EAF68B7C22872BEA7A421ED9C60551C235D86047CC4D6F1D25CEF921473BCB。原生检查发现普通完成页把执行写成 isolated，英文单项显示“1 targets”。修正后重新构建并用新建夹具从扫描走到完成，release SHA-256 为 B97D879591F51E3A992D0B5B27463A6C496726D9496B5AE384F38994C7406735，最终图中为“1 target”与准确的已记录提示。英文操作 ID 为 op-1795164649730，中文为 op-1795164659613。所有窗口均正常关闭。

整合 Host Actions 的全套桌面用例时，核实同一条简短通知也用于测试执行器。最终通用通知为“The move was recorded.”和“已记录移动结果。”，具体回收站目的地、成功状态与数量见同页结果行。普通 debug 构建 D325B433025E7912DA03482457369BCEE3881E55700D21F9649D4E9679CBF658 经同一夹具入口和键盘序列复核，两次均为 53 项剩 52 项，操作历史为 started → succeeded。[英文完成页](integration-1120/en-completion.png) · [英文历史](integration-1120/en-completion-history.json) · [中文完成页](integration-1120/zh-completion.png) · [中文历史](integration-1120/zh-completion-history.json)。
