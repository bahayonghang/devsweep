# 步骤 10 · 当前用户 MSIX 卸载

2026-11-20 用户回复“同意实施步骤 10”。工作树为 `D:/ds-integrate-1120`，基于本地 `264e7ac` 与远端 `324e72b` 的待提交集成结果。

软件清单中的当前用户 MSIX 条目可进入共用主机动作审阅。后台从 Windows 重新取得当前用户 MSIX 清单，调用 core 的 `build_selection_plan` 与 `preview_selection_plan_live`。审阅展示名称、完整包身份、不可撤销的影响及摘要。只可使用明确标有“确认卸载”的控件提交这一份计划。控件绑定审阅代次，重复点击、根节点 Enter 和行上再次审阅均不直接派发卸载。单项执行从派发到结束使用共用工作队列的取消标志及独占状态。

桌面与 CLI 调用相同的 `SoftwareExecutor::execute`。core 在持有进程和审计锁时取得实际清单、验证资格与摘要、持久化派发开始、调用 MSIX 适配器并重查精确包身份。界面保留操作 ID、结果、安装状态、重启证据及错误码，按语言显示安装状态和已知错误原因。派发后的审计错误显示结果未知。界面展示夹具不能取得主机卸载权限。MSI、残留执行及 ARP 跳转仍属于后续步骤。

| 检查 | 本阶段证据 |
| --- | --- |
| 格式与差异 | `cargo fmt --manifest-path desktop-gpui/Cargo.toml --all -- --check`、`git diff --check` 通过。 |
| 新增桌面夹具 | `desktop-gpui/src/host_action_tests/msix.rs` 通过 Software 行激活进入审阅，覆盖中英文结果、明确确认、重复激活、过期身份、资格和摘要、过期投递、导航、刷新、取消、批量与文件夹具隔离。测试中注入内存执行器。 |
| 新增 core 夹具 | `software::execution::tests::msix_durable_dispatch_and_terminal_write_failures_preserve_recovery_evidence` 使用隔离临时审计文件及适配器，断言派发开始写入失败零调用、终态写入失败后恢复只重查且不重复卸载。 |
| 原生及实机 | 没有对宿主安装的软件执行卸载。夹具结果和文本投影不能证明原生键盘与辅助功能、Windows 部署或重启结果。 |

正式门禁与独立检查结果由主会话在集成工作树核对后记录。
