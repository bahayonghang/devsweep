# 当前用户卸载、残留与手动管理入口

2026-11-20。用户批准步骤 10 后，独立检查修复了重复预览投递和 MSI 只读基线的两项问题。七项 MSIX 回归及一项 core 持久审计恢复回归通过。按后续授权接入步骤 11–15，验证均使用隔离执行器与任务夹具。

## 路径与验收

| 验收 | 实现与证据 |
| --- | --- |
| AC1、AC2、AC8 | MSIX 与当前用户非托管 MSI 先绑定一个选择项。MSI 保留该用户的登记/软件包/工具证据。准备与执行分别经过 core 实时清单、预览摘要和单次执行器。覆盖按键与重复行激活、重复确认、用户 SID/工具变动、登记缺失、资格拒绝，以及退出 0、1603、3010、1641 的结果呈现。core 固定参数及拒绝分派详见 [MSI 检查](msi-check.md)。 |
| AC3、AC4、AC9 | 实际 ARP 条目只提供固定目标 `ms-settings:appsfeatures` 的管理入口和既有只读目录查看。管理员提示可复制 core ID 绑定的 CLI 计划命令。测试断言 ARP 无卸载意图、设置 URI、MSI 管理员拒绝、复制内容不含显示名称。CLI 已有计划/预览/确认门槛，见[等价 CLI 表](../design.md)。检索 core 软件目录生产代码未发现读取或执行厂商卸载字符串；仅审计测试列出这两个禁用的字段名。 |
| AC5 | 只有具备成功软件报告的结果才显示“审阅卸载残留”。MSI 另核对退出 0、同一用户 SID、无中断和登记消失。单个候选目录的严格预览与执行分别核对持久化卸载操作、软件身份、目录身份和摘要，然后才能移入回收站。目录列表本身无法确认。测试覆盖成功、失败、无成功报告、另一用户 SID/软件身份、重启、未确认和过期摘要。预卸载“相关目录”继续只读。 |
| R7 注册表范围 | 现有 CLI 支持名为 `DevSweepResidue` 的文件夹具，先备份并核对原字节，更改后仅在当前字节和备份吻合时恢复。桌面目录选择页说明了此边界。core 回归 `ac3_registry_residue_backup_and_conditional_restore` 通过。本次没有增加 HKCU/HKLM 注册表扫描或写入。 |
| AC6、AC7 | [进程动作检验](process-step9-check.md) 与[保持唤醒检验](keep-awake-check.md)记录各自身份、审计和 owner/端点语义。此次桌面回归包含 fake probe/killer、保持唤醒后端和启动项夹具。 |
| AC10 截图 | 七项可执行动作的双语审阅及完成状态共 28 张原生 PNG。见[证据索引](native-host-reviews/receipts.json)。最终集成的 `just ci` 与 `just gpui` 由父任务串行核对。 |

## 原生证据

文件名为 `动作-状态-语言.png`。每项覆盖 review、result × en、zh。动作名为 awake、startup、update、process、msix、msi、leftovers。组件使用实际 `ShellView::host_review_bar`，状态来自隔离执行器或保持唤醒测试后端。卸载和文件移动为模拟分派。进程为 PID 42 的内存夹具。启动项只读写本次临时的 hive 文件。

启动 Windows 专用忽略测试 `shell::host_action_tests::native_receipt::native_host_review_receipt`，输入形如 `DEVSWEEP_NATIVE_HOST_RECEIPT=msi:review:en:light`。测试把完成的会话状态交给不带执行器的窗口，并在启动渲染前释放原夹具。每个测试进程位于单独的非输入 Win32 desktop。截图通过 `PrintWindow(..., 2)` 在自身 HWND 上生成。尺寸为 860 × 340 逻辑像素，120 DPI，包含标题栏的原图为 1093 × 472。所有窗口均正常退出。已逐项查看截图，审阅包含摘要和单独的确认控件，结果包含动作结论。MSI 显示产品代码、退出码和重查结果；残留移动显示完整路径。

Win32 UIA 在这个独立测试窗口上读到 AccessKit 根且没有返回子控件，本组截图不提供键盘或屏幕阅读器验收。固定 Windows 设置入口及复制命令没有审阅/结果画面，依靠上述契约测试核对，未打开用户设置或改写其剪贴板。没有在用户主机卸载、更新、移动目录或操作用户进程，也未测试第三方 Installer、UAC 或重启。

## 集成工作树运行

- 桌面主机动作过滤测试 25 通过，另有一项原生收据测试按设计默认忽略，随后显式执行。
- `cargo test --manifest-path desktop-gpui/Cargo.toml --locked --all-targets` 最终结果为桌面应用 257 通过、1 项忽略，示例 4 通过。第一次执行发现旧文案断言失败，修正通用清理通知与残留入口后全套通过。`cargo fmt` 及本次代码的差异空白检查通过。
- 注册表备份/条件恢复、MSI 资格拒绝及运行绑定的 core 夹具在集成工作树测试日志中通过。步骤 10 复核另见[独立检查](msix-step10-check.md)。最终归档的门禁以集成记录为准。
