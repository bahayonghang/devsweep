# Clean workbench

Clean follows a frozen authority chain:

scan observation → completed report → exact selection → saved untrusted plan → live preview → digest → confirm → execution → Clean V1 audit.

```powershell
devsweep clean scan --root . --scope all
devsweep clean plan --observation report.json --select TARGET_ID --output plan.json
devsweep clean preview --plan plan.json
devsweep clean execute --plan plan.json --preview-digest sha256:DIGEST --confirm
```

## Safety

- Scan results are not selectable until a completed report exists.
- Preview is always a dry run. It never runs a command or moves trash.
- Execute requires the saved plan, the live `sha256:` digest from that preview, and `--confirm`.
- Permanent delete is disabled. Docker is out of scope. Cargo home is inspect-only.
- Command cleanup keeps program and argv separate. There is no `--audit-log` flag.

## Capacity

Size labels show verified, partial lower-bound, or unknown evidence. Incomplete totals are never shown as exact. A trash success says the target was moved to trash; capacity becomes available after trash is emptied.

## Audit

Execution appends redacted Clean V1 records only to `%LOCALAPPDATA%\DevSweep\audit\v1\clean.jsonl`. Records are `execution_transition` or `protection_mutation`. They do not contain argv, command strings, raw paths, plan payloads, or localized text. Unknown versions and corrupt bytes are preserved and fail closed. The removed `--audit-log` path and `%APPDATA%\devsweep\audit.jsonl` are never discovered or rewritten.
