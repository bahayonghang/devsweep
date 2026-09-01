# devsweep-inspect

> 让 Agent 用 DevSweep CLI 检查本机开发缓存和磁盘占用，并给出带证据的清理/优化建议，默认不执行清理。

## 为什么值得用

Agent 在「磁盘满了」「开发缓存太大」这类请求上容易改用 `du` 或 `Remove-Item`。本 skill 把检查绑到 DevSweep 的 scan → 未信任 Cleanup Plan → preview digest 链路上，建议里使用仓库领域词，并在执行边界停下。

## 安装

仓库内直接使用：

```text
skills/devsweep-inspect/SKILL.md
```

若以后作为独立 skill 发布：

```bash
npx skills add bahayonghang/devsweep --skill devsweep-inspect
```

验证：

```powershell
python C:\Users\lyh\.grok\skills\qiaomu-meta\scripts\validate_skill.py skills/devsweep-inspect
```

## 你可以直接这样说

- 「检查本机，给出磁盘清理建议」
- 「看看开发缓存占了多少空间」
- 「recommend cleanup for this machine, inspect disk space first」
- 「给出优化建议，先不要执行」

## 它会做什么

1. 定位 `devsweep` 或仓库内 `cargo run --locked -p devsweep-cli --bin devsweep --`
2. 按请求运行 `clean scan`、`analyze scan`、`optimize list` 或 `status snapshot`
3. 输出 Cleanup Target 建议表，区分 verified / partial-lower-bound / unknown
4. 在执行前停止，并说明后续需要 preview digest 与 `--confirm`

## 前置条件

- [ ] 能运行 DevSweep：`cargo run --locked -p devsweep-cli --bin devsweep -- --version`
- [ ] Python 3：用于 `validate_skill.py`
- [ ] 本仓库 `CONTEXT.md` 作为领域词真源

## 输出示例

```text
Inspect: clean scan --root . --scope all --format json --output inspect.json
Estimated Recoverable: verified 12.4 GiB; partial-lower-bound 3.1 GiB; unknown 2 rows
Cleanup Target table: id / path / evidence / risk / class / advice
Next step: this skill does not execute. Later: clean preview, then clean execute --confirm.
```

## 配置

无环境变量。JSON 输出文件由 Agent 选择尚未存在的路径，经 `--output` 新建。

## 风险

- 本 skill 会运行只读/观察类 CLI，并可能新建 JSON 文件。
- 它不执行清理、不卸载软件、不修改保护列表。
- 不要把建议表当成已释放容量。

## Troubleshooting

| 问题 | 原因 | 解决 |
|---|---|---|
| `npx skills add` 找不到包 | 尚未发布独立 skill 仓库 | 在本仓库读取 `skills/devsweep-inspect/SKILL.md` |
| `validate_skill.py` 失败 | 缺 evidence 报告或嵌套 `SKILL.md` | 按脚本 failures 补 `reports/`，保持单入口 |
| CLI 退出码 2 | 参数或 TTY 冲突 | 对照 `docs/reference/cli.md`，JSON 命令不要加 `--language` |
| 想执行清理 | 本 skill 故意停在建议 | 用户明确要求后再走 `clean preview` 与 `clean execute --confirm`，不要用本 skill 执行 |

## 致谢

方法上参考了 avdlee/xcode-disk-cleanup-agent-skill; orzcls/win-disk-cleaner; az9713/claude-skill-disk-cleanup; heyzgj/storage-cleanup-skill; thearmagan/skills@dry-run-first 的审计优先与预览习惯。DevSweep CLI 与安全模型以本仓库为准。不复制这些仓库的清理脚本。

## License

MIT。见仓库根目录 `LICENSE`。
