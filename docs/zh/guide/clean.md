# Clean 工作台

Clean 使用冻结的授权链：

保存观察结果 → 精确选择 → 保存**新**计划 → 实时预览摘要 → `--confirm` → 执行 → Clean V1 审计。

观察 JSON 不是可运行计划。`clean preview` 和 `clean execute` 需要这份新计划文件。

```powershell
devsweep clean scan --root . --scope all --format json --output observation.json
devsweep clean plan --observation observation.json --select TARGET_ID --output plan.json
devsweep clean preview --plan plan.json
```

从观察信封中的 `data.plan.targets[].id` 读取 `TARGET_ID`。仅在取得实时摘要后再
执行。使用前用 `--help` 核对契约：

```powershell
devsweep clean execute --help
devsweep clean execute --plan plan.json --preview-digest sha256:DIGEST --confirm
```

没有 `--execute` 参数。

## 安全

- 扫描完成前结果不可选。
- 预览始终是演练，不会运行命令或移入回收站。
- 执行必须同时提供已保存计划、该次预览的 `sha256:` 摘要和 `--confirm`。
- 永久删除保持关闭。Docker 不在范围内。Cargo home 仅为检查。
- 命令清理保持 program 与 argv 分离。没有 `--audit-log` 参数。

## 容量

容量标签区分已验证、部分下限和未知证据。不完整合计不会显示为精确值。回收站成功表示目标已移入回收站；清空回收站后容量才会可用。

## 保护与规则

保护列表和规则嵌套在 Clean 下：

```powershell
devsweep clean protect list
devsweep clean protect add --path PATH --confirm
devsweep clean protect remove --path PATH --confirm
devsweep clean rules list
```

## 审计

执行只向 `%LOCALAPPDATA%\DevSweep\audit\v1\clean.jsonl` 追加经过脱敏的 Clean V1 记录。`record_kind` 只有 `execution_transition` 和 `protection_mutation`。记录不含 argv、命令字符串、原始路径、计划载荷或本地化文本。未知版本和损坏字节会被保留并失败关闭。已移除的 `--audit-log` 路径和 `%APPDATA%\devsweep\audit.jsonl` 不会被发现或改写。
