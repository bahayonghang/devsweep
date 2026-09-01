# Design — Production inspect-advise skill package

## Package layout

```text
skills/devsweep-inspect/
  SKILL.md
  README.md
  manifest.json
  agents/interface.yaml
  evals/trigger_cases.json
  evals/output_cases.json
  references/cli-playbook.md
  references/safety-contract.md
  references/recommendation-report.md
  reports/prior-art-research.md
  reports/skill-ir.json
  reports/trigger-eval.json
  reports/creation-handoff.md
```

Do not add `scripts/` unless a deterministic helper is required. CLI invocation
stays in the playbook. Do not add a nested `SKILL.md`.

## SKILL.md

Frontmatter `name`: `devsweep-inspect`.

`description` must mention:

- inspect / disk space / developer cache / cleanup advice / optimization advice
- Chinese: 检查本机、磁盘清理建议、释放空间、开发缓存、优化建议
- DevSweep CLI
- exclusions: execute, uninstall, TUI, `rm` / PowerShell delete

Body:

1. When to use / when not to use
2. Preflight binary
3. Route to inspect command
4. Build recommendation report
5. Stop before execute
6. Links to the three references

Keep the file under 14_000 bytes.

## CLI playbook

Map user intent to argv. Use exclusive create-new `--output`. Prefer JSON for
agent parsing. Record the exact command in the report.

| Intent | Command |
|---|---|
| Project and global cleanup candidates | `clean scan --root <PATH> --scope all --format json --output <FILE>` |
| Disk tree without cleanup authority | `analyze scan --root <PATH> --format json --output <FILE>` |
| Windows maintenance catalogue | `optimize list --format json` |
| Machine snapshot | `status snapshot --format json` |
| Save selected plan | `clean plan --observation <FILE> --select <ID>... --output <FILE>` |
| Preview saved plan | `clean preview --plan <FILE> --format json` |

From this repo the program is:

```text
cargo run --locked -p devsweep-cli --bin devsweep -- <ARGS>
```

If an installed `devsweep` exists and `--version` works, the agent may use it.

## Safety contract

- Cleanup request → inspect permission only.
- Never run execute/run/uninstall/protect-add/protect-remove.
- Never compose `cmd.exe` / `pwsh` cleanup strings.
- Cargo home, Docker, permanent delete stay non-recommendations for cleanup.
- Preview digest is not an execute grant.
- Settings launch from Optimize is not completed maintenance.

## Recommendation report

Required sections:

1. Inspect command and roots
2. Capacity summary with Estimated Recoverable classes
3. Cleanup Target table: id, path, evidence, risk, class, advice
4. Inspect Only / excluded rows
5. Optimize or Status facts when those commands ran
6. Next step: user must explicitly request execution later

Advice values: `recommend`, `inspect-only`, `exclude`. Default selection in
DevSweep ranking is not auto-approval.

## Evals

Trigger positives: 检查本机、清理建议、开发缓存、disk space, recommend cleanup.

Trigger negatives: `just ci`, GitHub issue triage, TUI layout, commit message.

Near neighbors that must not trigger this skill as an executor:

- "执行清理" / "run clean execute"
- "卸载这个 MSIX"
- "rm -rf node_modules"

Output eval: a fixture recommendation must omit execute argv and shell
deletion.

## Manifest

```text
name: devsweep-inspect
maturity_tier: production
owner: bahayonghang
status: active
```

Do not set Qiaomu copyright fields.

## Validation

Run qiaomu scripts against `skills/devsweep-inspect`. Record outputs in
`reports/`. Missing install-proof and human review stay labeled
`missing evidence`.
