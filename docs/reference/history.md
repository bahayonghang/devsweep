# History

History is inspect-only. It never retries or replays an action.

## Stores

The reader opens only the three fixed V1 journals from the shared
application-data resolver:

- `%LOCALAPPDATA%\DevSweep\audit\v1\clean.jsonl`
- `%LOCALAPPDATA%\DevSweep\audit\v1\software.jsonl`
- `%LOCALAPPDATA%\DevSweep\audit\v1\optimize.jsonl`

It does not walk directories, probe legacy paths, open `--audit-log` files, or
read the legacy default `%APPDATA%\devsweep\audit.jsonl`. Those files stay
untouched.

## Redaction

Command, argv, program, and path fields are rejected at the DTO boundary.
Protection-mutation records contain a SHA-256 identity, never a raw path.
Unknown schema versions are shown as unsupported and are not executed.

CLI: `history list` and `history show --operation-id <ID>`.

## 历史（简体中文）

历史仅为检查，不能重试或重放。只读取三个固定的 V1 审计存储，不扫描遗留文件。输出会遮盖命令、参数和路径；保护变更记录不含原始路径。
