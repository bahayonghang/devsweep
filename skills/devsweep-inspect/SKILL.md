---
name: devsweep-inspect
description: |
  Inspect this machine with the DevSweep CLI and return cleanup advice or
  optimization advice for developer caches and disk space. Use when the user
  asks to 检查本机, 磁盘清理建议, 释放空间, 开发缓存, or 优化建议; or says
  inspect disk space, developer cache, recommend cleanup, or optimization
  advice. Do not use for clean execute, software uninstall, TUI work, rm -rf,
  or PowerShell Remove-Item deletion.
---

# DevSweep inspect and advise

Receive an inspect or advice request. Run DevSweep inspect commands. Return a
recommendation report. Do not mutate the machine.

## When to use

- Inspect this machine, disk space, or developer caches
- 检查本机, 磁盘清理建议, 释放空间, 开发缓存, 优化建议
- Recommend cleanup or optimization advice without execution

## When not to use

- `clean execute`, `optimize run`, or `software uninstall`
- TUI or desktop implementation
- `just ci`, commits, issue triage
- Ad-hoc `rm -rf` or `Remove-Item`

Read [references/safety-contract.md](references/safety-contract.md) before any
command.

## Compact Workflow

1. Locate the DevSweep program. In this repo use `cargo run --locked -p devsweep-cli --bin devsweep --`. Prefer an installed `devsweep` when `devsweep --version` works.
2. Route the request with [references/cli-playbook.md](references/cli-playbook.md). Default to `clean scan --scope all --format json --output <new-file>`. Use `analyze scan` when the user wants tree usage without cleanup authority. Use `optimize list` for the Windows catalogue. Use `status snapshot` for a machine snapshot.
3. Keep program and argv separate. Do not compose a shell cleanup string.
4. Read the JSON result. Build the recommendation report from [references/recommendation-report.md](references/recommendation-report.md). Use Cleanup Target, Scan Report, Cleanup Plan, Estimated Recoverable, and Inspect Only.
5. Save `clean plan` only when the user asked for a plan file. The file stays an untrusted Cleanup Plan.
6. Stop. Do not run `clean execute`, `optimize run`, `software uninstall`, or protection mutations. Say that execution needs a later explicit request plus preview digest and `--confirm`.

## Output Contract

1. Name the inspect command and roots that actually ran.
2. Split Estimated Recoverable into verified, partial-lower-bound, and unknown.
3. Table Cleanup Targets with id, path, evidence, risk, class, and advice `recommend` / `inspect-only` / `exclude`.
4. Call out Inspect Only rows, including Cargo home.
5. Treat Optimize list rows as catalogue facts, not completed maintenance.
6. End with the execute-later next step. Omit execute argv from the default report.
