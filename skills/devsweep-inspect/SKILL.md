---
name: devsweep-inspect
description: |
  Inspect this machine with the globally installed DevSweep CLI and return
  cleanup or optimization advice for developer caches and disk space. Use when
  the user asks to 检查本机, 磁盘清理建议, 释放空间, 开发缓存, or 优化建议; or
  says inspect disk space, developer cache, recommend cleanup, or optimization
  advice. Also use for 帮我清理, 执行清理, 确认后清理, or clean selected
  Cleanup Targets. Show the cleanup list and wait for confirmation before
  clean execute. Call PATH `devsweep.exe`, never cargo run of this repository.
  Do not use for software uninstall, TUI work, just ci, rm -rf, or PowerShell
  Remove-Item.
---

# DevSweep inspect and confirmed clean

Receive an inspect, advice, or cleanup request. Call the globally installed
DevSweep CLI. Return a recommendation report. Show a cleanup list and wait
for confirmation before `clean execute`.

## When to use

- Inspect this machine, disk space, or developer caches
- 检查本机, 磁盘清理建议, 释放空间, 开发缓存, 优化建议
- 帮我清理, 执行清理, 确认后清理, clean selected Cleanup Targets

## When not to use

- `optimize run` or `software uninstall`
- TUI or desktop implementation
- `just ci`, commits, issue triage
- Ad-hoc `rm -rf` or `Remove-Item`
- `cargo run` of this repository as the cleaner

Read [references/safety-contract.md](references/safety-contract.md) before any
command.

## Compact Workflow

1. Resolve the program with `python scripts/resolve_devsweep.py --repo-root <repo>`. Use that absolute PATH binary. Never `cargo run -p devsweep-cli` and never `target\\debug\\devsweep.exe` or `target\\release\\devsweep.exe` from this checkout.
2. Route inspect with [references/cli-playbook.md](references/cli-playbook.md). Default to `clean scan --scope all --format json --output <new-file>` plus explicit `--root` values. Do not default `--root .` when cwd is this repository.
3. Keep program and argv separate. Do not compose a shell cleanup string.
4. Read the JSON result. Build the recommendation report from [references/recommendation-report.md](references/recommendation-report.md). Use Cleanup Target, Scan Report, Cleanup Plan, Estimated Recoverable, and Inspect Only.
5. If the user asked only for advice, stop after the report. Omit execute argv.
6. If the user asked to clean, follow [references/confirmed-clean.md](references/confirmed-clean.md): list selectable ids, display the table, wait for confirmation, then `clean plan` / `clean preview` / `clean execute`.

## Output Contract

1. Name the inspect command, the global binary, and the roots that actually ran.
2. Split Estimated Recoverable into verified, partial-lower-bound, and unknown.
3. Table Cleanup Targets with id, path, evidence, risk, class, and advice `recommend` / `inspect-only` / `exclude`.
4. Call out Inspect Only rows, including Cargo home.
5. Treat Optimize list rows as catalogue facts, not completed maintenance.
6. For inspect-only requests, end without execute argv.
7. For cleanup requests, display the full selectable list and wait. Do not run `clean execute` in the same turn as the first list.
