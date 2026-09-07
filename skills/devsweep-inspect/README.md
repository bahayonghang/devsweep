# devsweep-inspect

> 让 Agent 用**全局安装的** DevSweep CLI 检查本机开发缓存，给出带证据的建议；清理前必须展示列表并等待确认。不要用当前仓库的 `cargo run` 当清理器。

## 为什么值得用

Agent 在「磁盘满了」「帮我清理」上容易改用 `du`、`Remove-Item`，或 `cargo run` 本仓库从而扫到并清理自己。本 skill 把检查绑到 DevSweep 的 scan → 未信任 Cleanup Plan → preview digest 链路，建议使用仓库领域词，清理必须先出表再等人确认。

## 安装

语义真源是仓库内的 `skills/devsweep-inspect/`。工具发现的是下面两份**仓库内**副本，不要为每个 harness 再写一份业务正文，也不要拷到用户全局 skill 目录：

| 发现根 | 适用（已核实的共享路径） |
|---|---|
| `.agents/skills/devsweep-inspect/` | Codex 写入/发现；Grok 兼容发现；Kimi 官方也列出该共享根 |
| `.claude/skills/devsweep-inspect/` | Claude Code |

这两份副本被 gitignore；源提交不会自动更新它们。在仓库根同步与检查：

```powershell
just install-skill
just check-skills
```

`just install-skill` 只幂等替换上述两个仓库内 skill 目录。`just check-skills` 只读比较相对文件集合和字节哈希：缺文件、陈旧额外文件或内容漂移都会失败，**不会**隐式修复副本。

静态输出契约（fixture，**不是**五套 harness 的运行时测试）：

```powershell
python -X utf8 skills/devsweep-inspect/scripts/output_eval.py skills/devsweep-inspect
```

验证方式（发现 ≠ 运行）：

- 副本与源一致：`just check-skills`（只读哈希；失败不修复）
- 静态输出契约：上面的 `output_eval.py`（4 个 fixture，不是五套 harness 运行时测试）
- Grok 只读 discovery：`grok inspect --json` 的 `skills[].source.path` 指向 `.agents/skills/devsweep-inspect/SKILL.md`（兼容发现，不证明 hook/agent 已执行）
- Claude Code / Codex / Kimi / OMP 新会话是否加载并遵循该副本：UNVERIFIED，归 harness-alignment


若以后作为独立 skill 发布：

```bash
npx skills add bahayonghang/devsweep --skill devsweep-inspect
```

前置：PATH 上已有 `devsweep.exe`（例如 `just install` 装到 `~\.cargo\bin`）。不要用本仓库 `target\` 里的二进制做清理。

## 你可以直接这样说


- 「检查本机，给出磁盘清理建议」
- 「看看开发缓存占了多少空间」
- 「recommend cleanup for this machine, inspect disk space first」
- 「给出优化建议，先不要执行」
- 「帮我清理风险为 low 和 medium 的路径」
- 「确认清理这些目标」

## 它会做什么

1. 用 `scripts/resolve_devsweep.py` 解析全局 `devsweep.exe`
2. 按请求运行 `clean scan`、`analyze scan`、`optimize list` 或 `status snapshot`
3. 输出 Cleanup Target 建议表
4. 若用户要清理：列出可选项，**停下来等确认**，确认后再 `clean plan` → `preview` → `execute`

## 前置条件

- [ ] 全局 `devsweep --version` 可用，且路径不在本仓库 `target\` 下
- [ ] Python 3：用于本 skill 的辅助脚本、`just check-skills` 和静态 `output_eval.py`
- [ ] 本仓库 `CONTEXT.md` 作为领域词真源

## 输出示例

```text
Inspect: C:\Users\<you>\.cargo\bin\devsweep.exe clean scan --root D:\path\to\projects --scope all --format json --output inspect.json
Estimated Recoverable: verified 12.4 GiB; partial-lower-bound 3.1 GiB; unknown 2 rows
Cleanup Target table: id / path / evidence / risk / class / advice
若要清理：先展示可选项并等待确认，不要在同一轮执行。
```

## 配置

无环境变量。JSON 输出文件由 Agent 选择尚未存在的路径，经 `--output` 新建。

## 风险

- 本 skill 会运行观察类 CLI，并可能新建 JSON。
- 确认后会调用 `clean execute`（默认进回收站或 `cargo clean`）。
- 不要把建议表或执行报告写成已释放容量。
- 不要用当前仓库二进制，避免把本仓库 `target/` 当成清理器副作用清掉。

## Troubleshooting

| 问题 | 原因 | 解决 |
|---|---|---|
| `resolve_devsweep.py` 退出 2 | PATH 没有全局安装，或解析到了本仓库 `target\` | `just install`，确认 `where.exe devsweep` |
| `validate_plan` 拒绝某个 id | 扫描结果与规则路径不一致 | 跳过该 id，不要手改 plan |
| Windows 命令行过长 | `--select` 太多 | 使用 `scripts/plan_selected.py` 分批 |
| 想卸载软件 / Optimize run | 本 skill 不做 | 不要从这里调用 |

## 致谢

方法上参考了 avdlee/xcode-disk-cleanup-agent-skill; orzcls/win-disk-cleaner; az9713/claude-skill-disk-cleanup; heyzgj/storage-cleanup-skill; thearmagan/skills@dry-run-first; affaan-m/ECC@config-gc; vinta/awesome-python@preview-verdicts 的审计、预览与「先展示再等人」习惯。DevSweep CLI 与安全模型以本仓库为准。不复制这些仓库的清理脚本。

## License


MIT。见仓库根目录 `LICENSE`。
