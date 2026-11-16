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
